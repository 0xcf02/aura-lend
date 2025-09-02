use anchor_lang::prelude::*;
use crate::constants::*;

/// Global market state account
/// This account contains the overall configuration and state of the lending protocol
#[account]
#[derive(Default)]
pub struct Market {
    /// Version of the market account structure
    pub version: u8,
    
    /// The authority that can modify market parameters
    pub owner: Pubkey,
    
    /// Emergency authority that can pause the protocol
    pub emergency_authority: Pubkey,
    
    /// Quote currency (typically USDC) mint for price calculations
    pub quote_currency: Pubkey,
    
    /// Token mint for the AURA governance token
    pub aura_token_mint: Pubkey,
    
    /// Authority for minting AURA tokens (rewards distributor PDA)
    pub aura_mint_authority: Pubkey,
    
    /// Total number of reserves initialized in this market
    pub reserves_count: u64,
    
    /// Fees collected by the protocol (in quote currency)
    pub total_fees_collected: u64,
    
    /// Timestamp of the last market state update
    pub last_update_timestamp: u64,
    
    /// Global protocol flags
    pub flags: MarketFlags,
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 256],
}

impl Market {
    /// Size of the Market account in bytes
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // owner
        32 + // emergency_authority  
        32 + // quote_currency
        32 + // aura_token_mint
        32 + // aura_mint_authority
        8 + // reserves_count
        8 + // total_fees_collected
        8 + // last_update_timestamp
        32 + // flags (MarketFlags is u32, but we use 32 bytes for alignment)
        256; // reserved

    /// Create a new market with the given parameters
    pub fn new(
        owner: Pubkey,
        emergency_authority: Pubkey,
        quote_currency: Pubkey,
        aura_token_mint: Pubkey,
        aura_mint_authority: Pubkey,
    ) -> Result<Self> {
        let clock = Clock::get()?;
        Ok(Self {
            version: PROGRAM_VERSION,
            owner,
            emergency_authority,
            quote_currency,
            aura_token_mint,
            aura_mint_authority,
            reserves_count: 0,
            total_fees_collected: 0,
            last_update_timestamp: clock.unix_timestamp as u64,
            flags: MarketFlags::default(),
            reserved: [0; 256],
        })
    }

    /// Check if the market is paused
    pub fn is_paused(&self) -> bool {
        self.flags.contains(MarketFlags::PAUSED)
    }

    /// Check if emergency mode is active
    pub fn is_emergency(&self) -> bool {
        self.flags.contains(MarketFlags::EMERGENCY)
    }

    /// Check if lending is disabled
    pub fn is_lending_disabled(&self) -> bool {
        self.flags.contains(MarketFlags::LENDING_DISABLED)
    }

    /// Check if borrowing is disabled
    pub fn is_borrowing_disabled(&self) -> bool {
        self.flags.contains(MarketFlags::BORROWING_DISABLED)
    }

    /// Check if liquidations are disabled
    pub fn is_liquidation_disabled(&self) -> bool {
        self.flags.contains(MarketFlags::LIQUIDATION_DISABLED)
    }

    /// Update the market timestamp
    pub fn update_timestamp(&mut self) -> Result<()> {
        let clock = Clock::get()?;
        self.last_update_timestamp = clock.unix_timestamp as u64;
        Ok(())
    }

    /// Add fees to the total collected
    pub fn add_fees(&mut self, fee_amount: u64) -> Result<()> {
        self.total_fees_collected = self.total_fees_collected
            .checked_add(fee_amount)
            .ok_or(crate::error::LendingError::MathOverflow)?;
        Ok(())
    }

    /// Increment the reserves count
    pub fn increment_reserves_count(&mut self) -> Result<()> {
        if self.reserves_count >= MAX_RESERVES as u64 {
            return Err(crate::error::LendingError::InvalidReserveConfig.into());
        }
        self.reserves_count = self.reserves_count
            .checked_add(1)
            .ok_or(crate::error::LendingError::MathOverflow)?;
        Ok(())
    }
}

/// Market configuration flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct MarketFlags {
    bits: u32,
}

impl MarketFlags {
    /// Market is paused - no operations allowed
    pub const PAUSED: Self = Self { bits: 1 << 0 };
    
    /// Emergency mode - only withdrawals and liquidations allowed
    pub const EMERGENCY: Self = Self { bits: 1 << 1 };
    
    /// Lending is disabled - no new deposits
    pub const LENDING_DISABLED: Self = Self { bits: 1 << 2 };
    
    /// Borrowing is disabled - no new borrows
    pub const BORROWING_DISABLED: Self = Self { bits: 1 << 3 };
    
    /// Liquidations are disabled
    pub const LIQUIDATION_DISABLED: Self = Self { bits: 1 << 4 };

    /// Create empty flags
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Check if flags contain a specific flag
    pub fn contains(&self, flag: Self) -> bool {
        (self.bits & flag.bits) == flag.bits
    }

    /// Add a flag
    pub fn insert(&mut self, flag: Self) {
        self.bits |= flag.bits;
    }

    /// Remove a flag  
    pub fn remove(&mut self, flag: Self) {
        self.bits &= !flag.bits;
    }

    /// Toggle a flag
    pub fn toggle(&mut self, flag: Self) {
        self.bits ^= flag.bits;
    }
}

impl Default for MarketFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Parameters for initializing a market
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeMarketParams {
    pub owner: Pubkey,
    pub emergency_authority: Pubkey,
    pub quote_currency: Pubkey,
    pub aura_token_mint: Pubkey,
}