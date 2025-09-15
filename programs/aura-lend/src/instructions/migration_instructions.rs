use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::LendingError,
    migration::{validate_migration_compatibility, Migratable},
    state::{
        governance::GovernanceRegistry, market::Market, multisig::MultiSig, obligation::Obligation,
        reserve::Reserve, timelock::TimelockController,
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
        msg!(
            "Market is already at the latest version {}",
            market.version()
        );
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = market.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    market.migrate(from_version)?;

    msg!(
        "Market migration completed from version {} to {}",
        from_version,
        PROGRAM_VERSION
    );
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
        msg!(
            "Reserve is already at the latest version {}",
            reserve.version()
        );
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = reserve.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    reserve.migrate(from_version)?;

    msg!(
        "Reserve migration completed from version {} to {}",
        from_version,
        PROGRAM_VERSION
    );
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
        msg!(
            "Obligation is already at the latest version {}",
            obligation.version()
        );
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = obligation.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    obligation.migrate(from_version)?;

    msg!(
        "Obligation migration completed from version {} to {}",
        from_version,
        PROGRAM_VERSION
    );
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
        msg!(
            "MultiSig is already at the latest version {}",
            multisig.version()
        );
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = multisig.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    multisig.migrate(from_version)?;

    msg!(
        "MultiSig migration completed from version {} to {}",
        from_version,
        PROGRAM_VERSION
    );
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
        msg!(
            "TimelockController is already at the latest version {}",
            timelock.version()
        );
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = timelock.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    timelock.migrate(from_version)?;

    msg!(
        "TimelockController migration completed from version {} to {}",
        from_version,
        PROGRAM_VERSION
    );
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
        msg!(
            "GovernanceRegistry is already at the latest version {}",
            governance.version()
        );
        return Err(LendingError::MigrationAlreadyCompleted.into());
    }

    let from_version = governance.version();
    validate_migration_compatibility(from_version, PROGRAM_VERSION)?;

    // Perform migration
    governance.migrate(from_version)?;

    msg!(
        "GovernanceRegistry migration completed from version {} to {}",
        from_version,
        PROGRAM_VERSION
    );
    Ok(())
}

/// Batch migrate multiple reserves
pub fn batch_migrate_reserves<'info>(
    ctx: Context<'_, '_, '_, 'info, BatchMigrateReserves<'info>>,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let authority = &ctx.accounts.authority;

    // Validate authority has proper permissions
    validate_authority(&authority.to_account_info(), &market.multisig_owner)?;

    let remaining_accounts = &ctx.remaining_accounts;
    let mut migrated_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;

    // Process each reserve account in batches to avoid transaction size limits
    for account_info in remaining_accounts.iter() {
        // Validate account ownership
        if account_info.owner != &crate::id() {
            msg!(
                "Skipping account {} - not owned by program",
                account_info.key()
            );
            skipped_count += 1;
            continue;
        }

        // Try to deserialize as Reserve - use manual deserialization to avoid borrowing issues
        let account_data = account_info
            .try_borrow_data()
            .map_err(|_| LendingError::InvalidAccount)?;

        // Check if this is a Reserve account by checking discriminator
        if account_data.len() < 8 {
            skipped_count += 1;
            continue;
        }

        // Reserve discriminator check
        let expected_discriminator = anchor_lang::Discriminator::discriminator(&Reserve::default());
        if &account_data[0..8] != expected_discriminator {
            msg!(
                "Skipping account {} - not a Reserve account",
                account_info.key()
            );
            skipped_count += 1;
            continue;
        }

        // Drop the borrow before working with the account
        drop(account_data);

        // Now work with the account as a Reserve
        let mut reserve_account =
            Account::<Reserve>::try_from(account_info).map_err(|_| LendingError::InvalidAccount)?;

        // Verify reserve belongs to this market
        if reserve_account.market != market.key() {
            msg!(
                "Skipping reserve {} - belongs to different market",
                account_info.key()
            );
            skipped_count += 1;
            continue;
        }

        // Check if migration is needed
        if reserve_account.needs_migration() {
            let from_version = reserve_account.version();
            match validate_migration_compatibility(from_version, PROGRAM_VERSION) {
                Ok(()) => match reserve_account.migrate(from_version) {
                    Ok(()) => {
                        migrated_count += 1;
                        msg!(
                            "Successfully migrated reserve {} from version {} to {}",
                            account_info.key(),
                            from_version,
                            PROGRAM_VERSION
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        msg!("Failed to migrate reserve {}: {:?}", account_info.key(), e);
                    }
                },
                Err(e) => {
                    failed_count += 1;
                    msg!(
                        "Migration compatibility check failed for reserve {}: {:?}",
                        account_info.key(),
                        e
                    );
                }
            }
        } else {
            skipped_count += 1;
            msg!(
                "Reserve {} already up to date (version {})",
                account_info.key(),
                reserve_account.version()
            );
        }
    }

    msg!(
        "Batch migration completed: {} migrated, {} skipped, {} failed",
        migrated_count,
        skipped_count,
        failed_count
    );

    // Return error if any migrations failed
    if failed_count > 0 {
        return Err(LendingError::PartialMigrationFailure.into());
    }

    Ok(())
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
