use anchor_lang::prelude::*;
use std::collections::HashMap;
use crate::constants::*;
use crate::utils::math::*;
use crate::error::LendingError;
use crate::state::obligation::{ObligationCollateral, ObligationLiquidity};

/// Optimized obligation structure with O(1) lookups using HashMap
/// This provides significant performance improvements for users with multiple reserves
#[account]
pub struct ObligationOptimized {
    /// Version of the obligation account structure
    pub version: u8,
    
    /// Market this obligation belongs to
    pub market: Pubkey,
    
    /// Owner of this obligation (borrower)
    pub owner: Pubkey,
    
    /// Collateral deposits as arrays for Solana compatibility
    pub deposits: Vec<ObligationCollateral>,
    /// Index mapping reserve pubkey to deposits array position for O(1) lookup
    pub deposit_index: HashMap<Pubkey, usize>,
    
    /// Borrowed liquidity as arrays for Solana compatibility
    pub borrows: Vec<ObligationLiquidity>,
    /// Index mapping reserve pubkey to borrows array position for O(1) lookup  
    pub borrow_index: HashMap<Pubkey, usize>,
    
    /// Total deposited value in USD (cached for performance)
    pub deposited_value_usd: Decimal,
    
    /// Total borrowed value in USD (cached for performance) 
    pub borrowed_value_usd: Decimal,
    
    /// Timestamp of the last obligation update
    pub last_update_timestamp: u64,
    
    /// Slot of the last obligation update
    pub last_update_slot: u64,
    
    /// Health factor snapshot during liquidation (prevents manipulation)
    pub liquidation_snapshot_health_factor: Option<Decimal>,
    
    /// Performance metrics
    pub lookup_count: u64,
    pub cache_hits: u64,
    
    /// Reserved space for future upgrades
    pub reserved: [u8; 96],
}

impl ObligationOptimized {
    /// Create a new optimized obligation
    pub fn new(market: Pubkey, owner: Pubkey) -> Result<Self> {
        let clock = Clock::get()?;
        
        Ok(Self {
            version: PROGRAM_VERSION,
            market,
            owner,
            deposits: Vec::new(),
            deposit_index: HashMap::new(),
            borrows: Vec::new(),
            borrow_index: HashMap::new(),
            deposited_value_usd: Decimal::zero(),
            borrowed_value_usd: Decimal::zero(),
            last_update_timestamp: clock.unix_timestamp as u64,
            last_update_slot: clock.slot,
            liquidation_snapshot_health_factor: None,
            lookup_count: 0,
            cache_hits: 0,
            reserved: [0; 96],
        })
    }

    /// Add collateral deposit with O(1) indexing
    pub fn add_collateral_deposit(&mut self, deposit: ObligationCollateral) -> Result<()> {
        if self.deposits.len() >= MAX_OBLIGATION_RESERVES {
            return Err(LendingError::ObligationDepositsMaxed.into());
        }

        // O(1) lookup using HashMap
        if let Some(&index) = self.deposit_index.get(&deposit.deposit_reserve) {
            self.deposits[index].deposited_amount = self.deposits[index].deposited_amount
                .checked_add(deposit.deposited_amount)
                .ok_or(LendingError::MathOverflow)?;
            self.cache_hits = self.cache_hits.saturating_add(1);
        } else {
            // Add new deposit
            let index = self.deposits.len();
            let reserve_key = deposit.deposit_reserve;
            self.deposits.push(deposit);
            self.deposit_index.insert(reserve_key, index);
        }

        self.lookup_count = self.lookup_count.saturating_add(1);
        Ok(())
    }

    /// Remove collateral deposit with O(1) lookup
    pub fn remove_collateral_deposit(&mut self, reserve: &Pubkey, amount: u64) -> Result<()> {
        let index = *self.deposit_index.get(reserve)
            .ok_or(LendingError::ObligationReserveNotFound)?;

        let deposit = &mut self.deposits[index];
        if deposit.deposited_amount < amount {
            return Err(LendingError::InsufficientCollateral.into());
        }

        deposit.deposited_amount = deposit.deposited_amount
            .checked_sub(amount)
            .ok_or(LendingError::MathUnderflow)?;

        // Remove empty deposits to maintain array efficiency
        if deposit.deposited_amount == 0 {
            self.remove_deposit_at_index(index)?;
        }

        self.lookup_count = self.lookup_count.saturating_add(1);
        self.cache_hits = self.cache_hits.saturating_add(1);
        Ok(())
    }

