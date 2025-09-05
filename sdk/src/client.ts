// External dependencies
import {
  Commitment,
  Connection,
  Keypair,
  PublicKey,
  SendOptions,
  Transaction,
} from '@solana/web3.js';
import { AnchorProvider, Program, Wallet } from '@coral-xyz/anchor';
import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from '@solana/spl-token';

// Internal imports
import { AuraLend, IDL } from './idl/aura_lend';
import { BorrowingInstructions } from './instructions/borrowing';
import { LendingInstructions } from './instructions/lending';
import { LiquidationInstructions } from './instructions/liquidation';
import { MarketInstructions } from './instructions/market';
import { Market, Obligation, Reserve } from './state';

/**
 * Configuration interface for initializing the AuraLendClient
 */
export interface AuraLendClientConfig {
  /** Solana RPC connection instance */
  connection: Connection;
  /** Wallet instance for signing transactions */
  wallet: Wallet;
  /** Program ID of the AuraLend protocol */
  programId: PublicKey;
  /** Transaction commitment level (defaults to 'confirmed') */
  commitment?: Commitment;
}

/**
 * Main client for interacting with the Aura Lend Protocol
 * 
 * Provides a high-level interface for all protocol operations including:
 * - Market management and configuration
 * - Lending operations (deposit/withdraw)
 * - Borrowing operations (borrow/repay)
 * - Liquidation mechanisms
 * - Account state queries and PDA derivations
 * 
 * @example
 * ```typescript
 * const client = new AuraLendClient({
 *   connection: new Connection('https://api.devnet.solana.com'),
 *   wallet: new Wallet(keypair),
 *   programId: new PublicKey('AuRa1Lend1111111111111111111111111111111111')
 * });
 * 
 * // Deposit liquidity
 * await client.lending.depositLiquidity({
 *   reserve: usdcReserve,
 *   amount: 1000_000_000, // 1000 USDC
 *   userTokenAccount: userUsdcAccount,
 *   userCollateralAccount: userAusdcAccount
 * });
 * ```
 */
export class AuraLendClient {
  /** Solana RPC connection instance */
  public readonly connection: Connection;
  /** Wallet instance for signing transactions */
  public readonly wallet: Wallet;
  /** Anchor program instance for the AuraLend protocol */
  public readonly program: Program;
  /** Program ID of the AuraLend protocol */
  public readonly programId: PublicKey;

  /** Market instruction builder for market operations */
  public readonly market: MarketInstructions;
  /** Lending instruction builder for deposit/withdraw operations */
  public readonly lending: LendingInstructions;
  /** Borrowing instruction builder for borrow/repay operations */
  public readonly borrowing: BorrowingInstructions;
  /** Liquidation instruction builder for liquidation operations */
  public readonly liquidation: LiquidationInstructions;

  /**
   * Creates a new AuraLendClient instance
   * 
   * @param config - Client configuration options
   */
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

  /**
   * Sends and confirms a transaction with proper error handling
   * 
   * @param transaction - Transaction to send
   * @param signers - Additional signers required for the transaction (wallet is automatically included)
   * @param options - Send options for the transaction
   * @returns Transaction signature
   */
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

  /**
   * Retrieves market account data
   * 
   * @param marketPubkey - Optional market public key (defaults to derived market PDA)
   * @returns Market account data or null if not found
   */
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

  /**
   * Retrieves reserve account data for a given mint
   * 
   * @param mint - The mint address of the token
   * @returns Reserve account data or null if not found
   */
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

  /**
   * Retrieves obligation account data for a given owner
   * 
   * @param owner - The owner's public key
   * @returns Obligation account data or null if not found
   */
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

  /**
   * Retrieves all reserve accounts from the program
   * 
   * @returns Array of all reserve accounts
   */
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

  /**
   * Retrieves all obligation accounts from the program
   * 
   * @returns Array of all obligation accounts
   */
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

