use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::constants::*;

/// Dynamic configuration management for the protocol
#[account]
pub struct ProtocolConfig {
    pub version: u8,
    pub authority: Pubkey,
    pub last_updated_slot: u64,
    pub last_updated_timestamp: u64,
    
    // Market limits
    pub max_reserves: u64,
    pub max_obligations: u64,
    pub max_obligation_reserves: u64,
    
    // Economic parameters
    pub default_protocol_fee_bps: u64,
    pub max_protocol_fee_bps: u64,
    pub liquidation_close_factor_bps: u64,
    pub max_liquidation_bonus_bps: u64,
    
    // Risk parameters
    pub min_health_factor: u64,
    pub max_ltv_ratio: u64,
    pub min_liquidation_threshold: u64,
    
    // Oracle settings
    pub max_oracle_staleness_slots: u64,
    pub max_oracle_confidence_threshold: u64,
    pub min_oracle_sources: u8,
    
    // Governance settings
    pub max_multisig_signatories: u8,
    pub min_multisig_threshold: u8,
    pub max_governance_roles: u64,
    pub default_timelock_delay: u64,
    
    // Performance settings
    pub compute_unit_limit: u32,
    pub max_accounts_per_instruction: u8,
    pub pagination_default_limit: u64,
    pub pagination_max_limit: u64,
    
    // Emergency settings
    pub emergency_mode: bool,
    pub pause_deposits: bool,
    pub pause_withdrawals: bool,
    pub pause_borrows: bool,
    pub pause_liquidations: bool,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            version: 1,
            authority: Pubkey::default(),
            last_updated_slot: 0,
            last_updated_timestamp: 0,
            
            // Market limits
            max_reserves: MAX_RESERVES,
            max_obligations: MAX_OBLIGATIONS,
            max_obligation_reserves: MAX_OBLIGATION_RESERVES,
            
            // Economic parameters
            default_protocol_fee_bps: DEFAULT_PROTOCOL_FEE,
            max_protocol_fee_bps: MAX_PROTOCOL_FEE,
            liquidation_close_factor_bps: LIQUIDATION_CLOSE_FACTOR,
            max_liquidation_bonus_bps: MAX_LIQUIDATION_BONUS,
            
            // Risk parameters
            min_health_factor: MIN_HEALTH_FACTOR,
            max_ltv_ratio: MAX_LTV_RATIO,
            min_liquidation_threshold: MIN_LIQUIDATION_THRESHOLD,
            
            // Oracle settings
            max_oracle_staleness_slots: ORACLE_STALENESS_THRESHOLD,
            max_oracle_confidence_threshold: ORACLE_CONFIDENCE_THRESHOLD,
            min_oracle_sources: MIN_ORACLE_SOURCES,
            
            // Governance settings
            max_multisig_signatories: MAX_MULTISIG_SIGNATORIES,
            min_multisig_threshold: MIN_MULTISIG_THRESHOLD,
            max_governance_roles: MAX_GOVERNANCE_ROLES,
            default_timelock_delay: DEFAULT_TIMELOCK_DELAY,
            
            // Performance settings
            compute_unit_limit: COMPUTE_UNIT_LIMIT,
            max_accounts_per_instruction: MAX_ACCOUNTS_PER_INSTRUCTION,
            pagination_default_limit: PAGINATION_DEFAULT_LIMIT,
            pagination_max_limit: PAGINATION_MAX_LIMIT,
            
            // Emergency settings
            emergency_mode: false,
            pause_deposits: false,
            pause_withdrawals: false,
            pause_borrows: false,
            pause_liquidations: false,
        }
    }
}

