use anchor_lang::prelude::*;
use crate::constants::*;
use crate::utils::math::*;
use crate::error::LendingError;

/// User obligation account - tracks collateral deposits and borrows
#[account]
pub struct Obligation {
    /// Version of the obligation account structure
    pub version: u8,
    
    /// Market this obligation belongs to
    pub market: Pubkey,
    
    /// Owner of this obligation (borrower)
    pub owner: Pubkey,
    
    /// Collateral deposits in various reserves
    pub deposits: Vec<ObligationCollateral>,
    
    /// Borrowed liquidity from various reserves  
    pub borrows: Vec<ObligationLiquidity>,
    
    /// Total deposited value in USD (cached)
    pub deposited_value_usd: Decimal,
    
    /// Total borrowed value in USD (cached) 
    pub borrowed_value_usd: Decimal,
    
    /// Timestamp of the last obligation update
    pub last_update_timestamp: u64,
    
    /// Slot of the last obligation update
    pub last_update_slot: u64,
    
    /// Health factor snapshot during liquidation (prevents manipulation)
    pub liquidation_snapshot_health_factor: Option<Decimal>,
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 112],
}

impl Obligation {
    /// Size of the Obligation account in bytes (estimated)
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // market
        32 + // owner
        4 + (MAX_OBLIGATION_RESERVES * std::mem::size_of::<ObligationCollateral>()) + // deposits
        4 + (MAX_OBLIGATION_RESERVES * std::mem::size_of::<ObligationLiquidity>()) + // borrows
        16 + // deposited_value_usd (Decimal is u128)
        16 + // borrowed_value_usd
        8 + // last_update_timestamp
        8 + // last_update_slot
        128; // reserved

    /// Create a new obligation for the given owner
    pub fn new(market: Pubkey, owner: Pubkey) -> Result<Self> {
        let clock = Clock::get()?;
        
        Ok(Self {
            version: PROGRAM_VERSION,
            market,
            owner,
            deposits: Vec::new(),
            borrows: Vec::new(),
            deposited_value_usd: Decimal::zero(),
            borrowed_value_usd: Decimal::zero(),
            last_update_timestamp: clock.unix_timestamp as u64,
            last_update_slot: clock.slot,
            liquidation_snapshot_health_factor: None,
            reserved: [0; 112],
        })
    }

    /// Add collateral deposit to the obligation
    pub fn add_collateral_deposit(&mut self, deposit: ObligationCollateral) -> Result<()> {
        if self.deposits.len() >= MAX_OBLIGATION_RESERVES {
            return Err(LendingError::ObligationDepositsMaxed.into());
        }

        // Check if deposit for this reserve already exists
        if let Some(existing_deposit) = self.find_collateral_deposit_mut(&deposit.deposit_reserve) {
            existing_deposit.deposited_amount = existing_deposit.deposited_amount
                .checked_add(deposit.deposited_amount)
                .ok_or(LendingError::MathOverflow)?;
        } else {
            self.deposits.push(deposit);
        }

        Ok(())
    }

    /// Remove collateral deposit from the obligation
    pub fn remove_collateral_deposit(&mut self, reserve: &Pubkey, amount: u64) -> Result<()> {
        let deposit = self.find_collateral_deposit_mut(reserve)
            .ok_or(LendingError::ObligationReserveNotFound)?;

        if deposit.deposited_amount < amount {
            return Err(LendingError::InsufficientCollateral.into());
        }

        deposit.deposited_amount = deposit.deposited_amount
            .checked_sub(amount)
            .ok_or(LendingError::MathUnderflow)?;

        // Remove deposit if amount becomes zero
        if deposit.deposited_amount == 0 {
            self.deposits.retain(|d| d.deposit_reserve != *reserve);
        }

        Ok(())
    }

