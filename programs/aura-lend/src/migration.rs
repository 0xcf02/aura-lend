use anchor_lang::prelude::*;

use crate::{
    constants::PROGRAM_VERSION,
    error::LendingError,
    state::{
        governance::GovernanceRegistry, market::Market, multisig::MultiSig, obligation::Obligation,
        reserve::Reserve, timelock::TimelockController,
    },
};

/// Version migration trait that all state structures should implement
pub trait Migratable {
    /// Current version of the structure
    fn current_version() -> u8 {
        PROGRAM_VERSION
    }

    /// Get the version of this instance
    fn version(&self) -> u8;

    /// Migrate from an older version to the current version
    fn migrate(&mut self, from_version: u8) -> Result<()>;

    /// Check if migration is needed
    fn needs_migration(&self) -> bool {
        self.version() < Self::current_version()
    }
}

/// Migration handler for Market state
impl Migratable for Market {
    fn version(&self) -> u8 {
        self.version
    }

    fn migrate(&mut self, from_version: u8) -> Result<()> {
        msg!(
            "Migrating Market from version {} to {}",
            from_version,
            PROGRAM_VERSION
        );

        match from_version {
            1 => {
                // Currently at version 1, no migration needed yet
                // Future migrations will be added here
                msg!("Market already at latest version");
            }
            _ => {
                msg!("Unsupported Market migration from version {}", from_version);
                return Err(LendingError::UnsupportedMigration.into());
            }
        }

        // Update version to current
        self.version = PROGRAM_VERSION;
        msg!("Market migration completed to version {}", PROGRAM_VERSION);
        Ok(())
    }
}

/// Migration handler for Reserve state
impl Migratable for Reserve {
    fn version(&self) -> u8 {
        self.version
    }

    fn migrate(&mut self, from_version: u8) -> Result<()> {
        msg!(
            "Migrating Reserve from version {} to {}",
            from_version,
            PROGRAM_VERSION
        );

        match from_version {
            1 => {
                // Currently at version 1, no migration needed yet
                // Future migrations could include:
                // - New config parameters
                // - Updated state calculations
                // - Additional oracle support
                msg!("Reserve already at latest version");
            }
            _ => {
                msg!(
                    "Unsupported Reserve migration from version {}",
                    from_version
                );
                return Err(LendingError::UnsupportedMigration.into());
            }
        }

        // Update version to current
        self.version = PROGRAM_VERSION;
        msg!("Reserve migration completed to version {}", PROGRAM_VERSION);
        Ok(())
    }
}

/// Migration handler for Obligation state
impl Migratable for Obligation {
    fn version(&self) -> u8 {
        self.version
    }

    fn migrate(&mut self, from_version: u8) -> Result<()> {
        msg!(
            "Migrating Obligation from version {} to {}",
            from_version,
            PROGRAM_VERSION
        );

        match from_version {
            1 => {
                // Currently at version 1, no migration needed yet
                // Future migrations could include:
                // - New collateral types
                // - Updated health calculations
                // - Additional tracking fields
                msg!("Obligation already at latest version");
            }
            _ => {
                msg!(
                    "Unsupported Obligation migration from version {}",
                    from_version
                );
                return Err(LendingError::UnsupportedMigration.into());
            }
        }

        // Update version to current
        self.version = PROGRAM_VERSION;
        msg!(
            "Obligation migration completed to version {}",
            PROGRAM_VERSION
        );
        Ok(())
    }
}

/// Migration handler for MultiSig state
impl Migratable for MultiSig {
    fn version(&self) -> u8 {
        self.version
    }

    fn migrate(&mut self, from_version: u8) -> Result<()> {
        msg!(
            "Migrating MultiSig from version {} to {}",
            from_version,
            PROGRAM_VERSION
        );

        match from_version {
            1 => {
                // Currently at version 1, no migration needed yet
                // Future migrations could include:
                // - New operation types
                // - Updated signature requirements
                // - Additional security features
                msg!("MultiSig already at latest version");
            }
            _ => {
                msg!(
                    "Unsupported MultiSig migration from version {}",
                    from_version
                );
                return Err(LendingError::UnsupportedMigration.into());
            }
        }

        // Update version to current
        self.version = PROGRAM_VERSION;
        msg!(
            "MultiSig migration completed to version {}",
            PROGRAM_VERSION
        );
        Ok(())
    }
}

