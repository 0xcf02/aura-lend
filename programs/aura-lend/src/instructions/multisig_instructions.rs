use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::multisig::*;
use crate::state::market::*;
use crate::error::LendingError;
use crate::constants::*;

/// Initialize a new multisig wallet
pub fn initialize_multisig(
    ctx: Context<InitializeMultisig>,
    params: InitializeMultisigParams,
) -> Result<()> {
    let multisig = &mut ctx.accounts.multisig;
    let market = &ctx.accounts.market;
    
    // Initialize the multisig
    **multisig = MultiSig::new(
        params.signatories,
        params.threshold,
        market.key(),
    )?;
    
    msg!("Multisig initialized with {} signatories, threshold: {}", 
         multisig.signatories.len(), multisig.threshold);
    Ok(())
}

/// Create a new multisig proposal
pub fn create_multisig_proposal(
    ctx: Context<CreateMultisigProposal>,
    params: CreateProposalParams,
) -> Result<()> {
    let multisig = &ctx.accounts.multisig;
    let proposal = &mut ctx.accounts.proposal;
    let proposer = &ctx.accounts.proposer;
    
    // Verify proposer is a signatory
    if !multisig.is_signatory(&proposer.key()) {
        return Err(LendingError::InvalidSignatory.into());
    }
    
    // Create the proposal
    **proposal = MultisigProposal::new(
        multisig.key(),
        multisig.nonce,
        params.operation_type,
        params.instruction_data,
        proposer.key(),
        params.expires_at,
    )?;
    
    msg!("Multisig proposal created by {}", proposer.key());
    Ok(())
}

/// Sign a multisig proposal
pub fn sign_multisig_proposal(
    ctx: Context<SignMultisigProposal>,
) -> Result<()> {
    let multisig = &ctx.accounts.multisig;
    let proposal = &mut ctx.accounts.proposal;
    let signer = &ctx.accounts.signer;
    
    // Verify signer is a signatory
    if !multisig.is_signatory(&signer.key()) {
        return Err(LendingError::InvalidSignatory.into());
    }
    
    // Check if proposal is expired
    if proposal.is_expired()? {
        return Err(LendingError::ProposalExpired.into());
    }
    
    // Check if proposal is still active
    if proposal.status != ProposalStatus::Active {
        return Err(LendingError::ProposalNotActive.into());
    }
    
    // Add signature
    proposal.add_signature(&signer.key())?;
    
    msg!("Proposal signed by {}. Signatures: {}/{}", 
         signer.key(), proposal.signatures.len(), multisig.threshold);
    Ok(())
}

/// Execute a multisig proposal (once threshold is met)
pub fn execute_multisig_proposal(
    ctx: Context<ExecuteMultisigProposal>,
) -> Result<()> {
    let multisig = &mut ctx.accounts.multisig;
    let proposal = &mut ctx.accounts.proposal;
    
    // Check if proposal has enough signatures
    if !proposal.has_enough_signatures(multisig.threshold) {
        return Err(LendingError::MultisigThresholdNotMet.into());
    }
    
    // Check if proposal is expired
    if proposal.is_expired()? {
        return Err(LendingError::ProposalExpired.into());
    }
    
    // Check if proposal is still active
    if proposal.status != ProposalStatus::Active {
        return Err(LendingError::ProposalNotActive.into());
    }
    
    // Verify nonce matches (prevents replay attacks)
    if proposal.nonce != multisig.nonce {
        return Err(LendingError::InvalidNonce.into());
    }
    
    // Mark proposal as executed
    proposal.mark_executed()?;
    
    // Increment multisig nonce
    multisig.increment_nonce()?;
    
    msg!("Multisig proposal executed successfully");
    
    // The actual operation execution would be handled by the calling instruction
    // This just validates and marks the proposal as ready for execution
    Ok(())
}

/// Cancel a multisig proposal (only by proposer or if expired)
pub fn cancel_multisig_proposal(
    ctx: Context<CancelMultisigProposal>,
) -> Result<()> {
    let proposal = &mut ctx.accounts.proposal;
    let authority = &ctx.accounts.authority;
    
    // Check if caller is the proposer or if proposal is expired
    let can_cancel = proposal.proposer == authority.key() || proposal.is_expired()?;
    
    if !can_cancel {
        return Err(LendingError::UnauthorizedCancellation.into());
    }
    
    // Mark proposal as cancelled
    proposal.mark_cancelled()?;
    
    msg!("Multisig proposal cancelled by {}", authority.key());
    Ok(())
}

/// Update multisig configuration (requires multisig approval)
pub fn update_multisig_config(
    ctx: Context<UpdateMultisigConfig>,
    params: InitializeMultisigParams,
) -> Result<()> {
    let multisig = &mut ctx.accounts.multisig;
    let proposal = &ctx.accounts.executed_proposal;
    
    // Verify this is being called through an executed proposal
    if proposal.status != ProposalStatus::Executed {
        return Err(LendingError::ProposalNotExecuted.into());
    }
    
    // Verify proposal is for updating multisig config
    if proposal.operation_type != MultisigOperationType::UpdateMultisigConfig {
        return Err(LendingError::InvalidOperationType.into());
    }
    
    // Update multisig configuration
    multisig.signatories = params.signatories;
    multisig.threshold = params.threshold;
    
    // Validate new configuration
    if multisig.threshold == 0 || multisig.threshold as usize > multisig.signatories.len() {
        return Err(LendingError::InvalidMultisigThreshold.into());
    }
    
    msg!("Multisig configuration updated");
    Ok(())
}

// Account validation structs

#[derive(Accounts)]
pub struct InitializeMultisig<'info> {
    #[account(
        init,
        payer = payer,
        space = MultiSig::SIZE,
        seeds = [MULTISIG_SEED, market.key().as_ref()],
        bump
    )]
    pub multisig: Account<'info, MultiSig>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(params: CreateProposalParams)]
pub struct CreateMultisigProposal<'info> {
    pub multisig: Account<'info, MultiSig>,
    
    #[account(
        init,
        payer = proposer,
        space = MultisigProposal::SIZE,
    )]
    pub proposal: Account<'info, MultisigProposal>,
    
    #[account(mut)]
    pub proposer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SignMultisigProposal<'info> {
    pub multisig: Account<'info, MultiSig>,
    
    #[account(mut)]
    pub proposal: Account<'info, MultisigProposal>,
    
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteMultisigProposal<'info> {
    #[account(mut)]
    pub multisig: Account<'info, MultiSig>,
    
    #[account(mut)]
    pub proposal: Account<'info, MultisigProposal>,
    
    /// The account executing the proposal (must be a signatory)
    pub executor: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelMultisigProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, MultisigProposal>,
    
    /// Either the proposer or emergency authority
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(params: InitializeMultisigParams)]
pub struct UpdateMultisigConfig<'info> {
    #[account(mut)]
    pub multisig: Account<'info, MultiSig>,
    
    /// The executed proposal that authorizes this update
    pub executed_proposal: Account<'info, MultisigProposal>,
    
    /// One of the signatories executing the update
    pub executor: Signer<'info>,
}