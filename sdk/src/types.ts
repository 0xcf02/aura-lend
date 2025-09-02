import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

// Main program interface
export interface AuraLend {
  version: '0.1.0';
  name: 'aura_lend';
  instructions: Array<{
    name: string;
    accounts: Array<{
      name: string;
      isMut: boolean;
      isSigner: boolean;
    }>;
    args: Array<{
      name: string;
      type: string;
    }>;
  }>;
  accounts: Array<{
    name: string;
    type: {
      kind: 'struct';
      fields: Array<{
        name: string;
        type: string;
      }>;
    };
  }>;
  types: Array<{
    name: string;
    type: {
      kind: 'struct' | 'enum';
      fields?: Array<{
        name: string;
        type: string;
      }>;
    };
  }>;
  errors: Array<{
    code: number;
    name: string;
    msg: string;
  }>;
}

// Market configuration
export interface MarketConfig {
  owner: PublicKey;
  emergencyAuthority: PublicKey;
  quoteCurrency: PublicKey;
  auraTokenMint: PublicKey;
}

// Reserve configuration
export interface ReserveConfig {
  loanToValueRatioBps: number;
  liquidationThresholdBps: number;
  liquidationPenaltyBps: number;
  baseBorrowRateBps: number;
  borrowRateMultiplierBps: number;
  jumpRateMultiplierBps: number;
  optimalUtilizationRateBps: number;
  protocolFeeBps: number;
  maxBorrowRateBps: number;
  decimals: number;
  flags: ReserveConfigFlags;
}

export interface ReserveConfigFlags {
  depositsDisabled: boolean;
  withdrawalsDisabled: boolean;
  borrowingDisabled: boolean;
  repaymentsDisabled: boolean;
  liquidationsDisabled: boolean;
  collateralEnabled: boolean;
}

// Reserve state
export interface ReserveState {
  availableLiquidity: BN;
  totalBorrows: BN;
  totalLiquidity: BN;
  collateralMintSupply: BN;
  currentBorrowRate: Decimal;
  currentSupplyRate: Decimal;
  currentUtilizationRate: Decimal;
  accumulatedProtocolFees: BN;
}

// Decimal type for high-precision calculations
export class Decimal {
  public value: BN;

  constructor(value: BN | number | string) {
    this.value = new BN(value);
  }

  static fromNumber(num: number): Decimal {
    const PRECISION = new BN(10).pow(new BN(18));
    return new Decimal(new BN(num).mul(PRECISION));
  }

  toNumber(): number {
    const PRECISION = new BN(10).pow(new BN(18));
    return this.value.div(PRECISION).toNumber();
  }

  add(other: Decimal): Decimal {
    return new Decimal(this.value.add(other.value));
  }

  sub(other: Decimal): Decimal {
    return new Decimal(this.value.sub(other.value));
  }

  mul(other: Decimal): Decimal {
    const PRECISION = new BN(10).pow(new BN(18));
    return new Decimal(this.value.mul(other.value).div(PRECISION));
  }

  div(other: Decimal): Decimal {
    const PRECISION = new BN(10).pow(new BN(18));
    return new Decimal(this.value.mul(PRECISION).div(other.value));
  }

  isZero(): boolean {
    return this.value.isZero();
  }

  toString(): string {
    return this.value.toString();
  }
}

// Obligation collateral
export interface ObligationCollateral {
  depositReserve: PublicKey;
  depositedAmount: BN;
  marketValueUsd: Decimal;
  ltvBps: number;
  liquidationThresholdBps: number;
}

// Obligation liquidity
export interface ObligationLiquidity {
  borrowReserve: PublicKey;
  borrowedAmountWads: Decimal;
  marketValueUsd: Decimal;
}

// Market flags
export interface MarketFlags {
  paused: boolean;
  emergency: boolean;
  lendingDisabled: boolean;
  borrowingDisabled: boolean;
  liquidationDisabled: boolean;
}

// Instruction parameter types
export interface InitializeMarketParams {
  owner: PublicKey;
  emergencyAuthority: PublicKey;
  quoteCurrency: PublicKey;
  auraTokenMint: PublicKey;
}

export interface InitializeReserveParams {
  liquidityMint: PublicKey;
  priceOracle: PublicKey;
  oracleFeedId: Buffer;
  config: ReserveConfig;
}

export interface UpdateReserveConfigParams {
  config: ReserveConfig;
}

export interface LiquidationParams {
  liquidityAmount: BN;
  minCollateralAmount: BN;
}

// Oracle price information
export interface OraclePrice {
  price: BN;
  confidence: BN;
  exponent: number;
  publishTime: BN;
}

// Transaction builder options
export interface TransactionBuilderOptions {
  feePayer?: PublicKey;
  recentBlockhash?: string;
}

// Client configuration
export interface ClientConfig {
  cluster: 'devnet' | 'mainnet-beta' | 'localnet';
  endpoint?: string;
  commitment?: 'processed' | 'confirmed' | 'finalized';
}

// Health factor categories
export enum HealthFactorCategory {
  Excellent = 'excellent', // > 2.0
  Good = 'good',          // 1.5 - 2.0
  Risky = 'risky',        // 1.1 - 1.5
  Dangerous = 'dangerous', // 1.0 - 1.1
  Liquidatable = 'liquidatable' // < 1.0
}

// Interest rate model types
export enum InterestRateModel {
  Linear = 'linear',
  Kinked = 'kinked',
  Exponential = 'exponential'
}

// Asset types supported by the protocol
export enum AssetType {
  SOL = 'SOL',
  USDC = 'USDC',
  USDT = 'USDT',
  ETH = 'ETH',
  BTC = 'BTC'
}

// Notification types for events
export interface LendingEvent {
  type: 'deposit' | 'withdraw' | 'borrow' | 'repay' | 'liquidation';
  user: PublicKey;
  asset: PublicKey;
  amount: BN;
  timestamp: number;
  signature: string;
}

// Protocol statistics
export interface ProtocolStats {
  totalValueLocked: number;
  totalBorrowed: number;
  totalReserves: number;
  totalUsers: number;
  utilizationRate: number;
  averageSupplyAPY: number;
  averageBorrowAPY: number;
}

// Reserve statistics
export interface ReserveStats {
  asset: AssetType;
  totalDeposited: number;
  totalBorrowed: number;
  utilizationRate: number;
  supplyAPY: number;
  borrowAPY: number;
  price: number;
  priceChange24h: number;
}

// User position summary
export interface UserPosition {
  totalCollateralValueUSD: number;
  totalBorrowedValueUSD: number;
  healthFactor: number;
  liquidationPrice: number;
  availableToBorrow: number;
  deposits: Array<{
    asset: AssetType;
    amount: number;
    valueUSD: number;
    apy: number;
  }>;
  borrows: Array<{
    asset: AssetType;
    amount: number;
    valueUSD: number;
    apy: number;
  }>;
}

// Liquidation opportunity
export interface LiquidationOpportunity {
  obligation: PublicKey;
  user: PublicKey;
  healthFactor: number;
  collateralValue: number;
  borrowedValue: number;
  maxLiquidationAmount: number;
  expectedProfit: number;
  collateralAsset: AssetType;
  borrowAsset: AssetType;
}