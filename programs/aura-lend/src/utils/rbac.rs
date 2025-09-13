use anchor_lang::prelude::*;

/// Role-based access control definitions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum Role {
    SuperAdmin,
    Admin,
    Operator,
    User,
}

impl Role {
    pub fn has_permission(&self, required_role: &Role) -> bool {
        match (self, required_role) {
            (Role::SuperAdmin, _) => true,
            (Role::Admin, Role::Admin | Role::Operator | Role::User) => true,
            (Role::Operator, Role::Operator | Role::User) => true,
            (Role::User, Role::User) => true,
            _ => false,
        }
    }
}

/// Validates that an account has the required role
pub fn require_role(user_role: &Role, required_role: &Role) -> Result<()> {
    if !user_role.has_permission(required_role) {
        return Err(error!(crate::error::LendingError::InsufficientPermissions));
    }
    Ok(())
}

/// Checks if a user has admin privileges
pub fn is_admin(role: &Role) -> bool {
    matches!(role, Role::SuperAdmin | Role::Admin)
}

/// Checks if a user has operator privileges or higher
pub fn is_operator_or_higher(role: &Role) -> bool {
    matches!(role, Role::SuperAdmin | Role::Admin | Role::Operator)
}