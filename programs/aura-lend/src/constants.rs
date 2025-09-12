use anchor_lang::prelude::*;
use crate::state::{ReserveConfig, ReserveState};

/// Current program version for upgrade compatibility
pub const PROGRAM_VERSION: u8 = 1;

/// Seeds used for Program Derived Address (PDA) generation
pub const MARKET_SEED: &[u8] = b"market";
pub const RESERVE_SEED: &[u8] = b"reserve";
pub const OBLIGATION_SEED: &[u8] = b"obligation";
pub const COLLATERAL_TOKEN_SEED: &[u8] = b"collateral";
pub const LIQUIDITY_TOKEN_SEED: &[u8] = b"liquidity";

/// RBAC system seeds
pub const MULTISIG_SEED: &[u8] = b"multisig";
pub const TIMELOCK_SEED: &[u8] = b"timelock";
pub const GOVERNANCE_SEED: &[u8] = b"governance";

/// Maximum number of reserves allowed in a single market
/// Increased from 32 to 128 to support more asset types
pub const MAX_BATCH_OPERATIONS: usize = 50;
pub const MAX_RESERVES: usize = 128;
/// Maximum number of obligations that can be tracked
/// Increased from 1000 to 10000 for better scalability
pub const MAX_OBLIGATIONS: usize = 10_000;

/// High precision constant for financial calculations (18 decimal places)
pub const PRECISION: u64 = 1_000_000_000_000_000_000; // 1e18
/// Precision for percentage calculations (4 decimal places, 100% = 10,000)
pub const PERCENT_PRECISION: u64 = 10_000;
/// Precision for basis points calculations (1 bp = 1/10,000)
pub const BASIS_POINTS_PRECISION: u64 = 10_000;

/// Maximum safe value for calculations to prevent overflow
pub const MAX_SAFE_VALUE: u128 = u128::MAX / 1_000_000;
/// Minimum safe value to prevent underflow in calculations
pub const MIN_SAFE_VALUE: u128 = 1;

/// Number of seconds in a year for interest rate calculations
pub const SECONDS_PER_YEAR: u64 = 365 * 24 * 3600; // 31,536,000
/// Approximate number of slots per year on Solana (~2 slots/second)
pub const SLOTS_PER_YEAR: u64 = SECONDS_PER_YEAR * 2;

/// Maximum liquidation bonus that can be set (50%)
pub const MAX_LIQUIDATION_BONUS_BPS: u64 = 5000;
/// Minimum liquidation threshold that can be set (10%)
pub const MIN_LIQUIDATION_THRESHOLD_BPS: u64 = 1000;
/// Maximum loan-to-value ratio allowed (90%)
pub const MAX_LOAN_TO_VALUE_RATIO_BPS: u64 = 9000;

/// Maximum age of oracle data in slots before considered stale (~2 minutes)
pub const MAX_ORACLE_STALENESS_SLOTS: u64 = 240;
/// Emergency oracle staleness limit for extreme situations (~3 hours)
pub const EMERGENCY_ORACLE_STALENESS_SLOTS: u64 = 21600;

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
// Increased from 8 to 16 for better portfolio diversification
pub const MAX_OBLIGATION_RESERVES: usize = 16;

// RBAC Timelock delays (in seconds)
/// Critical operations - 7 days
pub const TIMELOCK_DELAY_CRITICAL: u64 = 7 * 24 * 3600; // 604,800 seconds
/// High priority operations - 3 days  
pub const TIMELOCK_DELAY_HIGH: u64 = 3 * 24 * 3600; // 259,200 seconds
/// Medium priority operations - 1 day
pub const TIMELOCK_DELAY_MEDIUM: u64 = 24 * 3600; // 86,400 seconds
/// Low priority operations - 6 hours
pub const TIMELOCK_DELAY_LOW: u64 = 6 * 3600; // 21,600 seconds
/// Default delay for unspecified operations
pub const TIMELOCK_DELAY_DEFAULT: u64 = TIMELOCK_DELAY_MEDIUM;

// Minimum delays (cannot be set lower than these)
pub const TIMELOCK_MIN_CRITICAL_DELAY: u64 = 3 * 24 * 3600; // 3 days minimum
pub const TIMELOCK_MIN_HIGH_DELAY: u64 = 24 * 3600; // 1 day minimum
pub const TIMELOCK_MIN_STANDARD_DELAY: u64 = 3600; // 1 hour minimum

// Timelock expiry period - proposals expire if not executed within this time after execution_time
pub const TIMELOCK_EXPIRY_PERIOD: i64 = 30 * 24 * 3600; // 30 days

// Emergency role constraints
/// Maximum duration for emergency roles (24 hours)
pub const EMERGENCY_ROLE_MAX_DURATION: i64 = 24 * 3600;

// MultSig constraints
/// Maximum number of signatories in a multisig
/// Increased from 10 to 20 for larger governance councils
pub const MAX_MULTISIG_SIGNATORIES: usize = 20;
/// Minimum threshold for multisig
pub const MIN_MULTISIG_THRESHOLD: u8 = 1;

// Governance constraints
/// Maximum number of concurrent roles per registry
/// Increased from 50 to 200 for larger organizations
pub const MAX_GOVERNANCE_ROLES: usize = 200;
/// Default role expiration (1 year)
pub const DEFAULT_ROLE_EXPIRATION: i64 = 365 * 24 * 3600;

// Configuration system constants
/// Default protocol fee (1%)
pub const DEFAULT_PROTOCOL_FEE: u64 = 100;
/// Maximum protocol fee (5%)
pub const MAX_PROTOCOL_FEE: u64 = 500;
/// Liquidation close factor (50%)
pub const LIQUIDATION_CLOSE_FACTOR: u64 = 5000;
/// Maximum liquidation bonus (20%)
pub const MAX_LIQUIDATION_BONUS: u64 = 2000;
/// Minimum health factor (1.0)
pub const MIN_HEALTH_FACTOR: u64 = PRECISION;
/// Maximum LTV ratio (90%)
pub const MAX_LTV_RATIO: u64 = 9000;
/// Minimum liquidation threshold (50%)
pub const MIN_LIQUIDATION_THRESHOLD: u64 = 5000;
/// Oracle staleness threshold in slots (2 minutes)
pub const ORACLE_STALENESS_THRESHOLD: u64 = 240;
/// Oracle confidence threshold (1%)
pub const ORACLE_CONFIDENCE_THRESHOLD: u64 = 100;
/// Minimum oracle sources required
pub const MIN_ORACLE_SOURCES: u8 = 3;
/// Default timelock delay (1 hour)
pub const DEFAULT_TIMELOCK_DELAY: u64 = 3600;
/// Compute unit limit for instructions
pub const COMPUTE_UNIT_LIMIT: u32 = 400_000;
/// Maximum accounts per instruction
pub const MAX_ACCOUNTS_PER_INSTRUCTION: u8 = 32;
/// Default pagination limit
pub const PAGINATION_DEFAULT_LIMIT: u64 = 50;
/// Maximum pagination limit
pub const PAGINATION_MAX_LIMIT: u64 = 1000;