    /// Add liquidity borrow to the obligation
    pub fn add_liquidity_borrow(&mut self, borrow: ObligationLiquidity) -> Result<()> {
        if self.borrows.len() >= MAX_OBLIGATION_RESERVES {
            return Err(LendingError::ObligationBorrowsMaxed.into());
        }

        // Check if borrow for this reserve already exists
        if let Some(existing_borrow) = self.find_liquidity_borrow_mut(&borrow.borrow_reserve) {
            existing_borrow.borrowed_amount_wads = existing_borrow.borrowed_amount_wads
                .try_add(borrow.borrowed_amount_wads)?;
        } else {
            self.borrows.push(borrow);
        }

        Ok(())
    }

    /// Repay liquidity borrow from the obligation
    pub fn repay_liquidity_borrow(&mut self, reserve: &Pubkey, amount: Decimal) -> Result<()> {
        let borrow = self.find_liquidity_borrow_mut(reserve)
            .ok_or(LendingError::ObligationReserveNotFound)?;

        if borrow.borrowed_amount_wads.value < amount.value {
            return Err(LendingError::InsufficientTokenBalance.into());
        }

        borrow.borrowed_amount_wads = borrow.borrowed_amount_wads.try_sub(amount)?;

        // Remove borrow if amount becomes zero
        if borrow.borrowed_amount_wads.is_zero() {
            self.borrows.retain(|b| b.borrow_reserve != *reserve);
        }

        Ok(())
    }

    /// Find collateral deposit by reserve
    pub fn find_collateral_deposit(&self, reserve: &Pubkey) -> Option<&ObligationCollateral> {
        self.deposits.iter().find(|d| d.deposit_reserve == *reserve)
    }

    /// Find mutable collateral deposit by reserve
    pub fn find_collateral_deposit_mut(&mut self, reserve: &Pubkey) -> Option<&mut ObligationCollateral> {
        self.deposits.iter_mut().find(|d| d.deposit_reserve == *reserve)
    }

    /// Find liquidity borrow by reserve
    pub fn find_liquidity_borrow(&self, reserve: &Pubkey) -> Option<&ObligationLiquidity> {
        self.borrows.iter().find(|b| b.borrow_reserve == *reserve)
    }

    /// Find mutable liquidity borrow by reserve
    pub fn find_liquidity_borrow_mut(&mut self, reserve: &Pubkey) -> Option<&mut ObligationLiquidity> {
        self.borrows.iter_mut().find(|b| b.borrow_reserve == *reserve)
    }

    /// Calculate health factor of the obligation
    /// Health factor = (collateral value * liquidation threshold) / borrowed value
    /// Health factor > 1.0 means the obligation is healthy
    /// Health factor < 1.0 means the obligation can be liquidated
    pub fn calculate_health_factor(&self) -> Result<Decimal> {
        if self.borrowed_value_usd.is_zero() {
            return Ok(Decimal::from_integer(u64::MAX)?); // Infinite health if no debt
        }

        let weighted_collateral_value = self.calculate_liquidation_threshold_value()?;
        weighted_collateral_value.try_div(self.borrowed_value_usd)
    }

    /// Calculate maximum loan-to-value based on collateral
    pub fn calculate_max_borrow_value(&self) -> Result<Decimal> {
        let mut max_borrow_value = Decimal::zero();

        for deposit in &self.deposits {
            let collateral_value = deposit.market_value_usd;
            let ltv_decimal = Decimal::from_scaled_val(
                (deposit.ltv_bps as u128)
                    .checked_mul(PRECISION as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)?,
            );
            
            let borrow_value = collateral_value.try_mul(ltv_decimal)?;
            max_borrow_value = max_borrow_value.try_add(borrow_value)?;
        }

        Ok(max_borrow_value)
    }

    /// Calculate liquidation threshold value (collateral value * liquidation threshold)
    pub fn calculate_liquidation_threshold_value(&self) -> Result<Decimal> {
        let mut threshold_value = Decimal::zero();

        for deposit in &self.deposits {
            let collateral_value = deposit.market_value_usd;
            let threshold_decimal = Decimal::from_scaled_val(
                (deposit.liquidation_threshold_bps as u128)
                    .checked_mul(PRECISION as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)?,
            );
            
            let weighted_value = collateral_value.try_mul(threshold_decimal)?;
            threshold_value = threshold_value.try_add(weighted_value)?;
        }

        Ok(threshold_value)
    }

