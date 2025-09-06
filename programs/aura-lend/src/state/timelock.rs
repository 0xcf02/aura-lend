use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::LendingError;

/// Timelock controller for delayed execution of critical operations
#[account]
pub struct TimelockController {
    /// Version of the timelock controller
    pub version: u8,
    
    /// The multisig that controls this timelock
    pub multisig: Pubkey,
    
    /// Minimum delay for different operation types (in seconds)
    pub min_delays: Vec<TimelockDelay>,
    
    /// List of active proposals awaiting execution
    pub active_proposals: Vec<Pubkey>,
    
    /// Timestamp when controller was created
    pub created_at: i64,
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 128],
}

impl TimelockController {
    /// Maximum number of active proposals
    pub const MAX_ACTIVE_PROPOSALS: usize = 50;
    
    /// Account size calculation
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // multisig
        4 + (10 * std::mem::size_of::<TimelockDelay>()) + // min_delays (assume max 10 operation types)
        4 + (Self::MAX_ACTIVE_PROPOSALS * 32) + // active_proposals
        8 + // created_at
        128; // reserved

    /// Create a new timelock controller
    pub fn new(multisig: Pubkey) -> Result<Self> {
        let clock = Clock::get()?;
        
        // Initialize default delays for different operation types
        let min_delays = vec![
            TimelockDelay {
                operation_type: TimelockOperationType::UpdateMarketOwner,
                delay_seconds: TIMELOCK_DELAY_CRITICAL, // 7 days
            },
            TimelockDelay {
                operation_type: TimelockOperationType::UpdateEmergencyAuthority,
                delay_seconds: TIMELOCK_DELAY_HIGH, // 3 days
            },
            TimelockDelay {
                operation_type: TimelockOperationType::UpdateReserveConfig,
                delay_seconds: TIMELOCK_DELAY_MEDIUM, // 1 day
            },
            TimelockDelay {
                operation_type: TimelockOperationType::InitializeReserve,
                delay_seconds: TIMELOCK_DELAY_LOW, // 6 hours
            },
            TimelockDelay {
                operation_type: TimelockOperationType::UpdateOracleConfig,
                delay_seconds: TIMELOCK_DELAY_MEDIUM, // 1 day
            },
            TimelockDelay {
                operation_type: TimelockOperationType::WithdrawFees,
                delay_seconds: TIMELOCK_DELAY_LOW, // 6 hours
            },
            // Program upgrade operations - critical delays
            TimelockDelay {
                operation_type: TimelockOperationType::ProgramUpgrade,
                delay_seconds: TIMELOCK_DELAY_CRITICAL, // 7 days
            },
            TimelockDelay {
                operation_type: TimelockOperationType::SetUpgradeAuthority,
                delay_seconds: TIMELOCK_DELAY_CRITICAL, // 7 days
            },
            TimelockDelay {
                operation_type: TimelockOperationType::FreezeProgram,
                delay_seconds: TIMELOCK_DELAY_CRITICAL, // 7 days
            },
            // Data migration operations - high priority
            TimelockDelay {
                operation_type: TimelockOperationType::DataMigration,
                delay_seconds: TIMELOCK_DELAY_HIGH, // 3 days
            },
        ];
        
        Ok(Self {
            version: PROGRAM_VERSION,
            multisig,
            min_delays,
            active_proposals: Vec::new(),
            created_at: clock.unix_timestamp,
            reserved: [0; 128],
        })
    }
    
    /// Get minimum delay for an operation type
    pub fn get_min_delay(&self, operation_type: TimelockOperationType) -> u64 {
        self.min_delays
            .iter()
            .find(|d| d.operation_type == operation_type)
            .map(|d| d.delay_seconds)
            .unwrap_or(TIMELOCK_DELAY_DEFAULT)
    }
    
    /// Add a proposal to active list
    pub fn add_active_proposal(&mut self, proposal: Pubkey) -> Result<()> {
        if self.active_proposals.len() >= Self::MAX_ACTIVE_PROPOSALS {
            return Err(LendingError::TooManyActiveProposals.into());
        }
        
        if self.active_proposals.contains(&proposal) {
            return Err(LendingError::ProposalAlreadyActive.into());
        }
        
        self.active_proposals.push(proposal);
        Ok(())
    }
    
    /// Remove a proposal from active list
    pub fn remove_active_proposal(&mut self, proposal: &Pubkey) -> Result<()> {
        if let Some(index) = self.active_proposals.iter().position(|p| p == proposal) {
            self.active_proposals.remove(index);
            Ok(())
        } else {
            Err(LendingError::ProposalNotFound.into())
        }
    }
}

/// Timelock proposal with delayed execution
#[account]
pub struct TimelockProposal {
    /// Version of the proposal
    pub version: u8,
    
    /// The timelock controller this belongs to
    pub controller: Pubkey,
    
    /// Type of operation
    pub operation_type: TimelockOperationType,
    
    /// Serialized instruction data
    pub instruction_data: Vec<u8>,
    
    /// Timestamp when proposal was created
    pub created_at: i64,
    
    /// Timestamp when proposal can be executed
    pub execution_time: i64,
    
    /// Status of the proposal
    pub status: TimelockStatus,
    
    /// Account that created this proposal
    pub proposer: Pubkey,
    
    /// Accounts that will be affected by this operation
    pub target_accounts: Vec<Pubkey>,
    
