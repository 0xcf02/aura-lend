use crate::constants::*;
use crate::error::LendingError;
use anchor_lang::prelude::*;

/// Governance system for role-based access control
#[account]
pub struct GovernanceRegistry {
    /// Version of the governance registry
    pub version: u8,

    /// The multisig that controls this governance system
    pub multisig: Pubkey,

    /// List of all active roles
    pub roles: Vec<GovernanceRole>,

    /// Global permissions that can be granted
    pub available_permissions: u64, // Bitmap of all possible permissions

    /// Timestamp when registry was created
    pub created_at: i64,

    /// Reserved space for future upgrades
    pub reserved: [u8; 128],
}

impl GovernanceRegistry {
    /// Maximum number of concurrent roles
    pub const MAX_ROLES: usize = 50;

    /// Account size calculation
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // multisig
        4 + (Self::MAX_ROLES * std::mem::size_of::<GovernanceRole>()) + // roles
        8 + // available_permissions
        8 + // created_at
        128; // reserved

    /// Create a new governance registry
    pub fn new(multisig: Pubkey) -> Result<Self> {
        let clock = Clock::get()?;

        // Initialize with all permissions available
        let available_permissions = Permission::SUPER_ADMIN.bits()
            | Permission::RESERVE_MANAGER.bits()
            | Permission::RISK_MANAGER.bits()
            | Permission::ORACLE_MANAGER.bits()
            | Permission::EMERGENCY_RESPONDER.bits()
            | Permission::FEE_MANAGER.bits()
            | Permission::GOVERNANCE_MANAGER.bits()
            | Permission::TIMELOCK_MANAGER.bits()
            | Permission::PROGRAM_UPGRADE_MANAGER.bits()
            | Permission::DATA_MIGRATION_MANAGER.bits();

        Ok(Self {
            version: PROGRAM_VERSION,
            multisig,
            roles: Vec::new(),
            available_permissions,
            created_at: clock.unix_timestamp,
            reserved: [0; 128],
        })
    }

    /// Grant a role to an account
    pub fn grant_role(
        &mut self,
        holder: Pubkey,
        role_type: RoleType,
        permissions: u64,
        expires_at: Option<i64>,
        granted_by: Pubkey,
    ) -> Result<()> {
        // Check if we have space for more roles
        if self.roles.len() >= Self::MAX_ROLES {
            return Err(LendingError::TooManyRoles.into());
        }

        // Check if account already has an active role
        if self.get_active_role(&holder).is_some() {
            return Err(LendingError::AccountAlreadyHasRole.into());
        }

        // Validate permissions are available
        if (permissions & self.available_permissions) != permissions {
            return Err(LendingError::InvalidPermissions.into());
        }

        let clock = Clock::get()?;
        let role = GovernanceRole {
            holder,
            role_type,
            permissions,
            granted_at: clock.unix_timestamp,
            expires_at,
            granted_by,
            is_active: true,
        };

        self.roles.push(role);
        Ok(())
    }

    /// Revoke a role from an account
    pub fn revoke_role(&mut self, holder: &Pubkey) -> Result<()> {
        if let Some(role) = self
            .roles
            .iter_mut()
            .find(|r| r.holder == *holder && r.is_active)
        {
            role.is_active = false;
            Ok(())
        } else {
            Err(LendingError::RoleNotFound.into())
        }
    }

    /// Get active role for an account
    pub fn get_active_role(&self, holder: &Pubkey) -> Option<&GovernanceRole> {
        self.roles
            .iter()
            .find(|r| r.holder == *holder && r.is_active && !r.is_expired().unwrap_or(true))
    }

    /// Check if account has specific permission
    pub fn has_permission(&self, holder: &Pubkey, permission: Permission) -> bool {
        if let Some(role) = self.get_active_role(holder) {
            (role.permissions & permission.bits()) != 0
        } else {
            false
        }
    }

    /// Check if account has any of the specified permissions
    pub fn has_any_permission(&self, holder: &Pubkey, permissions: &[Permission]) -> bool {
        if let Some(role) = self.get_active_role(holder) {
            permissions
                .iter()
                .any(|p| (role.permissions & p.bits()) != 0)
        } else {
            false
        }
    }

    /// Clean up expired roles
    pub fn cleanup_expired_roles(&mut self) -> Result<usize> {
        let initial_count = self.roles.len();
        self.roles.retain(|role| !role.is_expired().unwrap_or(true));
        Ok(initial_count - self.roles.len())
    }
}