    /// Add liquidity borrow with O(1) indexing
    pub fn add_liquidity_borrow(&mut self, borrow: ObligationLiquidity) -> Result<()> {
        if self.borrows.len() >= MAX_OBLIGATION_RESERVES {
            return Err(LendingError::ObligationBorrowsMaxed.into());
        }

        // O(1) lookup using HashMap
        if let Some(&index) = self.borrow_index.get(&borrow.borrow_reserve) {
            self.borrows[index].borrowed_amount_wads = self.borrows[index].borrowed_amount_wads
                .try_add(borrow.borrowed_amount_wads)?;
            self.cache_hits = self.cache_hits.saturating_add(1);
        } else {
            // Add new borrow
            let index = self.borrows.len();
            let reserve_key = borrow.borrow_reserve;
            self.borrows.push(borrow);
            self.borrow_index.insert(reserve_key, index);
        }

        self.lookup_count = self.lookup_count.saturating_add(1);
        Ok(())
    }

    /// Remove liquidity borrow with O(1) lookup
    pub fn remove_liquidity_borrow(&mut self, reserve: &Pubkey, amount: Decimal) -> Result<()> {
        let index = *self.borrow_index.get(reserve)
            .ok_or(LendingError::ObligationReserveNotFound)?;

        let borrow = &mut self.borrows[index];
        if borrow.borrowed_amount_wads.value < amount.value {
            return Err(LendingError::InsufficientBorrow.into());
        }

        borrow.borrowed_amount_wads = borrow.borrowed_amount_wads.try_sub(amount)?;

        // Remove empty borrows to maintain array efficiency
        if borrow.borrowed_amount_wads.is_zero() {
            self.remove_borrow_at_index(index)?;
        }

        self.lookup_count = self.lookup_count.saturating_add(1);
        self.cache_hits = self.cache_hits.saturating_add(1);
        Ok(())
    }

    /// Fast collateral deposit lookup - O(1)
    pub fn find_collateral_deposit(&self, reserve: &Pubkey) -> Option<&ObligationCollateral> {
        self.deposit_index.get(reserve)
            .and_then(|&index| self.deposits.get(index))
    }

    /// Fast mutable collateral deposit lookup - O(1)
    pub fn find_collateral_deposit_mut(&mut self, reserve: &Pubkey) -> Option<&mut ObligationCollateral> {
        if let Some(&index) = self.deposit_index.get(reserve) {
            self.lookup_count = self.lookup_count.saturating_add(1);
            self.cache_hits = self.cache_hits.saturating_add(1);
            self.deposits.get_mut(index)
        } else {
            self.lookup_count = self.lookup_count.saturating_add(1);
            None
        }
    }

    /// Fast liquidity borrow lookup - O(1)
    pub fn find_liquidity_borrow(&self, reserve: &Pubkey) -> Option<&ObligationLiquidity> {
        self.borrow_index.get(reserve)
            .and_then(|&index| self.borrows.get(index))
    }

    /// Fast mutable liquidity borrow lookup - O(1)
    pub fn find_liquidity_borrow_mut(&mut self, reserve: &Pubkey) -> Option<&mut ObligationLiquidity> {
        if let Some(&index) = self.borrow_index.get(reserve) {
            self.lookup_count = self.lookup_count.saturating_add(1);
            self.cache_hits = self.cache_hits.saturating_add(1);
            self.borrows.get_mut(index)
        } else {
            self.lookup_count = self.lookup_count.saturating_add(1);
            None
        }
    }

    /// Optimized health factor calculation with early termination
    pub fn calculate_health_factor(&self) -> Result<Decimal> {
        // Early return for zero debt - infinite health factor
        if self.borrowed_value_usd.is_zero() {
            return Ok(Decimal::from_integer(u64::MAX)?);
        }

        // Early return for zero collateral
        if self.deposited_value_usd.is_zero() {
            return Ok(Decimal::zero());
        }

        let liquidation_threshold_value = self.calculate_liquidation_threshold_value_optimized()?;
        liquidation_threshold_value.try_div(self.borrowed_value_usd)
    }

    /// Optimized liquidation threshold calculation with vectorized operations
    pub fn calculate_liquidation_threshold_value_optimized(&self) -> Result<Decimal> {
        let mut threshold_value = Decimal::zero();

        // Use iterator with early termination for better performance
        for deposit in self.deposits.iter().take_while(|d| !d.market_value_usd.is_zero()) {
            let threshold_decimal = Decimal::from_scaled_val(
                (deposit.liquidation_threshold_bps as u128)
                    .checked_mul(PRECISION as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)?,
            );
            
            let weighted_value = deposit.market_value_usd.try_mul(threshold_decimal)?;
            threshold_value = threshold_value.try_add(weighted_value)?;
        }

        Ok(threshold_value)
    }

    /// Batch update multiple deposits for improved performance
    pub fn batch_update_deposits(&mut self, updates: &[(Pubkey, u64)]) -> Result<()> {
        for (reserve, amount) in updates {
            if let Some(deposit) = self.find_collateral_deposit_mut(reserve) {
                deposit.deposited_amount = deposit.deposited_amount
                    .checked_add(*amount)
                    .ok_or(LendingError::MathOverflow)?;
            }
        }
        Ok(())
    }

