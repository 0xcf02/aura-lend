import { AccountInfo, PublicKey } from '@solana/web3.js';

export interface MarketFlags {
  paused: boolean;
  emergency: boolean;
  lendingDisabled: boolean;
  borrowingDisabled: boolean;
  liquidationDisabled: boolean;
}

export interface MarketData {
  version: number;
  owner: PublicKey;
  emergencyAuthority: PublicKey;
  quoteCurrency: PublicKey;
  auraTokenMint: PublicKey;
  auraMintAuthority: PublicKey;
  reservesCount: bigint;
  totalFeesCollected: bigint;
  lastUpdateTimestamp: bigint;
  flags: MarketFlags;
}

export class Market {
  constructor(
    public address: PublicKey,
    public data: MarketData
  ) {}

  static fromAccountInfo(address: PublicKey, accountInfo: AccountInfo<Buffer>): Market {
    if (!accountInfo.data) {
      throw new Error('Invalid market account data');
    }

    let offset = 0;
    const data = accountInfo.data;

    const version = data.readUInt8(offset);
    offset += 1;

    const owner = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const emergencyAuthority = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const quoteCurrency = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const auraTokenMint = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const auraMintAuthority = new PublicKey(data.subarray(offset, offset + 32));
    offset += 32;

    const reservesCount = data.readBigUInt64LE(offset);
    offset += 8;

    const totalFeesCollected = data.readBigUInt64LE(offset);
    offset += 8;

    const lastUpdateTimestamp = data.readBigUInt64LE(offset);
    offset += 8;

    const flags: MarketFlags = {
      paused: data.readUInt8(offset) === 1,
      emergency: data.readUInt8(offset + 1) === 1,
      lendingDisabled: data.readUInt8(offset + 2) === 1,
      borrowingDisabled: data.readUInt8(offset + 3) === 1,
      liquidationDisabled: data.readUInt8(offset + 4) === 1,
    };

    return new Market(address, {
      version,
      owner,
      emergencyAuthority,
      quoteCurrency,
      auraTokenMint,
      auraMintAuthority,
      reservesCount,
      totalFeesCollected,
      lastUpdateTimestamp,
      flags,
    });
  }

  isPaused(): boolean {
    return this.data.flags.paused;
  }

  isEmergency(): boolean {
    return this.data.flags.emergency;
  }

  isLendingDisabled(): boolean {
    return this.data.flags.lendingDisabled;
  }

  isBorrowingDisabled(): boolean {
    return this.data.flags.borrowingDisabled;
  }

  isLiquidationDisabled(): boolean {
    return this.data.flags.liquidationDisabled;
  }
}