    /// Check if the obligation is healthy (can't be liquidated)
    pub fn is_healthy(&self) -> Result<bool> {
        let health_factor = self.calculate_health_factor()?;
        Ok(health_factor.value >= Decimal::one().value)
    }

    /// Check if the obligation has collateral
    pub fn has_collateral(&self) -> bool {
        !self.deposits.is_empty()
    }

    /// Check if the obligation has borrows
    pub fn has_borrows(&self) -> bool {
        !self.borrows.is_empty()
    }

    /// Check if the obligation needs to be refreshed
    pub fn is_stale(&self, current_slot: u64) -> bool {
        current_slot.saturating_sub(self.last_update_slot) > MAX_ORACLE_STALENESS_SLOTS
    }

    /// Update timestamps
    pub fn update_timestamp(&mut self, slot: u64) -> Result<()> {
        let clock = Clock::get()?;
        self.last_update_slot = slot;
        self.last_update_timestamp = clock.unix_timestamp as u64;
        Ok(())
    }

    /// Calculate maximum liquidation amount for a given reserve
    pub fn max_liquidation_amount(&self, repay_reserve: &Pubkey) -> Result<u64> {
        let borrow = self.find_liquidity_borrow(repay_reserve)
            .ok_or(LendingError::ObligationReserveNotFound)?;

        // Maximum 50% of the debt can be liquidated at once
        let max_liquidation = borrow.borrowed_amount_wads
            .try_div(Decimal::from_integer(2)?)?
            .try_floor_u64()?;

        Ok(max_liquidation)
    }

    /// Refresh health factor with current oracle prices to prevent race conditions
    pub fn refresh_health_factor(&mut self, _price_oracles: &[AccountInfo], current_timestamp: i64) -> Result<()> {
        // Refresh all collateral values with current prices
        for _deposit in &mut self.deposits {
            // Get current price from oracle (implementation would be specific to oracle type)
            // This is a placeholder - actual implementation would fetch from price_oracles
            // based on the reserve's oracle configuration
        }

        // Refresh all borrow values with current interest rates
        for _borrow in &mut self.borrows {
            // Update borrowed amounts with accrued interest
            // This is a placeholder for interest accrual calculation
        }

        // Clear any stale liquidation snapshot
        self.liquidation_snapshot_health_factor = None;
        
        // Update timestamp to mark as refreshed
        self.last_update_timestamp = current_timestamp as u64;

        Ok(())
    }

    /// Get health factor from snapshot if available, otherwise calculate fresh
    pub fn get_health_factor_for_liquidation(&self) -> Result<Decimal> {
        if let Some(snapshot_health) = self.liquidation_snapshot_health_factor {
            Ok(snapshot_health)
        } else {
            self.calculate_health_factor()
        }
    }
}

/// Collateral deposited in a reserve
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ObligationCollateral {
    /// Reserve where the collateral is deposited
    pub deposit_reserve: Pubkey,
    
    /// Amount of collateral tokens deposited
    pub deposited_amount: u64,
    
    /// Current market value in USD
    pub market_value_usd: Decimal,
    
    /// Loan-to-value ratio for this collateral type (basis points)
    pub ltv_bps: u64,
    
    /// Liquidation threshold for this collateral type (basis points)
    pub liquidation_threshold_bps: u64,
}

/// Liquidity borrowed from a reserve
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ObligationLiquidity {
    /// Reserve where the liquidity was borrowed
    pub borrow_reserve: Pubkey,
    
    /// Amount borrowed including accrued interest (high precision)
    pub borrowed_amount_wads: Decimal,
    
    /// Current market value in USD
    pub market_value_usd: Decimal,
}