/// Migration handler for TimelockController state
impl Migratable for TimelockController {
    fn version(&self) -> u8 {
        self.version
    }

    fn migrate(&mut self, from_version: u8) -> Result<()> {
        msg!(
            "Migrating TimelockController from version {} to {}",
            from_version,
            PROGRAM_VERSION
        );

        match from_version {
            1 => {
                // Currently at version 1, no migration needed yet
                // Future migrations could include:
                // - New delay configurations
                // - Updated proposal types
                // - Enhanced security checks
                msg!("TimelockController already at latest version");
            }
            _ => {
                msg!(
                    "Unsupported TimelockController migration from version {}",
                    from_version
                );
                return Err(LendingError::UnsupportedMigration.into());
            }
        }

        // Update version to current
        self.version = PROGRAM_VERSION;
        msg!(
            "TimelockController migration completed to version {}",
            PROGRAM_VERSION
        );
        Ok(())
    }
}

/// Migration handler for GovernanceRegistry state
impl Migratable for GovernanceRegistry {
    fn version(&self) -> u8 {
        self.version
    }

    fn migrate(&mut self, from_version: u8) -> Result<()> {
        msg!(
            "Migrating GovernanceRegistry from version {} to {}",
            from_version,
            PROGRAM_VERSION
        );

        match from_version {
            1 => {
                // Currently at version 1, no migration needed yet
                // Future migrations could include:
                // - New permission types
                // - Updated role structures
                // - Enhanced delegation features
                msg!("GovernanceRegistry already at latest version");
            }
            _ => {
                msg!(
                    "Unsupported GovernanceRegistry migration from version {}",
                    from_version
                );
                return Err(LendingError::UnsupportedMigration.into());
            }
        }

        // Update version to current
        self.version = PROGRAM_VERSION;
        msg!(
            "GovernanceRegistry migration completed to version {}",
            PROGRAM_VERSION
        );
        Ok(())
    }
}

/// Generic migration validator
pub fn validate_migration_compatibility(from_version: u8, to_version: u8) -> Result<()> {
    if from_version > to_version {
        msg!(
            "Cannot downgrade from version {} to {}",
            from_version,
            to_version
        );
        return Err(LendingError::InvalidMigration.into());
    }

    if from_version == to_version {
        msg!("Already at target version {}", to_version);
        return Ok(());
    }

    // Check for supported migration paths
    match from_version {
        1 => {
            // Version 1 can migrate to any future version
            msg!("Migration from version 1 to {} is supported", to_version);
        }
        _ => {
            msg!("Unsupported migration from version {}", from_version);
            return Err(LendingError::UnsupportedMigration.into());
        }
    }

    Ok(())
}

/// Batch migration helper for multiple accounts
pub fn batch_migrate_accounts<T: Migratable>(accounts: &mut [T]) -> Result<()> {
    let mut migrated_count = 0;
    let mut error_count = 0;

    for account in accounts.iter_mut() {
        if account.needs_migration() {
            let from_version = account.version();
            match account.migrate(from_version) {
                Ok(()) => {
                    migrated_count += 1;
                    msg!(
                        "Successfully migrated account from version {}",
                        from_version
                    );
                }
                Err(e) => {
                    error_count += 1;
                    msg!(
                        "Failed to migrate account from version {}: {:?}",
                        from_version,
                        e
                    );
                    // Continue with other accounts instead of failing entirely
                }
            }
        }
    }

    msg!(
        "Batch migration completed: {} migrated, {} errors",
        migrated_count,
        error_count
    );

    if error_count > 0 {
        return Err(LendingError::PartialMigrationFailure.into());
    }

    Ok(())
}