    /// Hash of the operation data for validation
    pub operation_hash: [u8; 32],
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 64],
}

impl TimelockProposal {
    /// Maximum size of instruction data
    pub const MAX_INSTRUCTION_SIZE: usize = 1024;
    
    /// Maximum number of target accounts
    pub const MAX_TARGET_ACCOUNTS: usize = 10;
    
    /// Account size calculation
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // controller
        1 + // operation_type
        4 + Self::MAX_INSTRUCTION_SIZE + // instruction_data
        8 + // created_at
        8 + // execution_time
        1 + // status
        32 + // proposer
        4 + (Self::MAX_TARGET_ACCOUNTS * 32) + // target_accounts
        32 + // operation_hash
        64; // reserved

    /// Create a new timelock proposal
    pub fn new(
        controller: Pubkey,
        operation_type: TimelockOperationType,
        instruction_data: Vec<u8>,
        delay_seconds: u64,
        proposer: Pubkey,
        target_accounts: Vec<Pubkey>,
    ) -> Result<Self> {
        if instruction_data.len() > Self::MAX_INSTRUCTION_SIZE {
            return Err(LendingError::InstructionTooLarge.into());
        }
        
        if target_accounts.len() > Self::MAX_TARGET_ACCOUNTS {
            return Err(LendingError::TooManyTargetAccounts.into());
        }
        
        let clock = Clock::get()?;
        let execution_time = clock.unix_timestamp
            .checked_add(delay_seconds as i64)
            .ok_or(LendingError::MathOverflow)?;
            
        // Create hash of operation data for validation
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        instruction_data.hash(&mut hasher);
        target_accounts.hash(&mut hasher);
        let operation_hash_u64 = hasher.finish();
        let mut operation_hash = [0u8; 32];
        operation_hash[0..8].copy_from_slice(&operation_hash_u64.to_le_bytes());
        
        Ok(Self {
            version: PROGRAM_VERSION,
            controller,
            operation_type,
            instruction_data,
            created_at: clock.unix_timestamp,
            execution_time,
            status: TimelockStatus::Pending,
            proposer,
            target_accounts,
            operation_hash,
            reserved: [0; 64],
        })
    }
    
    /// Check if proposal is ready for execution
    pub fn is_ready_for_execution(&self) -> Result<bool> {
        if self.status != TimelockStatus::Pending {
            return Ok(false);
        }
        
        let clock = Clock::get()?;
        Ok(clock.unix_timestamp >= self.execution_time)
    }
    
    /// Check if proposal is expired
    pub fn is_expired(&self) -> Result<bool> {
        let clock = Clock::get()?;
        // Proposals expire if not executed within 30 days of execution time
        let expiry_time = self.execution_time
            .checked_add(TIMELOCK_EXPIRY_PERIOD)
            .ok_or(LendingError::MathOverflow)?;
        
        Ok(clock.unix_timestamp > expiry_time)
    }
    
    /// Mark proposal as executed
    pub fn mark_executed(&mut self) -> Result<()> {
        if self.status != TimelockStatus::Pending {
            return Err(LendingError::ProposalNotPending.into());
        }
        
        self.status = TimelockStatus::Executed;
        Ok(())
    }
    
    /// Mark proposal as cancelled
    pub fn mark_cancelled(&mut self) -> Result<()> {
        if self.status != TimelockStatus::Pending {
            return Err(LendingError::ProposalNotPending.into());
        }
        
        self.status = TimelockStatus::Cancelled;
        Ok(())
    }
}

/// Delay configuration for different operation types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct TimelockDelay {
    pub operation_type: TimelockOperationType,
    pub delay_seconds: u64,
}

/// Types of operations that can be timelocked
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelockOperationType {
    /// Update market owner (critical - 7 days)
    UpdateMarketOwner,
    /// Update emergency authority (high - 3 days)
    UpdateEmergencyAuthority,
    /// Update reserve configuration (medium - 1 day)
    UpdateReserveConfig,
    /// Initialize new reserve (low - 6 hours)
    InitializeReserve,
    /// Update oracle configuration (medium - 1 day)
    UpdateOracleConfig,
    /// Withdraw protocol fees (low - 6 hours)
    WithdrawFees,
    /// Update timelock delays (critical - 7 days)
    UpdateTimelockDelays,
    /// Grant administrative role (high - 3 days)
    GrantRole,
    /// Revoke administrative role (medium - 1 day)
    RevokeRole,
    /// Program upgrade (critical - 7 days)
    ProgramUpgrade,
    /// Set upgrade authority (critical - 7 days)
    SetUpgradeAuthority,
    /// Freeze program permanently (critical - 7 days)
    FreezeProgram,
    /// Data migration operations (high - 3 days)
    DataMigration,
}

impl Default for TimelockOperationType {
    fn default() -> Self {
        Self::UpdateReserveConfig
    }
}

/// Status of a timelock proposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TimelockStatus {
    /// Proposal is waiting for execution time
    Pending,
    /// Proposal has been executed
    Executed,
    /// Proposal has been cancelled
    Cancelled,
    /// Proposal has expired
    Expired,
}

impl Default for TimelockStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Parameters for creating a timelock proposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTimelockProposalParams {
    pub operation_type: TimelockOperationType,
    pub instruction_data: Vec<u8>,
    pub target_accounts: Vec<Pubkey>,
}