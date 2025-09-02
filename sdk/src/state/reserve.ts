import { PublicKey, AccountInfo } from '@solana/web3.js';

export interface ReserveConfigFlags {
  depositsDisabled: boolean;
  withdrawalsDisabled: boolean;
  borrowingDisabled: boolean;
  repaymentsDisabled: boolean;
  liquidationsDisabled: boolean;
  collateralEnabled: boolean;
}

export interface ReserveConfig {
  loanToValueRatioBps: bigint;
  liquidationThresholdBps: bigint;
  liquidationPenaltyBps: bigint;
  baseBorrowRateBps: bigint;
  borrowRateMultiplierBps: bigint;
  jumpRateMultiplierBps: bigint;
  optimalUtilizationRateBps: bigint;
  protocolFeeBps: bigint;
  maxBorrowRateBps: bigint;
  decimals: number;
  flags: ReserveConfigFlags;
}

export interface Decimal {
  value: bigint;
}

export interface ReserveState {
  availableLiquidity: bigint;
  totalBorrows: bigint;
  totalLiquidity: bigint;
  collateralMintSupply: bigint;
  currentBorrowRate: Decimal;
  currentSupplyRate: Decimal;
  currentUtilizationRate: Decimal;
  accumulatedProtocolFees: bigint;
}

export interface ReserveData {
  version: number;
  market: PublicKey;
  liquidityMint: PublicKey;
  collateralMint: PublicKey;
  liquiditySupply: PublicKey;
  feeReceiver: PublicKey;
  priceOracle: PublicKey;
  oracleFeedId: Buffer;
  config: ReserveConfig;
  state: ReserveState;
  lastUpdateTimestamp: bigint;
  lastUpdateSlot: bigint;
  reentrancyGuard: boolean;
}

export class Reserve {
  static readonly ACCOUNT_SIZE = 679;

  constructor(
    public address: PublicKey,
    public data: ReserveData
  ) {}

  static fromAccountInfo(address: PublicKey, accountInfo: AccountInfo<Buffer>): Reserve {
    if (!accountInfo.data) {
      throw new Error('Invalid reserve account data');
    }

    let offset = 0;
    const data = accountInfo.data;

    const version = data.readUInt8(offset);
    offset += 1;

    const market = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const liquidityMint = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const collateralMint = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const liquiditySupply = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const feeReceiver = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const priceOracle = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const oracleFeedId = data.subarray(offset, offset + 32);
    offset += 32;

    const config: ReserveConfig = {
      loanToValueRatioBps: data.readBigUInt64LE(offset),
      liquidationThresholdBps: data.readBigUInt64LE(offset + 8),
      liquidationPenaltyBps: data.readBigUInt64LE(offset + 16),
      baseBorrowRateBps: data.readBigUInt64LE(offset + 24),
      borrowRateMultiplierBps: data.readBigUInt64LE(offset + 32),
      jumpRateMultiplierBps: data.readBigUInt64LE(offset + 40),
      optimalUtilizationRateBps: data.readBigUInt64LE(offset + 48),
      protocolFeeBps: data.readBigUInt64LE(offset + 56),
      maxBorrowRateBps: data.readBigUInt64LE(offset + 64),
      decimals: data.readUInt8(offset + 72),
      flags: {
        depositsDisabled: data.readUInt8(offset + 73) === 1,
        withdrawalsDisabled: data.readUInt8(offset + 74) === 1,
        borrowingDisabled: data.readUInt8(offset + 75) === 1,
        repaymentsDisabled: data.readUInt8(offset + 76) === 1,
        liquidationsDisabled: data.readUInt8(offset + 77) === 1,
        collateralEnabled: data.readUInt8(offset + 78) === 1,
      }
    };
    offset += 79;

    const state: ReserveState = {
      availableLiquidity: data.readBigUInt64LE(offset),
      totalBorrows: data.readBigUInt64LE(offset + 8),
      totalLiquidity: data.readBigUInt64LE(offset + 16),
      collateralMintSupply: data.readBigUInt64LE(offset + 24),
      currentBorrowRate: { value: data.readBigUInt64LE(offset + 32) },
      currentSupplyRate: { value: data.readBigUInt64LE(offset + 48) },
      currentUtilizationRate: { value: data.readBigUInt64LE(offset + 64) },
      accumulatedProtocolFees: data.readBigUInt64LE(offset + 80),
    };
    offset += 88;

    const lastUpdateTimestamp = data.readBigUInt64LE(offset);
    offset += 8;

    const lastUpdateSlot = data.readBigUInt64LE(offset);
    offset += 8;

    const reentrancyGuard = data.readUInt8(offset) === 1;

    return new Reserve(address, {
      version,
      market,
      liquidityMint,
      collateralMint,
      liquiditySupply,
      feeReceiver,
      priceOracle,
      oracleFeedId,
      config,
      state,
      lastUpdateTimestamp,
      lastUpdateSlot,
      reentrancyGuard,
    });
  }

  isDepositsDisabled(): boolean {
    return this.data.config.flags.depositsDisabled;
  }

  isWithdrawalsDisabled(): boolean {
    return this.data.config.flags.withdrawalsDisabled;
  }

  isBorrowingDisabled(): boolean {
    return this.data.config.flags.borrowingDisabled;
  }

  isCollateralEnabled(): boolean {
    return this.data.config.flags.collateralEnabled;
  }

  getUtilizationRate(): number {
    return Number(this.data.state.currentUtilizationRate.value) / 1e18;
  }

  getBorrowRate(): number {
    return Number(this.data.state.currentBorrowRate.value) / 1e18;
  }

  getSupplyRate(): number {
    return Number(this.data.state.currentSupplyRate.value) / 1e18;
  }
}