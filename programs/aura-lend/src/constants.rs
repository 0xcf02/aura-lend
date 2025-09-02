use anchor_lang::prelude::*;

// Program constants
pub const PROGRAM_VERSION: u8 = 1;

// Seeds for PDA derivation  
pub const MARKET_SEED: &[u8] = b"market";
pub const RESERVE_SEED: &[u8] = b"reserve";
pub const OBLIGATION_SEED: &[u8] = b"obligation";
pub const COLLATERAL_TOKEN_SEED: &[u8] = b"collateral";
pub const LIQUIDITY_TOKEN_SEED: &[u8] = b"liquidity";

// Market configuration limits
pub const MAX_RESERVES: usize = 32;
pub const MAX_OBLIGATIONS: usize = 1000;

// Precision constants for calculations
pub const PRECISION: u64 = 1_000_000_000_000_000_000; // 18 decimals (1e18)
pub const PERCENT_PRECISION: u64 = 10_000; // 4 decimals (100%)
pub const BASIS_POINTS_PRECISION: u64 = 10_000; // 4 decimals

// Math safety constants
pub const MAX_SAFE_VALUE: u128 = u128::MAX / 1_000_000; // Safe upper bound for calculations
pub const MIN_SAFE_VALUE: u128 = 1; // Minimum value to prevent underflow

// Interest rate constants
pub const SECONDS_PER_YEAR: u64 = 365 * 24 * 3600; // 31,536,000
pub const SLOTS_PER_YEAR: u64 = SECONDS_PER_YEAR * 2; // ~2 slots per second on Solana

// Collateral and liquidation parameters
pub const MAX_LIQUIDATION_BONUS_BPS: u64 = 5000; // 50%
pub const MIN_LIQUIDATION_THRESHOLD_BPS: u64 = 1000; // 10%
pub const MAX_LOAN_TO_VALUE_RATIO_BPS: u64 = 9000; // 90%

// Oracle staleness limits (in slots)
pub const MAX_ORACLE_STALENESS_SLOTS: u64 = 240; // ~2 minutes
pub const EMERGENCY_ORACLE_STALENESS_SLOTS: u64 = 21600; // ~3 hours

// Time manipulation protection
pub const MIN_INTEREST_UPDATE_INTERVAL: u64 = 60; // 1 minute minimum between updates
pub const MAX_TIMESTAMP_FUTURE_TOLERANCE: u64 = 300; // 5 minutes max future
pub const MIN_TIMESTAMP_SOLANA_GENESIS: u64 = 1_609_459_200; // Solana genesis timestamp
pub const SLOT_TIMESTAMP_VARIANCE_BPS: u64 = 1000; // 10% variance allowed

// Minimum values to prevent dust
pub const MIN_DEPOSIT_AMOUNT: u64 = 1000; // Minimum deposit in base units
pub const MIN_BORROW_AMOUNT: u64 = 1000; // Minimum borrow in base units

// Flash loan parameters
pub const FLASH_LOAN_FEE_BPS: u64 = 9; // 0.09%

// Reserve configuration limits
pub const MAX_UTILIZATION_RATE_BPS: u64 = 10000; // 100%
pub const OPTIMAL_UTILIZATION_RATE_BPS: u64 = 8000; // 80%

// Token decimals
pub const NATIVE_MINT_DECIMALS: u8 = 9; // SOL decimals
pub const USDC_DECIMALS: u8 = 6;
pub const USDT_DECIMALS: u8 = 6;

// Account sizes for rent calculation
pub const MARKET_SIZE: usize = 8 + // discriminator 
    1 + // version
    32 + // owner
    32 + // emergency_authority
    32 + // quote_currency  
    32 + // aura_token_mint
    32 + // aura_mint_authority
    8 + // reserves_count
    8 + // total_fees_collected
    8 + // last_update_timestamp
    4 + // flags (MarketFlags)
    256; // reserved

pub const RESERVE_SIZE: usize = 8 + // discriminator
    1 + // version
    32 + // market
    32 + // liquidity_mint
    32 + // collateral_mint
    32 + // liquidity_supply
    32 + // fee_receiver
    32 + // price_oracle
    32 + // oracle_feed_id
    std::mem::size_of::<ReserveConfig>() + // config (approximately 80 bytes)
    std::mem::size_of::<ReserveState>() + // state (approximately 120 bytes)
    8 + // last_update_timestamp
    8 + // last_update_slot
    1 + // reentrancy_guard
    255; // reserved

pub const OBLIGATION_SIZE: usize = 8 + // discriminator
    1 + // version
    32 + // market
    32 + // owner
    4 + (MAX_OBLIGATION_RESERVES * 96) + // deposits (estimated 96 bytes per deposit)
    4 + (MAX_OBLIGATION_RESERVES * 64) + // borrows (estimated 64 bytes per borrow)
    16 + // deposited_value_usd (Decimal is u128)
    16 + // borrowed_value_usd
    8 + // last_update_timestamp
    8 + // last_update_slot
    128; // reserved

// Maximum number of deposits and borrows per obligation
pub const MAX_OBLIGATION_RESERVES: usize = 8;