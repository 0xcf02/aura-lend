pub mod math;
pub mod math_optimized;
pub mod oracle;
pub mod pagination;
pub mod pagination_optimized;
pub mod iterator_optimized;
pub mod memory_optimized;
pub mod logging;
pub mod metrics;
pub mod config;
pub mod token;
pub mod rbac;

use anchor_lang::prelude::*;

pub use math::*;
pub use math_optimized::*;
pub use oracle::*;
pub use pagination::*;
pub use pagination_optimized::*;
pub use iterator_optimized::*;
pub use memory_optimized::*;
pub use logging::*;
pub use metrics::*;
pub use config::*;
pub use token::*;
pub use rbac::*;

/// Validates that the provided account is a signer
pub fn validate_signer(account_info: &AccountInfo) -> Result<()> {
    if !account_info.is_signer {
        return Err(error!(crate::error::LendingError::UnauthorizedSigner));
    }
    Ok(())
}

/// Validates that the provided account has the correct authority
pub fn validate_authority(
    account_info: &AccountInfo,
    expected_authority: &Pubkey,
) -> Result<()> {
    if account_info.key() != expected_authority {
        return Err(error!(crate::error::LendingError::InvalidAuthority));
    }
    validate_signer(account_info)
}

/// Gets validated timestamp for logging purposes
pub fn get_validated_timestamp() -> i64 {
    Clock::get().unwrap().unix_timestamp
}