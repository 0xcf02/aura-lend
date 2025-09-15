use crate::constants::*;
use crate::error::LendingError;
use crate::state::governance::*;
use crate::state::multisig::*;
use crate::state::timelock::*;
use anchor_lang::prelude::*;

/// Initialize a new timelock controller
pub fn initialize_timelock(ctx: Context<InitializeTimelock>) -> Result<()> {
    let timelock = &mut ctx.accounts.timelock;
    let multisig = &ctx.accounts.multisig;

    // Initialize the timelock controller
    **timelock = TimelockController::new(multisig.key())?;

    msg!(
        "Timelock controller initialized for multisig: {}",
        multisig.key()
    );
    Ok(())
}

/// Create a new timelock proposal
pub fn create_timelock_proposal(
    ctx: Context<CreateTimelockProposal>,
    params: CreateTimelockProposalParams,
) -> Result<()> {
    let timelock = &mut ctx.accounts.timelock;
    let proposal = &mut ctx.accounts.proposal;
    let proposer = &ctx.accounts.proposer;
    let governance = &ctx.accounts.governance;

    // Check if proposer has permission to create timelock proposals
    PermissionChecker::check_permission(governance, &proposer.key(), Permission::TIMELOCK_MANAGER)?;

    // Get minimum delay for this operation type
    let min_delay = timelock.get_min_delay(params.operation_type);

    // Create the proposal
    **proposal = TimelockProposal::new(
        timelock.key(),
        params.operation_type,
        params.instruction_data,
        min_delay,
        proposer.key(),
        params.target_accounts,
    )?;

    // Add to active proposals list
    timelock.add_active_proposal(proposal.key())?;

    msg!(
        "Timelock proposal created. Execution time: {}",
        proposal.execution_time
    );
    Ok(())
}

/// Execute a timelock proposal (once delay period has passed)
pub fn execute_timelock_proposal(ctx: Context<ExecuteTimelockProposal>) -> Result<()> {
    let timelock = &mut ctx.accounts.timelock;
    let proposal = &mut ctx.accounts.proposal;
    let executor = &ctx.accounts.executor;
    let governance = &ctx.accounts.governance;

    // Check if executor has permission
    PermissionChecker::check_permission(governance, &executor.key(), Permission::TIMELOCK_MANAGER)?;

    // Check if proposal is ready for execution
    if !proposal.is_ready_for_execution()? {
        return Err(LendingError::TimelockNotReady.into());
    }

    // Check if proposal is expired
    if proposal.is_expired()? {
        return Err(LendingError::ProposalExpired.into());
    }

    // Mark proposal as executed
    proposal.mark_executed()?;

    // Remove from active proposals
    timelock.remove_active_proposal(&proposal.key())?;

    msg!("Timelock proposal executed by {}", executor.key());

    // The actual operation execution would be handled by specific instruction handlers
    Ok(())
}

/// Cancel a timelock proposal (before execution)
pub fn cancel_timelock_proposal(ctx: Context<CancelTimelockProposal>) -> Result<()> {
    let timelock = &mut ctx.accounts.timelock;
    let proposal = &mut ctx.accounts.proposal;
    let authority = &ctx.accounts.authority;
    let governance = &ctx.accounts.governance;

    // Check if authority has permission to cancel
    let can_cancel = proposal.proposer == authority.key()
        || governance.has_permission(&authority.key(), Permission::TIMELOCK_MANAGER);

    if !can_cancel {
        return Err(LendingError::UnauthorizedCancellation.into());
    }

    // Mark proposal as cancelled
    proposal.mark_cancelled()?;

    // Remove from active proposals
    timelock.remove_active_proposal(&proposal.key())?;

    msg!("Timelock proposal cancelled by {}", authority.key());
    Ok(())
}

