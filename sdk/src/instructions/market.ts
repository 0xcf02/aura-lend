import { Transaction, PublicKey, SystemProgram } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { AuraLendClient } from '../client';
import { InitializeMarketParams, InitializeReserveParams } from '../types';

export class MarketInstructions {
  constructor(private client: AuraLendClient) {}

  async initializeMarket(params: InitializeMarketParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();
    
    const [auraTokenMint] = PublicKey.findProgramAddressSync(
      [Buffer.from('aura_mint')],
      this.client.programId
    );

    const [auraMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('aura_mint_authority')],
      this.client.programId
    );

    return this.client.program.methods
      .initializeMarket(params)
      .accounts({
        market: marketPda,
        quoteCurrencyMint: params.quoteCurrency,
        auraTokenMint,
        auraMintAuthority,
        payer: this.client.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .transaction();
  }

  async initializeReserve(params: InitializeReserveParams): Promise<Transaction> {
    const marketPda = this.client.getMarketAddress();
    const reservePda = this.client.getReserveAddress(params.liquidityMint);
    const collateralMintPda = this.client.getCollateralMintAddress(params.liquidityMint);
    const liquiditySupplyPda = this.client.getLiquiditySupplyAddress(params.liquidityMint);
    
    const [collateralMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), params.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    const [liquiditySupplyAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), params.liquidityMint.toBuffer(), Buffer.from('authority')],
      this.client.programId
    );

    // Create fee receiver ATA
    const feeReceiver = await this.client.createAssociatedTokenAccount(
      params.liquidityMint,
      this.client.wallet.publicKey
    );

    return this.client.program.methods
      .initializeReserve(params)
      .accounts({
        market: marketPda,
        reserve: reservePda,
        liquidityMint: params.liquidityMint,
        collateralMint: collateralMintPda,
        collateralMintAuthority,
        liquiditySupply: liquiditySupplyPda,
        liquiditySupplyAuthority,
        feeReceiver,
        owner: this.client.wallet.publicKey,
        payer: this.client.wallet.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: PublicKey.default, // SYSVAR_RENT_PUBKEY
      })
      .transaction();
  }
}