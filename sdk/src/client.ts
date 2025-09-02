import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  Commitment,
  SendOptions,
} from '@solana/web3.js';
import { AnchorProvider, Program, Wallet, Idl } from '@coral-xyz/anchor';
import { getAssociatedTokenAddressSync, createAssociatedTokenAccountInstruction, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';

import { AuraLend, IDL } from './idl/aura_lend';
import { MarketInstructions } from './instructions/market';
import { LendingInstructions } from './instructions/lending';
import { BorrowingInstructions } from './instructions/borrowing';
import { LiquidationInstructions } from './instructions/liquidation';
import { Market, Reserve, Obligation } from './state';

export interface AuraLendClientConfig {
  connection: Connection;
  wallet: Wallet;
  programId: PublicKey;
  commitment?: Commitment;
}

export class AuraLendClient {
  public readonly connection: Connection;
  public readonly wallet: Wallet;
  public readonly program: Program;
  public readonly programId: PublicKey;

  // Instruction builders
  public readonly market: MarketInstructions;
  public readonly lending: LendingInstructions;
  public readonly borrowing: BorrowingInstructions;
  public readonly liquidation: LiquidationInstructions;

  constructor(config: AuraLendClientConfig) {
    this.connection = config.connection;
    this.wallet = config.wallet;
    this.programId = config.programId;

    const provider = new AnchorProvider(
      this.connection,
      this.wallet,
      {
        commitment: config.commitment || 'confirmed',
        preflightCommitment: config.commitment || 'confirmed',
      }
    );

    // Load program from IDL
    this.program = new Program(
      IDL as any,
      provider
    );

    // Initialize instruction builders
    this.market = new MarketInstructions(this);
    this.lending = new LendingInstructions(this);
    this.borrowing = new BorrowingInstructions(this);
    this.liquidation = new LiquidationInstructions(this);
  }

  // Convenience method to send transactions
  async sendAndConfirmTransaction(
    transaction: Transaction,
    signers: Keypair[] = [],
    options?: SendOptions
  ): Promise<string> {
    const signature = await this.connection.sendTransaction(
      transaction,
      [this.wallet.payer, ...signers],
      options
    );

    await this.connection.confirmTransaction(signature, 'confirmed');
    return signature;
  }

  // Market operations
  async getMarket(marketPubkey?: PublicKey): Promise<Market | null> {
    const marketKey = marketPubkey || this.getMarketAddress();
    
    try {
      const accountInfo = await this.connection.getAccountInfo(marketKey);
      if (!accountInfo) return null;
      return Market.fromAccountInfo(marketKey, accountInfo);
    } catch (error) {
      // Market not found - this is expected when market doesn't exist
      return null;
    }
  }

  async getReserve(mint: PublicKey): Promise<Reserve | null> {
    const reserveKey = this.getReserveAddress(mint);
    
    try {
      const accountInfo = await this.connection.getAccountInfo(reserveKey);
      if (!accountInfo) return null;
      return Reserve.fromAccountInfo(reserveKey, accountInfo);
    } catch (error) {
      // Reserve not found - this is expected when reserve doesn't exist
      return null;
    }
  }

  async getObligation(owner: PublicKey): Promise<Obligation | null> {
    const obligationKey = this.getObligationAddress(owner);
    
    try {
      const accountInfo = await this.connection.getAccountInfo(obligationKey);
      if (!accountInfo) return null;
      return Obligation.fromAccountInfo(obligationKey, accountInfo);
    } catch (error) {
      // Obligation not found - this is expected when obligation doesn't exist
      return null;
    }
  }

  async getAllReserves(): Promise<Reserve[]> {
    const programAccounts = await this.connection.getProgramAccounts(
      this.programId,
      {
        filters: [
          { dataSize: Reserve.ACCOUNT_SIZE },
          { memcmp: { offset: 0, bytes: '1' } } // version byte
        ]
      }
    );
    return programAccounts.map(({ pubkey, account }) => 
      Reserve.fromAccountInfo(pubkey, account)
    );
  }

  async getAllObligations(): Promise<Obligation[]> {
    const programAccounts = await this.connection.getProgramAccounts(
      this.programId,
      {
        filters: [
          { dataSize: Obligation.ACCOUNT_SIZE },
          { memcmp: { offset: 0, bytes: '1' } } // version byte
        ]
      }
    );
    return programAccounts.map(({ pubkey, account }) => 
      Obligation.fromAccountInfo(pubkey, account)
    );
  }

  // PDA derivation helpers
  getMarketAddress(): PublicKey {
    const [marketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('market')],
      this.programId
    );
    return marketPda;
  }

  getReserveAddress(liquidityMint: PublicKey): PublicKey {
    const [reservePda] = PublicKey.findProgramAddressSync(
      [Buffer.from('reserve'), liquidityMint.toBuffer()],
      this.programId
    );
    return reservePda;
  }

  getObligationAddress(owner: PublicKey): PublicKey {
    const [obligationPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('obligation'), owner.toBuffer()],
      this.programId
    );
    return obligationPda;
  }

  getCollateralMintAddress(liquidityMint: PublicKey): PublicKey {
    const [collateralMintPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), liquidityMint.toBuffer()],
      this.programId
    );
    return collateralMintPda;
  }

  getLiquiditySupplyAddress(liquidityMint: PublicKey): PublicKey {
    const [liquiditySupplyPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), liquidityMint.toBuffer()],
      this.programId
    );
    return liquiditySupplyPda;
  }

  getCollateralMintAuthorityAddress(liquidityMint: PublicKey): PublicKey {
    const [authorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), liquidityMint.toBuffer(), Buffer.from('authority')],
      this.programId
    );
    return authorityPda;
  }

  getLiquiditySupplyAuthorityAddress(liquidityMint: PublicKey): PublicKey {
    const [authorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), liquidityMint.toBuffer(), Buffer.from('authority')],
      this.programId
    );
    return authorityPda;
  }

  // Token helper methods
  async getTokenBalance(tokenAccount: PublicKey): Promise<number> {
    try {
      const accountInfo = await this.connection.getTokenAccountBalance(tokenAccount);
      return accountInfo.value.uiAmount ?? 0;
    } catch {
      return 0;
    }
  }

  async createAssociatedTokenAccount(
    mint: PublicKey,
    owner: PublicKey = this.wallet.publicKey
  ): Promise<PublicKey> {
    const ata = getAssociatedTokenAddressSync(mint, owner);

    const accountInfo = await this.connection.getAccountInfo(ata);
    
    if (!accountInfo) {
      const transaction = new Transaction().add(
        createAssociatedTokenAccountInstruction(
          this.wallet.publicKey,
          ata,
          owner,
          mint
        )
      );

      await this.sendAndConfirmTransaction(transaction);
    }

    return ata;
  }

  // Utility methods for calculations
  static calculateHealthFactor(
    collateralValueUSD: number,
    borrowedValueUSD: number,
    liquidationThreshold: number
  ): number {
    if (borrowedValueUSD === 0) return Number.MAX_SAFE_INTEGER;
    return (collateralValueUSD * liquidationThreshold) / borrowedValueUSD;
  }

  static calculateMaxBorrowAmount(
    collateralValueUSD: number,
    loanToValueRatio: number,
    currentBorrowedValueUSD: number = 0
  ): number {
    const maxBorrow = collateralValueUSD * loanToValueRatio;
    return Math.max(0, maxBorrow - currentBorrowedValueUSD);
  }

  static calculateLiquidationPrice(
    collateralAmount: number,
    borrowedAmount: number,
    liquidationThreshold: number
  ): number {
    if (collateralAmount === 0) return 0;
    return borrowedAmount / (collateralAmount * liquidationThreshold);
  }
}