impl ProtocolConfig {
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // authority
        8 + // last_updated_slot
        8 + // last_updated_timestamp
        8 + // max_reserves
        8 + // max_obligations
        8 + // max_obligation_reserves
        8 + // default_protocol_fee_bps
        8 + // max_protocol_fee_bps
        8 + // liquidation_close_factor_bps
        8 + // max_liquidation_bonus_bps
        8 + // min_health_factor
        8 + // max_ltv_ratio
        8 + // min_liquidation_threshold
        8 + // max_oracle_staleness_slots
        8 + // max_oracle_confidence_threshold
        1 + // min_oracle_sources
        1 + // max_multisig_signatories
        1 + // min_multisig_threshold
        8 + // max_governance_roles
        8 + // default_timelock_delay
        4 + // compute_unit_limit
        1 + // max_accounts_per_instruction
        8 + // pagination_default_limit
        8 + // pagination_max_limit
        1 + // emergency_mode
        1 + // pause_deposits
        1 + // pause_withdrawals
        1 + // pause_borrows
        1 + // pause_liquidations
        64; // padding
    
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        // Market limits validation
        require!(self.max_reserves > 0 && self.max_reserves <= 1000, LendingError::InvalidConfiguration);
        require!(self.max_obligations > 0 && self.max_obligations <= 100_000, LendingError::InvalidConfiguration);
        require!(self.max_obligation_reserves > 0 && self.max_obligation_reserves <= 32, LendingError::InvalidConfiguration);
        
        // Economic parameters validation
        require!(self.default_protocol_fee_bps <= BASIS_POINTS_PRECISION, LendingError::InvalidConfiguration);
        require!(self.max_protocol_fee_bps <= BASIS_POINTS_PRECISION, LendingError::InvalidConfiguration);
        require!(self.liquidation_close_factor_bps > 0 && self.liquidation_close_factor_bps <= BASIS_POINTS_PRECISION, LendingError::InvalidConfiguration);
        require!(self.max_liquidation_bonus_bps <= 2000, LendingError::InvalidConfiguration); // Max 20%
        
        // Risk parameters validation
        require!(self.min_health_factor >= PRECISION, LendingError::InvalidConfiguration); // At least 1.0
        require!(self.max_ltv_ratio > 0 && self.max_ltv_ratio <= 9000, LendingError::InvalidConfiguration); // Max 90%
        require!(self.min_liquidation_threshold >= self.max_ltv_ratio, LendingError::InvalidConfiguration);
        
        // Oracle settings validation
        require!(self.max_oracle_staleness_slots > 0 && self.max_oracle_staleness_slots <= 14400, LendingError::InvalidConfiguration); // Max 2 hours
        require!(self.max_oracle_confidence_threshold <= 10000, LendingError::InvalidConfiguration); // Max 100%
        require!(self.min_oracle_sources > 0 && self.min_oracle_sources <= 10, LendingError::InvalidConfiguration);
        
        // Governance settings validation
        require!(self.max_multisig_signatories >= 2 && self.max_multisig_signatories <= 50, LendingError::InvalidConfiguration);
        require!(self.min_multisig_threshold >= 1 && self.min_multisig_threshold <= self.max_multisig_signatories, LendingError::InvalidConfiguration);
        require!(self.max_governance_roles > 0 && self.max_governance_roles <= 1000, LendingError::InvalidConfiguration);
        require!(self.default_timelock_delay >= 3600, LendingError::InvalidConfiguration); // Min 1 hour
        
        // Performance settings validation
        require!(self.compute_unit_limit >= 200_000 && self.compute_unit_limit <= 1_400_000, LendingError::InvalidConfiguration);
        require!(self.max_accounts_per_instruction > 0 && self.max_accounts_per_instruction <= 64, LendingError::InvalidConfiguration);
        require!(self.pagination_default_limit > 0 && self.pagination_default_limit <= self.pagination_max_limit, LendingError::InvalidConfiguration);
        require!(self.pagination_max_limit > 0 && self.pagination_max_limit <= 1000, LendingError::InvalidConfiguration);
        
