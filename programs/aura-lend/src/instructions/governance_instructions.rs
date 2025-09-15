use crate::constants::*;
use crate::error::LendingError;
use crate::state::governance::*;
use crate::state::multisig::*;
use anchor_lang::prelude::*;

/// Initialize governance registry
pub fn initialize_governance(
    ctx: Context<InitializeGovernance>,
    params: InitializeGovernanceParams,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;

    // Initialize the governance registry
    **governance = GovernanceRegistry::new(params.multisig)?;

    msg!(
        "Governance registry initialized for multisig: {}",
        params.multisig
    );
    Ok(())
}

/// Grant a role to an account
pub fn grant_role(ctx: Context<GrantRole>, params: GrantRoleParams) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let granter = &ctx.accounts.granter;
    let multisig_proposal = &ctx.accounts.multisig_proposal;

    // Verify this is being called through an executed multisig proposal
    if multisig_proposal.status != crate::state::multisig::ProposalStatus::Executed {
        return Err(LendingError::ProposalNotExecuted.into());
    }

    // Verify proposal is for granting a role
    if multisig_proposal.operation_type
        != crate::state::multisig::MultisigOperationType::UpdateMultisigConfig
    {
        return Err(LendingError::InvalidOperationType.into());
    }

    // Get the default permissions for the role type
    let role_permissions = match params.role_type {
        RoleType::SuperAdmin => Permission::SUPER_ADMIN.bits(),
        RoleType::ReserveManager => Permission::RESERVE_MANAGER.bits(),
        RoleType::RiskManager => Permission::RISK_MANAGER.bits(),
        RoleType::OracleManager => Permission::ORACLE_MANAGER.bits(),
        RoleType::EmergencyResponder => Permission::EMERGENCY_RESPONDER.bits(),
        RoleType::FeeManager => Permission::FEE_MANAGER.bits(),
        RoleType::GovernanceManager => Permission::GOVERNANCE_MANAGER.bits(),
        RoleType::TimelockManager => Permission::TIMELOCK_MANAGER.bits(),
        RoleType::ProgramUpgradeManager => Permission::PROGRAM_UPGRADE_MANAGER.bits(),
        RoleType::DataMigrationManager => Permission::DATA_MIGRATION_MANAGER.bits(),
    };

    // Use provided permissions or default to role permissions
    let final_permissions = if params.permissions == 0 {
        role_permissions
    } else {
        params.permissions
    };

    // Grant the role
    governance.grant_role(
        params.holder,
        params.role_type,
        final_permissions,
        params.expires_at,
        granter.key(),
    )?;

    msg!(
        "Role {:?} granted to {} by {}",
        params.role_type,
        params.holder,
        granter.key()
    );
    Ok(())
}

/// Revoke a role from an account
pub fn revoke_role(ctx: Context<RevokeRole>, target_holder: Pubkey) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let revoker = &ctx.accounts.revoker;
    let multisig_proposal = &ctx.accounts.multisig_proposal;

    // Verify this is being called through an executed multisig proposal
    if multisig_proposal.status != crate::state::multisig::ProposalStatus::Executed {
        return Err(LendingError::ProposalNotExecuted.into());
    }

    // Verify proposal is for revoking a role
    if multisig_proposal.operation_type
        != crate::state::multisig::MultisigOperationType::UpdateMultisigConfig
    {
        return Err(LendingError::InvalidOperationType.into());
    }

    // Revoke the role
    governance.revoke_role(&target_holder)?;

    msg!("Role revoked from {} by {}", target_holder, revoker.key());
    Ok(())
}

/// Delegate specific permissions to an account (temporary)
pub fn delegate_permissions(
    ctx: Context<DelegatePermissions>,
    params: DelegatePermissionsParams,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let delegator = &ctx.accounts.delegator;

    // Check if delegator has governance management permissions
    PermissionChecker::check_permission(
        governance,
        &delegator.key(),
        Permission::GOVERNANCE_MANAGER,
    )?;

    // Check if delegator has the permissions they want to delegate
    if let Some(delegator_role) = governance.get_active_role(&delegator.key()) {
        if (delegator_role.permissions & params.permissions) != params.permissions {
            return Err(LendingError::CannotDelegatePermissionsNotHeld.into());
        }
    } else {
        return Err(LendingError::RoleNotFound.into());
    }

    // Create a temporary role with delegated permissions
    governance.grant_role(
        params.delegate,
        RoleType::GovernanceManager, // Temporary delegation role
        params.permissions,
        Some(params.expires_at),
        delegator.key(),
    )?;

    msg!(
        "Permissions delegated to {} by {} until {}",
        params.delegate,
        delegator.key(),
        params.expires_at
    );
    Ok(())
}