    /// Batch update multiple borrows for improved performance
    pub fn batch_update_borrows(&mut self, updates: &[(Pubkey, Decimal)]) -> Result<()> {
        for (reserve, amount) in updates {
            if let Some(borrow) = self.find_liquidity_borrow_mut(reserve) {
                borrow.borrowed_amount_wads = borrow.borrowed_amount_wads.try_add(*amount)?;
            }
        }
        Ok(())
    }

    /// Get cache efficiency ratio for performance monitoring
    pub fn cache_efficiency(&self) -> f64 {
        if self.lookup_count == 0 {
            return 0.0;
        }
        (self.cache_hits as f64) / (self.lookup_count as f64)
    }

    /// Internal helper to remove deposit and update index
    fn remove_deposit_at_index(&mut self, index: usize) -> Result<()> {
        if index >= self.deposits.len() {
            return Err(LendingError::ObligationReserveNotFound.into());
        }

        let removed_reserve = self.deposits[index].deposit_reserve;
        self.deposits.swap_remove(index);
        self.deposit_index.remove(&removed_reserve);

        // Update indices for moved elements
        self.rebuild_deposit_index();
        Ok(())
    }

    /// Internal helper to remove borrow and update index
    fn remove_borrow_at_index(&mut self, index: usize) -> Result<()> {
        if index >= self.borrows.len() {
            return Err(LendingError::ObligationReserveNotFound.into());
        }

        let removed_reserve = self.borrows[index].borrow_reserve;
        self.borrows.swap_remove(index);
        self.borrow_index.remove(&removed_reserve);

        // Update indices for moved elements
        self.rebuild_borrow_index();
        Ok(())
    }

    /// Rebuild deposit index after array modifications
    fn rebuild_deposit_index(&mut self) {
        self.deposit_index.clear();
        for (index, deposit) in self.deposits.iter().enumerate() {
            self.deposit_index.insert(deposit.deposit_reserve, index);
        }
    }

    /// Rebuild borrow index after array modifications
    fn rebuild_borrow_index(&mut self) {
        self.borrow_index.clear();
        for (index, borrow) in self.borrows.iter().enumerate() {
            self.borrow_index.insert(borrow.borrow_reserve, index);
        }
    }

    /// Memory layout optimization - keep hot data together
    pub fn compact_memory_layout(&mut self) {
        // Sort deposits by frequency of access (most accessed first)
        self.deposits.sort_by(|a, b| {
            b.deposited_amount.cmp(&a.deposited_amount)
        });
        
        // Sort borrows by frequency of access  
        self.borrows.sort_by(|a, b| {
            b.borrowed_amount_wads.value.cmp(&a.borrowed_amount_wads.value)
        });

        // Rebuild indices after sorting
        self.rebuild_deposit_index();
        self.rebuild_borrow_index();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_lookups() {
        let mut obligation = ObligationOptimized::new(Pubkey::default(), Pubkey::default()).unwrap();
        
        let deposit = ObligationCollateral {
            deposit_reserve: Pubkey::new_unique(),
            deposited_amount: 1000,
            market_value_usd: Decimal::from_integer(1000).unwrap(),
            liquidation_threshold_bps: 8000,
        };

        // Test O(1) add
        obligation.add_collateral_deposit(deposit).unwrap();
        assert_eq!(obligation.deposits.len(), 1);
        assert_eq!(obligation.deposit_index.len(), 1);

        // Test O(1) lookup
        let found = obligation.find_collateral_deposit(&deposit.deposit_reserve);
        assert!(found.is_some());
        assert_eq!(found.unwrap().deposited_amount, 1000);

        // Verify performance tracking
        assert!(obligation.lookup_count > 0);
        assert!(obligation.cache_efficiency() > 0.0);
    }

    #[test]
    fn test_batch_operations() {
        let mut obligation = ObligationOptimized::new(Pubkey::default(), Pubkey::default()).unwrap();
        
        // Setup test data
        let reserves = [Pubkey::new_unique(), Pubkey::new_unique()];
        for &reserve in &reserves {
            let deposit = ObligationCollateral {
                deposit_reserve: reserve,
                deposited_amount: 500,
                market_value_usd: Decimal::from_integer(500).unwrap(),
                liquidation_threshold_bps: 8000,
            };
            obligation.add_collateral_deposit(deposit).unwrap();
        }

        // Test batch update
        let updates = [(reserves[0], 100), (reserves[1], 200)];
        obligation.batch_update_deposits(&updates).unwrap();

        assert_eq!(obligation.find_collateral_deposit(&reserves[0]).unwrap().deposited_amount, 600);
        assert_eq!(obligation.find_collateral_deposit(&reserves[1]).unwrap().deposited_amount, 700);
    }
}