        Ok(())
    }
    
    /// Update configuration with new values
    pub fn update(&mut self, clock: &Clock) -> Result<()> {
        self.last_updated_slot = clock.slot;
        self.last_updated_timestamp = clock.unix_timestamp as u64;
        self.validate()
    }
    
    /// Check if protocol is in emergency mode
    pub fn is_emergency_mode(&self) -> bool {
        self.emergency_mode
    }
    
    /// Check if specific operations are paused
    pub fn is_deposits_paused(&self) -> bool {
        self.emergency_mode || self.pause_deposits
    }
    
    pub fn is_withdrawals_paused(&self) -> bool {
        self.emergency_mode || self.pause_withdrawals
    }
    
    pub fn is_borrows_paused(&self) -> bool {
        self.emergency_mode || self.pause_borrows
    }
    
    pub fn is_liquidations_paused(&self) -> bool {
        self.pause_liquidations // Note: liquidations should remain active even in emergency
    }
    
    /// Get effective protocol fee for a reserve
    pub fn get_protocol_fee_bps(&self, reserve_fee_bps: Option<u64>) -> u64 {
        reserve_fee_bps.unwrap_or(self.default_protocol_fee_bps).min(self.max_protocol_fee_bps)
    }
    
    /// Calculate timelock delay based on operation priority
    pub fn get_timelock_delay(&self, priority: TimelockPriority) -> u64 {
        match priority {
            TimelockPriority::Critical => self.default_timelock_delay * 7, // 7x for critical
            TimelockPriority::High => self.default_timelock_delay * 3,     // 3x for high
            TimelockPriority::Medium => self.default_timelock_delay,       // 1x for medium
            TimelockPriority::Low => self.default_timelock_delay / 4,      // 0.25x for low
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub enum TimelockPriority {
    Critical, // Major protocol changes, emergency actions
    High,     // Reserve parameter updates, fee changes
    Medium,   // Minor parameter adjustments
    Low,      // Routine maintenance operations
}

/// Configuration update parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConfigUpdateParams {
    // Market limits
    pub max_reserves: Option<u64>,
    pub max_obligations: Option<u64>,
    pub max_obligation_reserves: Option<u64>,
    
    // Economic parameters
    pub default_protocol_fee_bps: Option<u64>,
    pub max_protocol_fee_bps: Option<u64>,
    pub liquidation_close_factor_bps: Option<u64>,
    pub max_liquidation_bonus_bps: Option<u64>,
    
    // Risk parameters
    pub min_health_factor: Option<u64>,
    pub max_ltv_ratio: Option<u64>,
    pub min_liquidation_threshold: Option<u64>,
    
    // Oracle settings
    pub max_oracle_staleness_slots: Option<u64>,
    pub max_oracle_confidence_threshold: Option<u64>,
    pub min_oracle_sources: Option<u8>,
    
    // Governance settings
    pub max_multisig_signatories: Option<u8>,
    pub min_multisig_threshold: Option<u8>,
    pub max_governance_roles: Option<u64>,
    pub default_timelock_delay: Option<u64>,
    
    // Performance settings
    pub compute_unit_limit: Option<u32>,
    pub max_accounts_per_instruction: Option<u8>,
    pub pagination_default_limit: Option<u64>,
    pub pagination_max_limit: Option<u64>,
    
    // Emergency settings
    pub emergency_mode: Option<bool>,
    pub pause_deposits: Option<bool>,
    pub pause_withdrawals: Option<bool>,
    pub pause_borrows: Option<bool>,
    pub pause_liquidations: Option<bool>,
}

impl ConfigUpdateParams {
    /// Apply updates to existing configuration
    pub fn apply_to(&self, config: &mut ProtocolConfig) {
        // Market limits
        if let Some(value) = self.max_reserves { config.max_reserves = value; }
        if let Some(value) = self.max_obligations { config.max_obligations = value; }
        if let Some(value) = self.max_obligation_reserves { config.max_obligation_reserves = value; }
        
        // Economic parameters
        if let Some(value) = self.default_protocol_fee_bps { config.default_protocol_fee_bps = value; }
        if let Some(value) = self.max_protocol_fee_bps { config.max_protocol_fee_bps = value; }
        if let Some(value) = self.liquidation_close_factor_bps { config.liquidation_close_factor_bps = value; }
        if let Some(value) = self.max_liquidation_bonus_bps { config.max_liquidation_bonus_bps = value; }
        
        // Risk parameters
        if let Some(value) = self.min_health_factor { config.min_health_factor = value; }
        if let Some(value) = self.max_ltv_ratio { config.max_ltv_ratio = value; }
        if let Some(value) = self.min_liquidation_threshold { config.min_liquidation_threshold = value; }
        
        // Oracle settings
        if let Some(value) = self.max_oracle_staleness_slots { config.max_oracle_staleness_slots = value; }
        if let Some(value) = self.max_oracle_confidence_threshold { config.max_oracle_confidence_threshold = value; }
        if let Some(value) = self.min_oracle_sources { config.min_oracle_sources = value; }
        
        // Governance settings
        if let Some(value) = self.max_multisig_signatories { config.max_multisig_signatories = value; }
        if let Some(value) = self.min_multisig_threshold { config.min_multisig_threshold = value; }
        if let Some(value) = self.max_governance_roles { config.max_governance_roles = value; }
        if let Some(value) = self.default_timelock_delay { config.default_timelock_delay = value; }
        
        // Performance settings
        if let Some(value) = self.compute_unit_limit { config.compute_unit_limit = value; }
        if let Some(value) = self.max_accounts_per_instruction { config.max_accounts_per_instruction = value; }
        if let Some(value) = self.pagination_default_limit { config.pagination_default_limit = value; }
        if let Some(value) = self.pagination_max_limit { config.pagination_max_limit = value; }
        
        // Emergency settings
        if let Some(value) = self.emergency_mode { config.emergency_mode = value; }
        if let Some(value) = self.pause_deposits { config.pause_deposits = value; }
        if let Some(value) = self.pause_withdrawals { config.pause_withdrawals = value; }
        if let Some(value) = self.pause_borrows { config.pause_borrows = value; }
        if let Some(value) = self.pause_liquidations { config.pause_liquidations = value; }
    }
}

/// Configuration history for audit trail
#[account]
pub struct ConfigHistory {
    pub version: u8,
    pub config_address: Pubkey,
    pub updated_by: Pubkey,
    pub updated_at_slot: u64,
    pub updated_at_timestamp: u64,
    pub changes: Vec<ConfigChange>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConfigChange {
    pub parameter: String,
    pub old_value: String,
    pub new_value: String,
}

impl ConfigHistory {
    pub const MAX_CHANGES: usize = 50;
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // config_address
        32 + // updated_by
        8 + // updated_at_slot
        8 + // updated_at_timestamp
        4 + (Self::MAX_CHANGES * (4 + 64 + 32 + 32)) + // changes vector
        64; // padding
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_validation() {
        let config = ProtocolConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_update_params() {
        let mut config = ProtocolConfig::default();
        let params = ConfigUpdateParams {
            max_reserves: Some(256),
            default_protocol_fee_bps: Some(150),
            emergency_mode: Some(true),
            ..Default::default()
        };
        
        params.apply_to(&mut config);
        
        assert_eq!(config.max_reserves, 256);
        assert_eq!(config.default_protocol_fee_bps, 150);
        assert!(config.emergency_mode);
    }
    
    #[test]
    fn test_timelock_delay_calculation() {
        let config = ProtocolConfig {
            default_timelock_delay: 3600, // 1 hour
            ..Default::default()
        };
        
        assert_eq!(config.get_timelock_delay(TimelockPriority::Critical), 25200); // 7 hours
        assert_eq!(config.get_timelock_delay(TimelockPriority::High), 10800);     // 3 hours
        assert_eq!(config.get_timelock_delay(TimelockPriority::Medium), 3600);    // 1 hour
        assert_eq!(config.get_timelock_delay(TimelockPriority::Low), 900);        // 15 minutes
    }
}