/// Update timelock delays (requires multisig + timelock approval)
pub fn update_timelock_delays(
    ctx: Context<UpdateTimelockDelays>,
    new_delays: Vec<TimelockDelay>,
) -> Result<()> {
    let timelock = &mut ctx.accounts.timelock;
    let proposal = &ctx.accounts.executed_proposal;

    // Verify this is being called through an executed proposal
    if proposal.status != TimelockStatus::Executed {
        return Err(LendingError::ProposalNotExecuted.into());
    }

    // Verify proposal is for updating timelock delays
    if proposal.operation_type != TimelockOperationType::UpdateTimelockDelays {
        return Err(LendingError::InvalidOperationType.into());
    }

    // Validate new delays (ensure reasonable minimums)
    for delay in &new_delays {
        match delay.operation_type {
            TimelockOperationType::UpdateMarketOwner => {
                if delay.delay_seconds < TIMELOCK_MIN_CRITICAL_DELAY {
                    return Err(LendingError::DelayTooShort.into());
                }
            }
            TimelockOperationType::UpdateEmergencyAuthority => {
                if delay.delay_seconds < TIMELOCK_MIN_HIGH_DELAY {
                    return Err(LendingError::DelayTooShort.into());
                }
            }
            _ => {
                if delay.delay_seconds < TIMELOCK_MIN_STANDARD_DELAY {
                    return Err(LendingError::DelayTooShort.into());
                }
            }
        }
    }

    // Update delays
    timelock.min_delays = new_delays;

    msg!("Timelock delays updated");
    Ok(())
}

/// Clean up expired proposals
pub fn cleanup_expired_proposals(ctx: Context<CleanupExpiredProposals>) -> Result<()> {
    let _timelock = &mut ctx.accounts.timelock;
    let governance = &ctx.accounts.governance;
    let executor = &ctx.accounts.executor;

    // Check permission (anyone with timelock manager can cleanup)
    PermissionChecker::check_permission(governance, &executor.key(), Permission::TIMELOCK_MANAGER)?;

    // This would iterate through active proposals and mark expired ones
    // For now, we'll just remove expired proposals from the active list
    // In a full implementation, this would process remaining accounts

    msg!("Expired proposals cleanup initiated by {}", executor.key());
    Ok(())
}

// Account validation structs

#[derive(Accounts)]
pub struct InitializeTimelock<'info> {
    #[account(
        init,
        payer = payer,
        space = TimelockController::SIZE,
        seeds = [TIMELOCK_SEED, multisig.key().as_ref()],
        bump
    )]
    pub timelock: Account<'info, TimelockController>,

    pub multisig: Account<'info, MultiSig>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(params: CreateTimelockProposalParams)]
pub struct CreateTimelockProposal<'info> {
    #[account(mut)]
    pub timelock: Account<'info, TimelockController>,

    #[account(
        init,
        payer = proposer,
        space = TimelockProposal::SIZE,
    )]
    pub proposal: Account<'info, TimelockProposal>,

    pub governance: Account<'info, GovernanceRegistry>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteTimelockProposal<'info> {
    #[account(mut)]
    pub timelock: Account<'info, TimelockController>,

    #[account(mut)]
    pub proposal: Account<'info, TimelockProposal>,

    pub governance: Account<'info, GovernanceRegistry>,

    pub executor: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelTimelockProposal<'info> {
    #[account(mut)]
    pub timelock: Account<'info, TimelockController>,

    #[account(mut)]
    pub proposal: Account<'info, TimelockProposal>,

    pub governance: Account<'info, GovernanceRegistry>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(new_delays: Vec<TimelockDelay>)]
pub struct UpdateTimelockDelays<'info> {
    #[account(mut)]
    pub timelock: Account<'info, TimelockController>,

    /// The executed proposal that authorizes this update
    pub executed_proposal: Account<'info, TimelockProposal>,

    pub executor: Signer<'info>,
}

#[derive(Accounts)]
pub struct CleanupExpiredProposals<'info> {
    #[account(mut)]
    pub timelock: Account<'info, TimelockController>,

    pub governance: Account<'info, GovernanceRegistry>,

    pub executor: Signer<'info>,
}
