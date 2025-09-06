use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::LendingError,
    migration::{Migratable, validate_migration_compatibility},
    state::{
        market::Market,
        reserve::Reserve,
        obligation::Obligation,
        multisig::MultiSig,
        timelock::TimelockController,
        governance::GovernanceRegistry,
    },
    utils::validate_authority,
};

/// Migrate Market state to current version
pub fn migrate_market(ctx: Context<MigrateMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let authority = &ctx.accounts.authority;

    // Validate authority - only multisig owner can migrate
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    // Check if migration is needed
    if !market.needs_migration() {
        msg!("Market is already at the latest version {}", market.version());
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = market.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    market.migrate(from_version)?;

    msg!("Market migration completed from version {} to {}", from_version, PROGRAM_VERSION);
    Ok(())
}

/// Migrate Reserve state to current version
pub fn migrate_reserve(ctx: Context<MigrateReserve>) -> Result<()> {
    let market = &ctx.accounts.market;
    let reserve = &mut ctx.accounts.reserve;
    let authority = &ctx.accounts.authority;

    // Validate authority
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    // Verify reserve belongs to market
    if reserve.market != market.key() {
        return Err(LendingError::InvalidAccount.into());
    }

    // Check if migration is needed
    if !reserve.needs_migration() {
        msg!("Reserve is already at the latest version {}", reserve.version());
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = reserve.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    reserve.migrate(from_version)?;

    msg!("Reserve migration completed from version {} to {}", from_version, PROGRAM_VERSION);
    Ok(())
}

/// Migrate Obligation state to current version
pub fn migrate_obligation(ctx: Context<MigrateObligation>) -> Result<()> {
    let market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let authority = &ctx.accounts.authority;

    // Validate authority
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    // Verify obligation belongs to market
    if obligation.market != market.key() {
        return Err(LendingError::InvalidAccount.into());
    }

    // Check if migration is needed
    if !obligation.needs_migration() {
        msg!("Obligation is already at the latest version {}", obligation.version());
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = obligation.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    obligation.migrate(from_version)?;

    msg!("Obligation migration completed from version {} to {}", from_version, PROGRAM_VERSION);
    Ok(())
}

/// Migrate MultiSig state to current version
pub fn migrate_multisig(ctx: Context<MigrateMultisig>) -> Result<()> {
    let market = &ctx.accounts.market;
    let multisig = &mut ctx.accounts.multisig;
    let authority = &ctx.accounts.authority;

    // Validate authority
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    // Check if migration is needed
    if !multisig.needs_migration() {
        msg!("MultiSig is already at the latest version {}", multisig.version());
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = multisig.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    multisig.migrate(from_version)?;

    msg!("MultiSig migration completed from version {} to {}", from_version, PROGRAM_VERSION);
    Ok(())
}

/// Migrate TimelockController state to current version
pub fn migrate_timelock(ctx: Context<MigrateTimelock>) -> Result<()> {
    let market = &ctx.accounts.market;
    let timelock = &mut ctx.accounts.timelock;
    let authority = &ctx.accounts.authority;

    // Validate authority
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    // Check if migration is needed
    if !timelock.needs_migration() {
        msg!("TimelockController is already at the latest version {}", timelock.version());
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = timelock.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    timelock.migrate(from_version)?;

    msg!("TimelockController migration completed from version {} to {}", from_version, PROGRAM_VERSION);
    Ok(())
}

/// Migrate GovernanceRegistry state to current version
pub fn migrate_governance(ctx: Context<MigrateGovernance>) -> Result<()> {
    let market = &ctx.accounts.market;
    let governance = &mut ctx.accounts.governance;
    let authority = &ctx.accounts.authority;

    // Validate authority
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    // Check if migration is needed
    if !governance.needs_migration() {
        msg!("GovernanceRegistry is already at the latest version {}", governance.version());
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = governance.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    governance.migrate(from_version)?;

    msg!("GovernanceRegistry migration completed from version {} to {}", from_version, PROGRAM_VERSION);
    Ok(())
}

/// Batch migrate multiple reserves
pub fn batch_migrate_reserves(_ctx: Context<BatchMigrateReserves>) -> Result<()> {
    // TODO: Fix lifetime issues and implement batch migration
    return Err(LendingError::OperationNotPermitted.into());
    
    /*
    let market = &ctx.accounts.market;
    let authority = &ctx.accounts.authority;

    // Validate authority
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    let remaining_accounts = &ctx.remaining_accounts;
    let mut migrated_count = 0;
    let mut skipped_count = 0;

    // Process each reserve account
    for account_info in remaining_accounts.iter() {
        // Try to deserialize as Reserve
        if let Ok(mut reserve) = Account::<Reserve>::try_from(account_info) {
            // Verify reserve belongs to market
            if reserve.market != market.key() {
                msg!("Skipping reserve {} - belongs to different market", account_info.key());
                skipped_count += 1;
                continue;
            }

            // Check if migration is needed
            if reserve.needs_migration() {
                let from_version = reserve.version();
                match validate_migration_compatibility(from_version, PROGRAM_VERSION) {
                    Ok(()) => {
                        match reserve.migrate(from_version) {
                            Ok(()) => {
                                // Save the migrated account
                                reserve.exit(&crate::ID)?;
                                migrated_count += 1;
                                msg!("Migrated reserve {} from version {}", account_info.key(), from_version);
                            }
                            Err(e) => {
                                msg!("Failed to migrate reserve {}: {:?}", account_info.key(), e);
                                skipped_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        msg!("Invalid migration for reserve {}: {:?}", account_info.key(), e);
                        skipped_count += 1;
                    }
                }
            } else {
                msg!("Reserve {} already at latest version", account_info.key());
                skipped_count += 1;
            }
        } else {
            msg!("Invalid reserve account: {}", account_info.key());
            skipped_count += 1;
        }
    }

    msg!("Batch reserve migration completed: {} migrated, {} skipped", migrated_count, skipped_count);
    Ok(())
    */
}

// Account validation structs

#[derive(Accounts)]
pub struct MigrateMarket<'info> {
    #[account(
        mut,
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MigrateReserve<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        has_one = market @ LendingError::InvalidAccount
    )]
    pub reserve: Account<'info, Reserve>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MigrateObligation<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        has_one = market @ LendingError::InvalidAccount
    )]
    pub obligation: Account<'info, Obligation>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MigrateMultisig<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub multisig: Account<'info, MultiSig>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MigrateTimelock<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub timelock: Account<'info, TimelockController>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct MigrateGovernance<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    #[account(mut)]
    pub governance: Account<'info, GovernanceRegistry>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BatchMigrateReserves<'info> {
    #[account(
        seeds = [MARKET_SEED],
        bump,
        // Multisig owner validation will be done manually
    )]
    pub market: Account<'info, Market>,

    /// Authority (must be market's multisig owner)
    pub authority: Signer<'info>,
}