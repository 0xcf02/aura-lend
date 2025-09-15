use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    program::invoke_signed,
    system_instruction,
};

use crate::{constants::*, error::LendingError, state::market::Market, utils::validate_authority};

/// Set the upgrade authority of the program to a new authority (typically MultiSig)
pub fn set_upgrade_authority(ctx: Context<SetUpgradeAuthority>) -> Result<()> {
    let market = &ctx.accounts.market;
    let current_authority = &ctx.accounts.current_authority;
    let new_authority = ctx.accounts.new_authority.key();
    let program_data = &ctx.accounts.program_data;

    // Validate that the current authority is the market's multisig owner
    validate_authority(&current_authority.to_account_info(), &market.multisig_owner)?;

    // Verify program data account
    let program_data_info = program_data.to_account_info();
    if program_data_info.owner != &bpf_loader_upgradeable::id() {
        return Err(LendingError::InvalidAccountOwner.into());
    }

    // Create the set authority instruction for BPF Upgradeable Loader
    let set_authority_ix = bpf_loader_upgradeable::set_upgrade_authority(
        &ctx.accounts.program_data.key(),
        &current_authority.key(),
        Some(&new_authority),
    );

    // Execute the instruction with current authority signature
    invoke_signed(
        &set_authority_ix,
        &[
            program_data.to_account_info(),
            current_authority.to_account_info(),
        ],
        &[],
    )?;

    msg!(
        "Program upgrade authority transferred to: {}",
        new_authority
    );
    Ok(())
}

/// Upgrade the program to a new buffer account
pub fn upgrade_program(ctx: Context<UpgradeProgram>) -> Result<()> {
    let market = &ctx.accounts.market;
    let upgrade_authority = &ctx.accounts.upgrade_authority;
    let buffer_account = ctx.accounts.buffer_account.key();

    // Validate that the upgrade authority is the market's multisig owner
    validate_authority(&upgrade_authority.to_account_info(), &market.multisig_owner)?;

    // Create the upgrade instruction
    let upgrade_ix = bpf_loader_upgradeable::upgrade(
        &ctx.accounts.program_id.key(),
        &buffer_account,
        &upgrade_authority.key(),
        &ctx.accounts.spill_account.key(),
    );

    // Create upgrade authority seeds for PDA signing if needed
    let authority_seeds: &[&[&[u8]]] = &[];

    // Execute the upgrade instruction
    invoke_signed(
        &upgrade_ix,
        &[
            ctx.accounts.program_data.to_account_info(),
            ctx.accounts.program_id.to_account_info(),
            ctx.accounts.buffer_account.to_account_info(),
            ctx.accounts.spill_account.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.clock.to_account_info(),
            upgrade_authority.to_account_info(),
        ],
        authority_seeds,
    )?;

    msg!(
        "Program successfully upgraded using buffer: {}",
        buffer_account
    );
    Ok(())
}

/// Freeze the program (remove upgrade authority permanently)
pub fn freeze_program(ctx: Context<FreezeProgram>) -> Result<()> {
    let market = &ctx.accounts.market;
    let upgrade_authority = &ctx.accounts.upgrade_authority;

    // Validate that the upgrade authority is the market's multisig owner
    validate_authority(&upgrade_authority.to_account_info(), &market.multisig_owner)?;

    // Create the set authority instruction to remove upgrade authority (set to None)
    let freeze_ix = bpf_loader_upgradeable::set_upgrade_authority(
        &ctx.accounts.program_data.key(),
        &upgrade_authority.key(),
        None, // Setting to None freezes the program
    );

    // Execute the instruction
    invoke_signed(
        &freeze_ix,
        &[
            ctx.accounts.program_data.to_account_info(),
            upgrade_authority.to_account_info(),
        ],
        &[],
    )?;

    msg!("Program permanently frozen - upgrade authority removed");
    Ok(())
}

#[derive(Accounts)]
pub struct SetUpgradeAuthority<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    /// Current upgrade authority (must be market's multisig owner)
    pub current_authority: Signer<'info>,

    /// New upgrade authority (can be any account)
    /// CHECK: This can be any account that will become the new upgrade authority
    pub new_authority: UncheckedAccount<'info>,

    /// Program data account of the upgradeable program
    #[account(mut)]
    pub program_data: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpgradeProgram<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    /// Upgrade authority (must be market's multisig owner)
    pub upgrade_authority: Signer<'info>,

    /// Program data account
    #[account(mut)]
    pub program_data: UncheckedAccount<'info>,

    /// Program ID account
    pub program_id: UncheckedAccount<'info>,

    /// Buffer account containing the new program code
    #[account(mut)]
    pub buffer_account: UncheckedAccount<'info>,

    /// Account to receive refunded lamports from the buffer
    #[account(mut)]
    pub spill_account: UncheckedAccount<'info>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,

    /// Clock sysvar
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct FreezeProgram<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    /// Upgrade authority (must be market's multisig owner)
    pub upgrade_authority: Signer<'info>,

    /// Program data account
    #[account(mut)]
    pub program_data: UncheckedAccount<'info>,
}
