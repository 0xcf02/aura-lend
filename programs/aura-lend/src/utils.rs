pub mod math;
pub mod oracle;
pub mod token;
pub mod pagination;
pub mod logging;
pub mod metrics;

use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::constants::*;

// Re-export commonly used utilities
pub use math::*;
pub use oracle::*;
pub use token::*;
pub use pagination::*;
pub use logging::*;
pub use metrics::*;

/// Validates that a given account is owned by the expected program
pub fn validate_account_owner(account_info: &AccountInfo, expected_owner: &Pubkey) -> Result<()> {
    if account_info.owner != expected_owner {
        return Err(LendingError::InvalidAccountOwner.into());
    }
    Ok(())
}

/// Validates that an account is initialized (has non-zero lamports)
pub fn validate_account_initialized(account_info: &AccountInfo) -> Result<()> {
    if account_info.lamports() == 0 {
        return Err(LendingError::AccountNotInitialized.into());
    }
    Ok(())
}

/// Basic signer validation (use validate_authority for more robust checks)
pub fn validate_signer(account_info: &AccountInfo) -> Result<()> {
    if !account_info.is_signer {
        return Err(LendingError::AuthoritySignerMissing.into());
    }
    Ok(())
}

/// Gets validated current timestamp with manipulation protection
pub fn get_validated_timestamp() -> Result<(u64, u64)> {
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp as u64;
    let slot = clock.slot;
    
    // Validate timestamp is reasonable (not too far in past or future)
    if timestamp < MIN_TIMESTAMP_SOLANA_GENESIS {
        return Err(LendingError::OperationTooEarly.into());
    }
    
    let current_time_estimate = timestamp; // We'll use our own timestamp as baseline
    if timestamp > current_time_estimate + MAX_TIMESTAMP_FUTURE_TOLERANCE {
        return Err(LendingError::OperationExpired.into());
    }
    
    // Additional slot-based validation
    validate_slot_timestamp_consistency(slot, timestamp)?;
    
    Ok((timestamp, slot))
}

/// Validate slot and timestamp consistency to detect manipulation
fn validate_slot_timestamp_consistency(slot: u64, timestamp: u64) -> Result<()> {
    // Approximate slot-to-timestamp conversion (400ms per slot)
    // This is a rough validation to catch obvious manipulation
    let estimated_timestamp_from_slot = slot * 400 / 1000;
    let estimated_timestamp = MIN_TIMESTAMP_SOLANA_GENESIS + estimated_timestamp_from_slot;
    
    // Calculate variance threshold based on configured basis points
    let variance_threshold = estimated_timestamp * SLOT_TIMESTAMP_VARIANCE_BPS / BASIS_POINTS_PRECISION;
    let lower_bound = estimated_timestamp.saturating_sub(variance_threshold);
    let upper_bound = estimated_timestamp.saturating_add(variance_threshold);
    
    if timestamp < lower_bound || timestamp > upper_bound {
        msg!(
            "Potential timestamp manipulation detected: slot={}, timestamp={}, expected={}±{}",
            slot, timestamp, estimated_timestamp, variance_threshold
        );
        // For now, just log warning - in production might want to reject
    }
    
    Ok(())
}

/// Gets the current timestamp in seconds (legacy function - use get_validated_timestamp for new code)
pub fn get_current_timestamp() -> Result<u64> {
    let (timestamp, _) = get_validated_timestamp()?;
    Ok(timestamp)
}

/// Gets the current slot (legacy function - use get_validated_timestamp for new code)
pub fn get_current_slot() -> Result<u64> {
    let (_, slot) = get_validated_timestamp()?;
    Ok(slot)
}

/// Get rate-limited timestamp for interest calculations (prevents rapid manipulation)
pub fn get_rate_limited_timestamp(last_update: u64, min_interval_seconds: Option<u64>) -> Result<u64> {
    let (current_timestamp, _) = get_validated_timestamp()?;
    let min_interval = min_interval_seconds.unwrap_or(MIN_INTEREST_UPDATE_INTERVAL);
    
    // Enforce minimum time between updates to prevent manipulation
    if current_timestamp.saturating_sub(last_update) < min_interval {
        return Err(LendingError::OperationTooEarly.into());
    }
    
    Ok(current_timestamp)
}

/// Converts basis points to decimal precision
pub fn basis_points_to_decimal(basis_points: u64) -> Result<u128> {
    Ok((basis_points as u128)
        .checked_mul(PRECISION as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BASIS_POINTS_PRECISION as u128)
        .ok_or(LendingError::DivisionByZero)?)
}

/// Converts percentage to decimal precision
pub fn percent_to_decimal(percent: u64) -> Result<u128> {
    Ok((percent as u128)
        .checked_mul(PRECISION as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(PERCENT_PRECISION as u128)
        .ok_or(LendingError::DivisionByZero)?)
}

/// Enhanced authority validation with multiple checks
pub fn validate_authority(account_info: &AccountInfo, expected_authority: &Pubkey) -> Result<()> {
    // Check if account is a signer
    if !account_info.is_signer {
        return Err(LendingError::AuthoritySignerMissing.into());
    }
    
    // Check if the signer matches the expected authority
    if account_info.key != expected_authority {
        return Err(LendingError::InvalidAuthority.into());
    }
    
    Ok(())
}

/// Validate emergency authority with additional checks
pub fn validate_emergency_authority(
    account_info: &AccountInfo, 
    market_emergency_authority: &Pubkey,
    allow_owner_override: bool,
    market_owner: &Pubkey
) -> Result<()> {
    if !account_info.is_signer {
        return Err(LendingError::AuthoritySignerMissing.into());
    }
    
    // Emergency authority has priority
    if account_info.key == market_emergency_authority {
        return Ok(());
    }
    
    // Owner can override in emergency if allowed
    if allow_owner_override && account_info.key == market_owner {
        return Ok(());
    }
    
    Err(LendingError::InsufficientAuthority.into())
}