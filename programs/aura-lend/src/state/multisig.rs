use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::LendingError;

/// Multi-signature wallet for critical protocol operations
/// Requires multiple signatures before execution
#[account]
#[derive(Default)]
pub struct MultiSig {
    /// Version of the multisig account structure
    pub version: u8,
    
    /// List of public keys that can sign transactions
    pub signatories: Vec<Pubkey>,
    
    /// Number of signatures required to execute a transaction
    pub threshold: u8,
    
    /// Current nonce to prevent replay attacks
    pub nonce: u64,
    
    /// The market this multisig controls
    pub market: Pubkey,
    
    /// Timestamp when this multisig was created
    pub created_at: i64,
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 128],
}

impl MultiSig {
    /// Maximum number of signatories allowed
    pub const MAX_SIGNATORIES: usize = 10;
    
    /// Account size calculation
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        4 + (Self::MAX_SIGNATORIES * 32) + // signatories (Vec<Pubkey>)
        1 + // threshold
        8 + // nonce
        32 + // market
        8 + // created_at
        128; // reserved

    /// Create a new multisig wallet
    pub fn new(
        signatories: Vec<Pubkey>,
        threshold: u8,
        market: Pubkey,
    ) -> Result<Self> {
        // Validate threshold
        if threshold == 0 || threshold as usize > signatories.len() {
            return Err(LendingError::InvalidMultisigThreshold.into());
        }
        
        // Validate number of signatories
        if signatories.is_empty() || signatories.len() > Self::MAX_SIGNATORIES {
            return Err(LendingError::InvalidSignatoryCount.into());
        }
        
        // Validate no duplicate signatories
        let mut sorted_sigs = signatories.clone();
        sorted_sigs.sort();
        for i in 1..sorted_sigs.len() {
            if sorted_sigs[i] == sorted_sigs[i - 1] {
                return Err(LendingError::DuplicateSignatory.into());
            }
        }
        
        let clock = Clock::get()?;
        Ok(Self {
            version: PROGRAM_VERSION,
            signatories,
            threshold,
            nonce: 0,
            market,
            created_at: clock.unix_timestamp,
            reserved: [0; 128],
        })
    }
    
    /// Check if a pubkey is a valid signatory
    pub fn is_signatory(&self, pubkey: &Pubkey) -> bool {
        self.signatories.contains(pubkey)
    }
    
    /// Increment nonce to prevent replay attacks
    pub fn increment_nonce(&mut self) -> Result<u64> {
        self.nonce = self.nonce
            .checked_add(1)
            .ok_or(LendingError::MathOverflow)?;
        Ok(self.nonce)
    }
}

/// Multisig transaction proposal
#[account]
#[derive(Default)]
pub struct MultisigProposal {
    /// Version of the proposal account structure
    pub version: u8,
    
    /// The multisig this proposal belongs to
    pub multisig: Pubkey,
    
    /// Current nonce of the multisig when this proposal was created
    pub nonce: u64,
    
    /// Type of operation being proposed
    pub operation_type: MultisigOperationType,
    
    /// Serialized instruction data for the operation
    pub instruction_data: Vec<u8>,
    
    /// List of signatories who have signed this proposal
    pub signatures: Vec<Pubkey>,
    
    /// Status of the proposal
    pub status: ProposalStatus,
    
    /// Timestamp when proposal was created
    pub created_at: i64,
    
    /// Timestamp when proposal expires (optional)
    pub expires_at: Option<i64>,
    
    /// The account that created this proposal
    pub proposer: Pubkey,
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 64],
}

impl MultisigProposal {
    /// Maximum size of instruction data
    pub const MAX_INSTRUCTION_SIZE: usize = 1024;
    
    /// Account size calculation
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // multisig
        8 + // nonce
        1 + // operation_type
        4 + Self::MAX_INSTRUCTION_SIZE + // instruction_data
        4 + (MultiSig::MAX_SIGNATORIES * 32) + // signatures
        1 + // status
        8 + // created_at
        1 + 8 + // expires_at (Option<i64>)
        32 + // proposer
        64; // reserved

    /// Create a new proposal
    pub fn new(
        multisig: Pubkey,
        nonce: u64,
        operation_type: MultisigOperationType,
        instruction_data: Vec<u8>,
        proposer: Pubkey,
        expires_at: Option<i64>,
    ) -> Result<Self> {
        if instruction_data.len() > Self::MAX_INSTRUCTION_SIZE {
            return Err(LendingError::InstructionTooLarge.into());
        }
        
        let clock = Clock::get()?;
        Ok(Self {
            version: PROGRAM_VERSION,
            multisig,
            nonce,
            operation_type,
            instruction_data,
            signatures: Vec::new(),
            status: ProposalStatus::Active,
            created_at: clock.unix_timestamp,
            expires_at,
            proposer,
            reserved: [0; 64],
        })
    }
    
    /// Add a signature to the proposal
    pub fn add_signature(&mut self, signatory: &Pubkey) -> Result<()> {
        if self.signatures.contains(signatory) {
            return Err(LendingError::AlreadySigned.into());
        }
        
        self.signatures.push(*signatory);
        Ok(())
    }
    
    /// Check if proposal has enough signatures
    pub fn has_enough_signatures(&self, threshold: u8) -> bool {
        self.signatures.len() >= threshold as usize
    }
    
    /// Check if proposal is expired
    pub fn is_expired(&self) -> Result<bool> {
        if let Some(expires_at) = self.expires_at {
            let clock = Clock::get()?;
            Ok(clock.unix_timestamp > expires_at)
        } else {
            Ok(false)
        }
    }
    
    /// Mark proposal as executed
    pub fn mark_executed(&mut self) -> Result<()> {
        if self.status != ProposalStatus::Active {
            return Err(LendingError::ProposalNotActive.into());
        }
        
        self.status = ProposalStatus::Executed;
        Ok(())
    }
    
    /// Mark proposal as cancelled
    pub fn mark_cancelled(&mut self) -> Result<()> {
        if self.status != ProposalStatus::Active {
            return Err(LendingError::ProposalNotActive.into());
        }
        
        self.status = ProposalStatus::Cancelled;
        Ok(())
    }
}

/// Types of operations that can be performed via multisig
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum MultisigOperationType {
    /// Update market owner
    UpdateMarketOwner,
    /// Update emergency authority
    UpdateEmergencyAuthority,
    /// Update reserve configuration
    UpdateReserveConfig,
    /// Initialize new reserve
    InitializeReserve,
    /// Update oracle configuration
    UpdateOracleConfig,
    /// Change multisig configuration
    UpdateMultisigConfig,
    /// Execute emergency action
    EmergencyAction,
    /// Withdraw protocol fees
    WithdrawFees,
}

impl Default for MultisigOperationType {
    fn default() -> Self {
        Self::UpdateMarketOwner
    }
}

/// Status of a multisig proposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Proposal is active and can be signed/executed
    Active,
    /// Proposal has been executed
    Executed,
    /// Proposal has been cancelled
    Cancelled,
    /// Proposal has expired
    Expired,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Parameters for initializing a multisig
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeMultisigParams {
    pub signatories: Vec<Pubkey>,
    pub threshold: u8,
}

/// Parameters for creating a multisig proposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateProposalParams {
    pub operation_type: MultisigOperationType,
    pub instruction_data: Vec<u8>,
    pub expires_at: Option<i64>,
}