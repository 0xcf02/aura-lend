export * from './client';
export * from './instructions';
export * from './state';
export { InitializeMarketParams, InitializeReserveParams } from './types';
export { AuraLend, IDL } from './idl';

// Main client class
export { AuraLendClient } from './client';

// State types
export {
  Market,
  Reserve,
  Obligation,
  ReserveConfig,
  ReserveState,
  ObligationCollateral,
  ObligationLiquidity
} from './state';

// Instruction builders
export {
  MarketInstructions,
  LendingInstructions,
  BorrowingInstructions,
  LiquidationInstructions
} from './instructions';

// Re-export commonly used types
export { Connection, PublicKey, Keypair, Transaction } from '@solana/web3.js';
export { AnchorProvider, Wallet } from '@coral-xyz/anchor';
export { BN } from 'bn.js';