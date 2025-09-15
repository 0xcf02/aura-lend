use crate::utils::get_validated_timestamp;
use anchor_lang::prelude::*;

/// Log levels for structured logging
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Event types for protocol monitoring
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum EventType {
    // Market events
    MarketInitialized,
    MarketPaused,
    MarketUnpaused,

    // Reserve events
    ReserveInitialized,
    ReserveConfigUpdated,
    LiquidityDeposited,
    LiquidityWithdrawn,
    InterestAccrued,

    // Obligation events
    ObligationInitialized,
    CollateralDeposited,
    CollateralWithdrawn,
    LiquidityBorrowed,
    LiquidityRepaid,

    // Liquidation events
    LiquidationExecuted,
    FlashLoanExecuted,

    // Oracle events
    PriceUpdated,
    OracleStale,
    PriceManipulationDetected,

    // Governance events
    ProposalCreated,
    ProposalSigned,
    ProposalExecuted,
    ProposalCancelled,
    RoleGranted,
    RoleRevoked,
    EmergencyActionTaken,

    // Security events
    ReentrancyDetected,
    UnauthorizedAccess,
    MathOverflow,
    InvalidOperation,

    // System events
    ProgramUpgraded,
    AccountMigrated,
    ConfigurationChanged,
}

/// Structured log entry
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LogEntry {
    pub timestamp: u64,
    pub slot: u64,
    pub level: LogLevel,
    pub event_type: EventType,
    pub message: String,
    pub user: Option<Pubkey>,
    pub market: Option<Pubkey>,
    pub reserve: Option<Pubkey>,
    pub obligation: Option<Pubkey>,
    pub amount: Option<u64>,
    pub additional_data: Option<String>,
}

/// Logger implementation for structured logging
pub struct Logger;

impl Logger {
    /// Log with structured data
    pub fn log(
        level: LogLevel,
        event_type: EventType,
        message: &str,
        user: Option<Pubkey>,
        market: Option<Pubkey>,
        reserve: Option<Pubkey>,
        obligation: Option<Pubkey>,
        amount: Option<u64>,
        additional_data: Option<String>,
    ) -> Result<()> {
        let (timestamp, slot) = get_validated_timestamp()?;

        let entry = LogEntry {
            timestamp,
            slot,
            level: level.clone(),
            event_type: event_type.clone(),
            message: message.to_string(),
            user,
            market,
            reserve,
            obligation,
            amount,
            additional_data,
        };

        // Emit structured log
        match level {
            LogLevel::Debug => {
                msg!(
                    "[DEBUG] {} | {:?} | {} | slot: {} | timestamp: {}",
                    Self::format_event_type(&event_type),
                    event_type,
                    message,
                    slot,
                    timestamp
                );
            }
            LogLevel::Info => {
                msg!(
                    "[INFO] {} | {:?} | {} | slot: {} | timestamp: {}",
                    Self::format_event_type(&event_type),
                    event_type,
                    message,
                    slot,
                    timestamp
                );
            }
            LogLevel::Warning => {
                msg!(
                    "[WARNING] {} | {:?} | {} | slot: {} | timestamp: {}",
                    Self::format_event_type(&event_type),
                    event_type,
                    message,
                    slot,
                    timestamp
                );
            }
            LogLevel::Error => {
                msg!(
                    "[ERROR] {} | {:?} | {} | slot: {} | timestamp: {}",
                    Self::format_event_type(&event_type),
                    event_type,
                    message,
                    slot,
                    timestamp
                );
            }
            LogLevel::Critical => {
                msg!(
                    "[CRITICAL] {} | {:?} | {} | slot: {} | timestamp: {}",
                    Self::format_event_type(&event_type),
                    event_type,
                    message,
                    slot,
                    timestamp
                );
            }
        }

        // Include additional context if provided
        if let Some(user) = user {
            msg!("  user: {}", user);
        }
        if let Some(market) = market {
            msg!("  market: {}", market);
        }
        if let Some(reserve) = reserve {
            msg!("  reserve: {}", reserve);
        }
        if let Some(obligation) = obligation {
            msg!("  obligation: {}", obligation);
        }
        if let Some(amount) = amount {
            msg!("  amount: {}", amount);
        }
        if let Some(data) = additional_data {
            msg!("  data: {}", data);
        }

        Ok(())
    }

