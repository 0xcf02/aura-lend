import { PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { AuraLendClient } from '../client';
import BN from 'bn.js';

export interface InitObligationParams {
  obligationOwner: PublicKey;
}

export interface BorrowObligationLiquidityParams {
  obligation: PublicKey;
  borrowReserve: PublicKey;
  amount: BN;
  destinationLiquidity: PublicKey;
}

export class BorrowingInstructions {
  constructor(private client: AuraLendClient) {}

  async initObligation(params: InitObligationParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();
    const obligationPda = this.client.getObligationAddress(params.obligationOwner);

    return this.client.program.methods
      .initObligation()
      .accounts({
        market: marketPda,
        obligation: obligationPda,
        obligationOwner: params.obligationOwner,
        payer: this.client.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .transaction();
  }

  async borrowObligationLiquidity(params: BorrowObligationLiquidityParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();
    const reserve = await this.client.getReserve(params.borrowReserve);
    
    if (!reserve) {
      throw new Error('Reserve not found');
    }

    const liquiditySupplyPda = this.client.getLiquiditySupplyAddress(reserve.data.liquidityMint);
    
    const [liquiditySupplyAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), reserve.data.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    return this.client.program.methods
      .borrowObligationLiquidity(params.amount)
      .accounts({
        market: marketPda,
        obligation: params.obligation,
        borrowReserve: params.borrowReserve,
        liquiditySupply: liquiditySupplyPda,
        liquiditySupplyAuthority,
        destinationLiquidity: params.destinationLiquidity,
        obligationOwner: this.client.wallet.publicKey,
        priceOracle: reserve.data.priceOracle,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .transaction();
  }
}