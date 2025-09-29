import { PublicKey, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { AuraLendClient } from '../client';
import BN from 'bn.js';

export interface DepositLiquidityParams {
  reserve: PublicKey;
  amount: BN;
  userTokenAccount: PublicKey;
  userCollateralAccount: PublicKey;
}

export interface RedeemCollateralParams {
  reserve: PublicKey;
  amount: BN;
  userTokenAccount: PublicKey;
  userCollateralAccount: PublicKey;
}

export class LendingInstructions {
  constructor(private client: AuraLendClient) {}

  async depositLiquidity(params: DepositLiquidityParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();
    const reserve = await this.client.getReserve(params.reserve);
    
    if (!reserve) {
      throw new Error('Reserve not found');
    }

    const liquiditySupplyPda = this.client.getLiquiditySupplyAddress(reserve.data.liquidityMint);
    const collateralMintPda = this.client.getCollateralMintAddress(reserve.data.liquidityMint);
    
    const [liquiditySupplyAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), reserve.data.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    const [collateralMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), reserve.data.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    return this.client.program.methods
      .depositReserveLiquidity(params.amount)
      .accounts({
        market: marketPda,
        reserve: params.reserve,
        destinationLiquidity: liquiditySupplyPda,
        liquiditySupplyAuthority,
        collateralMint: collateralMintPda,
        collateralMintAuthority,
        sourceLiquidity: params.userTokenAccount,
        destinationCollateral: params.userCollateralAccount,
        userTransferAuthority: this.client.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
  }

  async redeemCollateral(params: RedeemCollateralParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();
    const reserve = await this.client.getReserve(params.reserve);
    
    if (!reserve) {
      throw new Error('Reserve not found');
    }

    const liquiditySupplyPda = this.client.getLiquiditySupplyAddress(reserve.data.liquidityMint);
    const collateralMintPda = this.client.getCollateralMintAddress(reserve.data.liquidityMint);
    
    const [liquiditySupplyAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), reserve.data.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    const [collateralMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), reserve.data.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    return this.client.program.methods
      .redeemReserveCollateral(params.amount)
      .accounts({
        market: marketPda,
        reserve: params.reserve,
        sourceLiquidity: liquiditySupplyPda,
        liquiditySupplyAuthority,
        collateralMint: collateralMintPda,
        collateralMintAuthority,
        destinationCollateral: params.userTokenAccount,
        sourceCollateral: params.userCollateralAccount,
        userTransferAuthority: this.client.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
  }
}