    /// Helper to format event type for display
    fn format_event_type(event_type: &EventType) -> &'static str {
        match event_type {
            EventType::MarketInitialized => "MARKET_INIT",
            EventType::MarketPaused => "MARKET_PAUSE",
            EventType::MarketUnpaused => "MARKET_UNPAUSE",
            EventType::ReserveInitialized => "RESERVE_INIT",
            EventType::ReserveConfigUpdated => "RESERVE_CONFIG",
            EventType::LiquidityDeposited => "DEPOSIT",
            EventType::LiquidityWithdrawn => "WITHDRAW",
            EventType::InterestAccrued => "INTEREST",
            EventType::ObligationInitialized => "OBLIGATION_INIT",
            EventType::CollateralDeposited => "COLLATERAL_DEPOSIT",
            EventType::CollateralWithdrawn => "COLLATERAL_WITHDRAW",
            EventType::LiquidityBorrowed => "BORROW",
            EventType::LiquidityRepaid => "REPAY",
            EventType::LiquidationExecuted => "LIQUIDATION",
            EventType::FlashLoanExecuted => "FLASH_LOAN",
            EventType::PriceUpdated => "PRICE_UPDATE",
            EventType::OracleStale => "ORACLE_STALE",
            EventType::PriceManipulationDetected => "PRICE_MANIPULATION",
            EventType::ProposalCreated => "PROPOSAL_CREATE",
            EventType::ProposalSigned => "PROPOSAL_SIGN",
            EventType::ProposalExecuted => "PROPOSAL_EXECUTE",
            EventType::ProposalCancelled => "PROPOSAL_CANCEL",
            EventType::RoleGranted => "ROLE_GRANT",
            EventType::RoleRevoked => "ROLE_REVOKE",
            EventType::EmergencyActionTaken => "EMERGENCY",
            EventType::ReentrancyDetected => "REENTRANCY",
            EventType::UnauthorizedAccess => "UNAUTHORIZED",
            EventType::MathOverflow => "MATH_OVERFLOW",
            EventType::InvalidOperation => "INVALID_OP",
            EventType::ProgramUpgraded => "UPGRADE",
            EventType::AccountMigrated => "MIGRATION",
            EventType::ConfigurationChanged => "CONFIG_CHANGE",
        }
    }

    /// Log info level event
    pub fn info(event_type: EventType, message: &str) -> Result<()> {
        Self::log(
            LogLevel::Info,
            event_type,
            message,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Log warning level event
    pub fn warning(event_type: EventType, message: &str) -> Result<()> {
        Self::log(
            LogLevel::Warning,
            event_type,
            message,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Log error level event
    pub fn error(event_type: EventType, message: &str) -> Result<()> {
        Self::log(
            LogLevel::Error,
            event_type,
            message,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Log critical level event
    pub fn critical(event_type: EventType, message: &str) -> Result<()> {
        Self::log(
            LogLevel::Critical,
            event_type,
            message,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Log market event with context
    pub fn market_event(
        level: LogLevel,
        event_type: EventType,
        message: &str,
        market: Pubkey,
        user: Option<Pubkey>,
    ) -> Result<()> {
        Self::log(
            level,
            event_type,
            message,
            user,
            Some(market),
            None,
            None,
            None,
            None,
        )
    }

    /// Log reserve event with context
    pub fn reserve_event(
        level: LogLevel,
        event_type: EventType,
        message: &str,
        market: Pubkey,
        reserve: Pubkey,
        user: Option<Pubkey>,
        amount: Option<u64>,
    ) -> Result<()> {
        Self::log(
            level,
            event_type,
            message,
            user,
            Some(market),
            Some(reserve),
            None,
            amount,
            None,
        )
    }

    /// Log obligation event with context
    pub fn obligation_event(
        level: LogLevel,
        event_type: EventType,
        message: &str,
        market: Pubkey,
        obligation: Pubkey,
        user: Option<Pubkey>,
        amount: Option<u64>,
    ) -> Result<()> {
        Self::log(
            level,
            event_type,
            message,
            user,
            Some(market),
            None,
            Some(obligation),
            amount,
            None,
        )
    }

    /// Log liquidation event with full context
    pub fn liquidation_event(
        message: &str,
        market: Pubkey,
        obligation: Pubkey,
        liquidator: Pubkey,
        repay_amount: u64,
        collateral_amount: u64,
        reserve_repay: Pubkey,
        reserve_collateral: Pubkey,
    ) -> Result<()> {
        let additional_data = format!(
            "repay_reserve: {}, collateral_reserve: {}, repay_amount: {}, collateral_amount: {}",
            reserve_repay, reserve_collateral, repay_amount, collateral_amount
        );

        Self::log(
            LogLevel::Info,
            EventType::LiquidationExecuted,
            message,
            Some(liquidator),
            Some(market),
            None,
            Some(obligation),
            Some(repay_amount),
            Some(additional_data),
        )
    }

    /// Log security event
    pub fn security_event(
        event_type: EventType,
        message: &str,
        user: Option<Pubkey>,
        additional_context: Option<String>,
    ) -> Result<()> {
        Self::log(
            LogLevel::Critical,
            event_type,
            message,
            user,
            None,
            None,
            None,
            None,
            additional_context,
        )
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor;

impl PerformanceMonitor {
    /// Log instruction execution time
    pub fn log_instruction_time(
        instruction_name: &str,
        start_slot: u64,
        end_slot: u64,
    ) -> Result<()> {
        let duration = end_slot.saturating_sub(start_slot);

        if duration > 5 {
            Logger::warning(
                EventType::InvalidOperation,
                &format!(
                    "Slow instruction: {} took {} slots",
                    instruction_name, duration
                ),
            )?;
        }

        Logger::log(
            LogLevel::Debug,
            EventType::ConfigurationChanged,
            &format!(
                "Instruction {} executed in {} slots",
                instruction_name, duration
            ),
            None,
            None,
            None,
            None,
            Some(duration),
            None,
        )
    }

    /// Log memory usage
    pub fn log_account_size(account_name: &str, size: usize) -> Result<()> {
        if size > 10240 {
            // Warn if account > 10KB
            Logger::warning(
                EventType::ConfigurationChanged,
                &format!("Large account: {} is {} bytes", account_name, size),
            )?;
        }

        Ok(())
    }
}

/// Macros for convenient logging
#[macro_export]
macro_rules! log_info {
    ($event:expr, $msg:expr) => {
        Logger::info($event, $msg)?;
    };
    ($event:expr, $msg:expr, $($arg:expr),*) => {
        Logger::info($event, &format!($msg, $($arg),*))?;
    };
}

#[macro_export]
macro_rules! log_warning {
    ($event:expr, $msg:expr) => {
        Logger::warning($event, $msg)?;
    };
    ($event:expr, $msg:expr, $($arg:expr),*) => {
        Logger::warning($event, &format!($msg, $($arg),*))?;
    };
}

#[macro_export]
macro_rules! log_error {
    ($event:expr, $msg:expr) => {
        Logger::error($event, $msg)?;
    };
    ($event:expr, $msg:expr, $($arg:expr),*) => {
        Logger::error($event, &format!($msg, $($arg),*))?;
    };
}

#[macro_export]
macro_rules! log_security {
    ($event:expr, $msg:expr) => {
        Logger::security_event($event, $msg, None, None)?;
    };
    ($event:expr, $msg:expr, $user:expr) => {
        Logger::security_event($event, $msg, Some($user), None)?;
    };
}