/// Individual role assigned to an account
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GovernanceRole {
    /// Account that holds this role
    pub holder: Pubkey,

    /// Type of role
    pub role_type: RoleType,

    /// Permissions bitmap
    pub permissions: u64,

    /// When this role was granted
    pub granted_at: i64,

    /// Optional expiration time
    pub expires_at: Option<i64>,

    /// Account that granted this role
    pub granted_by: Pubkey,

    /// Whether this role is currently active
    pub is_active: bool,
}

impl GovernanceRole {
    /// Check if the role is expired
    pub fn is_expired(&self) -> Result<bool> {
        if let Some(expires_at) = self.expires_at {
            let clock = Clock::get()?;
            Ok(clock.unix_timestamp > expires_at)
        } else {
            Ok(false)
        }
    }

    /// Check if role has a specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.is_active && (self.permissions & permission.bits()) != 0
    }
}

/// Types of roles that can be assigned
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoleType {
    /// Super administrator with all permissions
    SuperAdmin,
    /// Can manage reserve configurations
    ReserveManager,
    /// Can adjust risk parameters
    RiskManager,
    /// Can manage oracle configurations
    OracleManager,
    /// Can respond to emergencies
    EmergencyResponder,
    /// Can manage protocol fees
    FeeManager,
    /// Can manage governance and roles
    GovernanceManager,
    /// Can manage timelock proposals
    TimelockManager,
    /// Can manage program upgrades
    ProgramUpgradeManager,
    /// Can perform data migrations
    DataMigrationManager,
}

impl Default for RoleType {
    fn default() -> Self {
        Self::ReserveManager
    }
}

/// Permission flags that can be combined
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Permission {
    bits: u64,
}

impl Permission {
    /// Super admin - all permissions
    pub const SUPER_ADMIN: Self = Self { bits: u64::MAX };

    /// Can initialize and configure reserves
    pub const RESERVE_MANAGER: Self = Self { bits: 1 << 0 };

    /// Can update risk parameters (LTV, liquidation thresholds, etc.)
    pub const RISK_MANAGER: Self = Self { bits: 1 << 1 };

    /// Can manage oracle configurations and feeds
    pub const ORACLE_MANAGER: Self = Self { bits: 1 << 2 };

    /// Can pause protocol and handle emergencies
    pub const EMERGENCY_RESPONDER: Self = Self { bits: 1 << 3 };

    /// Can manage protocol fees and fee collection
    pub const FEE_MANAGER: Self = Self { bits: 1 << 4 };

    /// Can grant and revoke roles
    pub const GOVERNANCE_MANAGER: Self = Self { bits: 1 << 5 };

    /// Can create and execute timelock proposals
    pub const TIMELOCK_MANAGER: Self = Self { bits: 1 << 6 };

    /// Can update interest rate models
    pub const RATE_MANAGER: Self = Self { bits: 1 << 7 };

    /// Can manage collateral configurations
    pub const COLLATERAL_MANAGER: Self = Self { bits: 1 << 8 };

    /// Can execute liquidations (for automated liquidators)
    pub const LIQUIDATION_MANAGER: Self = Self { bits: 1 << 9 };

    /// Can manage program upgrades and upgrade authority
    pub const PROGRAM_UPGRADE_MANAGER: Self = Self { bits: 1 << 10 };

    /// Can perform data migrations between versions
    pub const DATA_MIGRATION_MANAGER: Self = Self { bits: 1 << 11 };

    /// Get the bits value
    pub fn bits(&self) -> u64 {
        self.bits
    }

    /// Check if contains another permission
    pub fn contains(&self, other: Permission) -> bool {
        (self.bits & other.bits) == other.bits
    }

    /// Combine permissions
    pub fn union(&self, other: Permission) -> Permission {
        Permission {
            bits: self.bits | other.bits,
        }
    }
}

/// Validation helper for permission checks
pub struct PermissionChecker;

impl PermissionChecker {
    /// Check if account has required permission for operation
    pub fn check_permission(
        governance: &GovernanceRegistry,
        account: &Pubkey,
        required_permission: Permission,
    ) -> Result<()> {
        if governance.has_permission(account, required_permission) {
            Ok(())
        } else {
            Err(LendingError::InsufficientPermissions.into())
        }
    }

    /// Check if account has any of the required permissions
    pub fn check_any_permission(
        governance: &GovernanceRegistry,
        account: &Pubkey,
        required_permissions: &[Permission],
    ) -> Result<()> {
        if governance.has_any_permission(account, required_permissions) {
            Ok(())
        } else {
            Err(LendingError::InsufficientPermissions.into())
        }
    }
}

/// Parameters for granting a role
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct GrantRoleParams {
    pub holder: Pubkey,
    pub role_type: RoleType,
    pub permissions: u64,
    pub expires_at: Option<i64>,
}

/// Parameters for creating governance registry
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeGovernanceParams {
    pub multisig: Pubkey,
}