/// Clean up expired roles
pub fn cleanup_expired_roles(ctx: Context<CleanupExpiredRoles>) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let executor = &ctx.accounts.executor;

    // Check permission (anyone with governance management can cleanup)
    PermissionChecker::check_permission(
        governance,
        &executor.key(),
        Permission::GOVERNANCE_MANAGER,
    )?;

    // Clean up expired roles
    let removed_count = governance.cleanup_expired_roles()?;

    msg!("Cleaned up {} expired roles", removed_count);
    Ok(())
}

/// Update governance configuration
pub fn update_governance_config(
    ctx: Context<UpdateGovernanceConfig>,
    new_available_permissions: u64,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let multisig_proposal = &ctx.accounts.multisig_proposal;

    // Verify this is being called through an executed multisig proposal
    if multisig_proposal.status != crate::state::multisig::ProposalStatus::Executed {
        return Err(LendingError::ProposalNotExecuted.into());
    }

    // Update available permissions
    governance.available_permissions = new_available_permissions;

    msg!("Governance configuration updated");
    Ok(())
}

/// Emergency role grant (bypass normal multisig process in extreme situations)
pub fn emergency_grant_role(
    ctx: Context<EmergencyGrantRole>,
    params: GrantRoleParams,
) -> Result<()> {
    let governance = &mut ctx.accounts.governance;
    let emergency_authority = &ctx.accounts.emergency_authority;
    let market = &ctx.accounts.market;

    // Verify caller is the emergency authority
    if emergency_authority.key() != market.emergency_authority {
        return Err(LendingError::InvalidAuthority.into());
    }

    // Emergency roles are limited and temporary
    if params.expires_at.is_none() {
        return Err(LendingError::EmergencyRoleMustHaveExpiration.into());
    }

    let clock = Clock::get()?;
    let max_emergency_duration = clock.unix_timestamp + EMERGENCY_ROLE_MAX_DURATION;

    let expires_at = params
        .expires_at
        .ok_or(LendingError::EmergencyRoleMustHaveExpiration)?;

    if expires_at > max_emergency_duration {
        return Err(LendingError::EmergencyRoleTooLong.into());
    }

    // Only allow emergency responder or limited permissions
    let allowed_permissions = Permission::EMERGENCY_RESPONDER.bits()
        | Permission::ORACLE_MANAGER.bits()
        | Permission::TIMELOCK_MANAGER.bits();

    if (params.permissions & !allowed_permissions) != 0 {
        return Err(LendingError::InvalidEmergencyPermissions.into());
    }

    // Grant emergency role
    governance.grant_role(
        params.holder,
        params.role_type,
        params.permissions,
        params.expires_at,
        emergency_authority.key(),
    )?;

    msg!(
        "Emergency role granted to {} by emergency authority",
        params.holder
    );
    Ok(())
}

// Account validation structs

#[derive(Accounts)]
#[instruction(params: InitializeGovernanceParams)]
pub struct InitializeGovernance<'info> {
    #[account(
        init,
        payer = payer,
        space = GovernanceRegistry::SIZE,
        seeds = [GOVERNANCE_SEED, params.multisig.as_ref()],
        bump
    )]
    pub governance: Account<'info, GovernanceRegistry>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(params: GrantRoleParams)]
pub struct GrantRole<'info> {
    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    /// The executed multisig proposal that authorizes this grant
    pub multisig_proposal: Account<'info, crate::state::multisig::MultisigProposal>,

    pub granter: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(target_holder: Pubkey)]
pub struct RevokeRole<'info> {
    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    /// The executed multisig proposal that authorizes this revocation
    pub multisig_proposal: Account<'info, crate::state::multisig::MultisigProposal>,

    pub revoker: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(params: DelegatePermissionsParams)]
pub struct DelegatePermissions<'info> {
    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    pub delegator: Signer<'info>,
}

#[derive(Accounts)]
pub struct CleanupExpiredRoles<'info> {
    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    pub executor: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(new_available_permissions: u64)]
pub struct UpdateGovernanceConfig<'info> {
    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    /// The executed multisig proposal that authorizes this update
    pub multisig_proposal: Account<'info, crate::state::multisig::MultisigProposal>,

    pub executor: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(params: GrantRoleParams)]
pub struct EmergencyGrantRole<'info> {
    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    pub market: Account<'info, crate::state::market::Market>,

    pub emergency_authority: Signer<'info>,
}

// Parameter structs for governance operations

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DelegatePermissionsParams {
    pub delegate: Pubkey,
    pub permissions: u64,
    pub expires_at: i64,
}
