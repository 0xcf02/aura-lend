use anchor_lang::prelude::*;
use crate::state::*;
use crate::utils::config::*;
use crate::utils::rbac::*;
use crate::error::LendingError;

/// Initialize protocol configuration
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = ProtocolConfig::SIZE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize_config(
    ctx: Context<InitializeConfig>,
    params: ConfigUpdateParams,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let clock = Clock::get()?;
    
    // Initialize with default values
    *config = ProtocolConfig::default();
    config.authority = ctx.accounts.authority.key();
    
    // Apply any custom parameters
    params.apply_to(config);
    
    // Validate and update timestamps
    config.update(&clock)?;
    
    msg!("Protocol configuration initialized by: {}", ctx.accounts.authority.key());
    
    Ok(())
}

/// Update protocol configuration (requires governance approval)
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(
        seeds = [b"governance"],
        bump
    )]
    pub governance: Account<'info, GovernanceRegistry>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = ConfigHistory::SIZE,
        seeds = [b"config_history", config.key().as_ref(), &config.last_updated_slot.to_le_bytes()],
        bump
    )]
    pub config_history: Account<'info, ConfigHistory>,
    
    pub system_program: Program<'info, System>,
}

pub fn update_config(
    ctx: Context<UpdateConfig>,
    params: ConfigUpdateParams,
    timelock_priority: TimelockPriority,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let governance = &ctx.accounts.governance;
    let authority = &ctx.accounts.authority;
    let clock = Clock::get()?;
    
    // Verify authority has appropriate permissions
    let required_permission = match timelock_priority {
        TimelockPriority::Critical => "SUPER_ADMIN",
        TimelockPriority::High => "CONFIG_MANAGER",
        TimelockPriority::Medium => "RISK_MANAGER",
        TimelockPriority::Low => "FEE_MANAGER",
    };
    
    require!(
        governance.has_permission(authority.key(), required_permission)?,
        LendingError::InsufficientPermissions
    );
    
    // Create history record before updating
    let config_history = &mut ctx.accounts.config_history;
    config_history.version = 1;
    config_history.config_address = config.key();
    config_history.updated_by = authority.key();
    config_history.updated_at_slot = clock.slot;
    config_history.updated_at_timestamp = clock.unix_timestamp as u64;
    config_history.changes = Vec::new();
    
    // Track changes for audit
    track_config_changes(config, &params, &mut config_history.changes);
    
    // Apply updates
    params.apply_to(config);
    
    // Validate and update timestamps
    config.update(&clock)?;
    
    msg!("Protocol configuration updated by: {} with priority: {:?}", 
         authority.key(), timelock_priority);
    
    Ok(())
}

/// Emergency configuration update (bypasses timelock)
#[derive(Accounts)]
pub struct EmergencyConfigUpdate<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(
        seeds = [b"governance"],
        bump
    )]
    pub governance: Account<'info, GovernanceRegistry>,
    
    #[account(mut)]
    pub emergency_authority: Signer<'info>,
}

pub fn emergency_config_update(
    ctx: Context<EmergencyConfigUpdate>,
    emergency_params: EmergencyConfigParams,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let governance = &ctx.accounts.governance;
    let authority = &ctx.accounts.emergency_authority;
    let clock = Clock::get()?;
    
    // Verify emergency authority
    require!(
        governance.has_permission(authority.key(), "EMERGENCY_RESPONDER")? || 
        governance.has_permission(authority.key(), "SUPER_ADMIN")?,
        LendingError::InsufficientPermissions
    );
    
    // Apply emergency settings
    config.emergency_mode = emergency_params.emergency_mode;
    config.pause_deposits = emergency_params.pause_deposits;
    config.pause_withdrawals = emergency_params.pause_withdrawals;
    config.pause_borrows = emergency_params.pause_borrows;
    config.pause_liquidations = emergency_params.pause_liquidations;
    
    // Update timestamps
    config.update(&clock)?;
    
    msg!("Emergency configuration update by: {}, emergency_mode: {}", 
         authority.key(), config.emergency_mode);
    
    Ok(())
}

/// Get current configuration
#[derive(Accounts)]
pub struct GetConfig<'info> {
    #[account(
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
}

pub fn get_config(ctx: Context<GetConfig>) -> Result<ProtocolConfig> {
    Ok(*ctx.accounts.config)
}

/// Emergency configuration parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct EmergencyConfigParams {
    pub emergency_mode: bool,
    pub pause_deposits: bool,
    pub pause_withdrawals: bool,
    pub pause_borrows: bool,
    pub pause_liquidations: bool,
}

/// Track configuration changes for audit trail
fn track_config_changes(
    current: &ProtocolConfig,
    params: &ConfigUpdateParams,
    changes: &mut Vec<ConfigChange>,
) {
    if let Some(value) = params.max_reserves {
        if value != current.max_reserves {
            changes.push(ConfigChange {
                parameter: "max_reserves".to_string(),
                old_value: current.max_reserves.to_string(),
                new_value: value.to_string(),
            });
        }
    }
    
    if let Some(value) = params.default_protocol_fee_bps {
        if value != current.default_protocol_fee_bps {
            changes.push(ConfigChange {
                parameter: "default_protocol_fee_bps".to_string(),
                old_value: current.default_protocol_fee_bps.to_string(),
                new_value: value.to_string(),
            });
        }
    }
    
    if let Some(value) = params.emergency_mode {
        if value != current.emergency_mode {
            changes.push(ConfigChange {
                parameter: "emergency_mode".to_string(),
                old_value: current.emergency_mode.to_string(),
                new_value: value.to_string(),
            });
        }
    }
    
    // Add more parameter tracking as needed
    // This is a simplified version - in production, you'd want to track all parameters
}

/// Configuration validation helper
pub fn validate_config_update(
    config: &ProtocolConfig,
    params: &ConfigUpdateParams,
) -> Result<()> {
    // Create a temporary config to validate
    let mut temp_config = *config;
    params.apply_to(&mut temp_config);
    temp_config.validate()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_emergency_config_params() {
        let params = EmergencyConfigParams {
            emergency_mode: true,
            pause_deposits: true,
            pause_withdrawals: false,
            pause_borrows: true,
            pause_liquidations: false,
        };
        
        let mut config = ProtocolConfig::default();
        config.emergency_mode = params.emergency_mode;
        config.pause_deposits = params.pause_deposits;
        config.pause_withdrawals = params.pause_withdrawals;
        config.pause_borrows = params.pause_borrows;
        config.pause_liquidations = params.pause_liquidations;
        
        assert!(config.is_emergency_mode());
        assert!(config.is_deposits_paused());
        assert!(config.is_withdrawals_paused()); // Emergency mode affects withdrawals
        assert!(config.is_borrows_paused());
        assert!(!config.is_liquidations_paused());
    }
}