  /**
   * Derives the market PDA address
   * 
   * @returns The market account public key
   */
  getMarketAddress(): PublicKey {
    const [marketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('market')],
      this.programId
    );
    return marketPda;
  }

  /**
   * Derives the reserve PDA address for a given liquidity mint
   * 
   * @param liquidityMint - The liquidity token mint address
   * @returns The reserve account public key
   */
  getReserveAddress(liquidityMint: PublicKey): PublicKey {
    const [reservePda] = PublicKey.findProgramAddressSync(
      [Buffer.from('reserve'), liquidityMint.toBuffer()],
      this.programId
    );
    return reservePda;
  }

  /**
   * Derives the obligation PDA address for a given owner
   * 
   * @param owner - The obligation owner's public key
   * @returns The obligation account public key
   */
  getObligationAddress(owner: PublicKey): PublicKey {
    const [obligationPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('obligation'), owner.toBuffer()],
      this.programId
    );
    return obligationPda;
  }

  /**
   * Derives the collateral mint PDA address for a given liquidity mint
   * 
   * @param liquidityMint - The liquidity token mint address
   * @returns The collateral mint public key
   */
  getCollateralMintAddress(liquidityMint: PublicKey): PublicKey {
    const [collateralMintPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), liquidityMint.toBuffer()],
      this.programId
    );
    return collateralMintPda;
  }

  /**
   * Derives the liquidity supply token account PDA address
   * 
   * @param liquidityMint - The liquidity token mint address
   * @returns The liquidity supply token account public key
   */
  getLiquiditySupplyAddress(liquidityMint: PublicKey): PublicKey {
    const [liquiditySupplyPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), liquidityMint.toBuffer()],
      this.programId
    );
    return liquiditySupplyPda;
  }

  /**
   * Derives the collateral mint authority PDA address
   * 
   * @param liquidityMint - The liquidity token mint address
   * @returns The collateral mint authority public key
   */
  getCollateralMintAuthorityAddress(liquidityMint: PublicKey): PublicKey {
    const [authorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('collateral'), liquidityMint.toBuffer(), Buffer.from('authority')],
      this.programId
    );
    return authorityPda;
  }

  /**
   * Derives the liquidity supply authority PDA address
   * 
   * @param liquidityMint - The liquidity token mint address
   * @returns The liquidity supply authority public key
   */
  getLiquiditySupplyAuthorityAddress(liquidityMint: PublicKey): PublicKey {
    const [authorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('liquidity'), liquidityMint.toBuffer(), Buffer.from('authority')],
      this.programId
    );
    return authorityPda;
  }

  /**
   * Gets the token balance of a token account
   * 
   * @param tokenAccount - The token account public key
   * @returns The token balance in UI amount (with decimals applied)
   */
  async getTokenBalance(tokenAccount: PublicKey): Promise<number> {
    try {
      const accountInfo = await this.connection.getTokenAccountBalance(tokenAccount);
      return accountInfo.value.uiAmount ?? 0;
    } catch {
      return 0;
    }
  }

  /**
   * Creates an Associated Token Account if it doesn't exist
   * 
   * @param mint - The token mint address
   * @param owner - The token account owner (defaults to wallet public key)
   * @returns The Associated Token Account public key
   */
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

  /**
   * Calculates the health factor for a position
   * 
   * Health factor = (collateral value * liquidation threshold) / borrowed value
   * A health factor < 1.0 indicates the position can be liquidated
   * 
   * @param collateralValueUSD - Total collateral value in USD
   * @param borrowedValueUSD - Total borrowed value in USD
   * @param liquidationThreshold - Liquidation threshold (as decimal, e.g., 0.8 for 80%)
   * @returns Health factor (higher is safer, < 1.0 = liquidatable)
   */
  static calculateHealthFactor(
    collateralValueUSD: number,
    borrowedValueUSD: number,
    liquidationThreshold: number
  ): number {
    if (borrowedValueUSD === 0) return Number.MAX_SAFE_INTEGER;
    return (collateralValueUSD * liquidationThreshold) / borrowedValueUSD;
  }

  /**
   * Calculates the maximum amount that can be borrowed against collateral
   * 
   * @param collateralValueUSD - Total collateral value in USD
   * @param loanToValueRatio - Loan-to-value ratio (as decimal, e.g., 0.75 for 75%)
   * @param currentBorrowedValueUSD - Current borrowed value in USD (defaults to 0)
   * @returns Maximum additional borrow amount in USD
   */
  static calculateMaxBorrowAmount(
    collateralValueUSD: number,
    loanToValueRatio: number,
    currentBorrowedValueUSD: number = 0
  ): number {
    const maxBorrow = collateralValueUSD * loanToValueRatio;
    return Math.max(0, maxBorrow - currentBorrowedValueUSD);
  }

  /**
   * Calculates the liquidation price for a collateral asset
   * 
   * This is the price at which the collateral asset would trigger liquidation
   * 
   * @param collateralAmount - Amount of collateral tokens
   * @param borrowedAmount - Amount of borrowed value in USD
   * @param liquidationThreshold - Liquidation threshold (as decimal, e.g., 0.8 for 80%)
   * @returns Liquidation price in USD per collateral token
   */
  static calculateLiquidationPrice(
    collateralAmount: number,
    borrowedAmount: number,
    liquidationThreshold: number
  ): number {
    if (collateralAmount === 0) return 0;
    return borrowedAmount / (collateralAmount * liquidationThreshold);
  }
}