import { PublicKey, AccountInfo } from '@solana/web3.js';

export interface Decimal {
  value: bigint;
}

export interface ObligationCollateral {
  depositReserve: PublicKey;
  depositedAmount: bigint;
  marketValueUsd: Decimal;
  ltvBps: bigint;
  liquidationThresholdBps: bigint;
}

export interface ObligationLiquidity {
  borrowReserve: PublicKey;
  borrowedAmountWads: Decimal;
  marketValueUsd: Decimal;
}

export interface ObligationData {
  version: number;
  market: PublicKey;
  owner: PublicKey;
  deposits: ObligationCollateral[];
  borrows: ObligationLiquidity[];
  depositedValueUsd: Decimal;
  borrowedValueUsd: Decimal;
  lastUpdateTimestamp: bigint;
  lastUpdateSlot: bigint;
  liquidationSnapshotHealthFactor?: Decimal;
}

export class Obligation {
  static readonly ACCOUNT_SIZE = 376;

  constructor(
    public address: PublicKey,
    public data: ObligationData
  ) {}

  static fromAccountInfo(address: PublicKey, accountInfo: AccountInfo<Buffer>): Obligation {
    if (!accountInfo.data) {
      throw new Error('Invalid obligation account data');
    }

    let offset = 0;
    const data = accountInfo.data;

    const version = data.readUInt8(offset);
    offset += 1;

    const market = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const owner = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const depositsLength = data.readUInt32LE(offset);
    offset += 4;

    const deposits: ObligationCollateral[] = [];
    for (let i = 0; i < depositsLength; i++) {
      const depositReserve = new PublicKey(data.subarray(offset, offset + 32));
      offset += 32;
      
      const depositedAmount = data.readBigUInt64LE(offset);
      offset += 8;
      
      const marketValueUsd = { value: data.readBigUInt64LE(offset) };
      offset += 16;
      
      const ltvBps = data.readBigUInt64LE(offset);
      offset += 8;
      
      const liquidationThresholdBps = data.readBigUInt64LE(offset);
      offset += 8;

      deposits.push({
        depositReserve,
        depositedAmount,
        marketValueUsd,
        ltvBps,
        liquidationThresholdBps,
      });
    }

    const borrowsLength = data.readUInt32LE(offset);
    offset += 4;

    const borrows: ObligationLiquidity[] = [];
    for (let i = 0; i < borrowsLength; i++) {
      const borrowReserve = new PublicKey(data.subarray(offset, offset + 32));
      offset += 32;
      
      const borrowedAmountWads = { value: data.readBigUInt64LE(offset) };
      offset += 16;
      
      const marketValueUsd = { value: data.readBigUInt64LE(offset) };
      offset += 16;

      borrows.push({
        borrowReserve,
        borrowedAmountWads,
        marketValueUsd,
      });
    }

    const depositedValueUsd = { value: data.readBigUInt64LE(offset) };
    offset += 16;

    const borrowedValueUsd = { value: data.readBigUInt64LE(offset) };
    offset += 16;

    const lastUpdateTimestamp = data.readBigUInt64LE(offset);
    offset += 8;

    const lastUpdateSlot = data.readBigUInt64LE(offset);
    offset += 8;

    const hasLiquidationSnapshot = data.readUInt8(offset) === 1;
    offset += 1;
    
    let liquidationSnapshotHealthFactor: Decimal | undefined;
    if (hasLiquidationSnapshot) {
      liquidationSnapshotHealthFactor = { value: data.readBigUInt64LE(offset) };
    }

    return new Obligation(address, {
      version,
      market,
      owner,
      deposits,
      borrows,
      depositedValueUsd,
      borrowedValueUsd,
      lastUpdateTimestamp,
      lastUpdateSlot,
      liquidationSnapshotHealthFactor,
    });
  }

  getHealthFactor(): number {
    if (this.data.borrowedValueUsd.value === 0n) {
      return Infinity;
    }

    const collateralValue = Number(this.data.depositedValueUsd.value);
    const borrowedValue = Number(this.data.borrowedValueUsd.value);
    
    return collateralValue / borrowedValue;
  }

  isHealthy(): boolean {
    return this.getHealthFactor() > 1.0;
  }

  getLoanToValue(): number {
    if (this.data.depositedValueUsd.value === 0n) {
      return 0;
    }

    const collateralValue = Number(this.data.depositedValueUsd.value);
    const borrowedValue = Number(this.data.borrowedValueUsd.value);
    
    return borrowedValue / collateralValue;
  }

  getTotalDeposits(): number {
    return Number(this.data.depositedValueUsd.value) / 1e18;
  }

  getTotalBorrows(): number {
    return Number(this.data.borrowedValueUsd.value) / 1e18;
  }
}