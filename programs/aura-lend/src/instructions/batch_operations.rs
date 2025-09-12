use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::utils::math::Decimal;
use crate::state::obligation::{ObligationCollateral, ObligationLiquidity};
use crate::state::obligation_optimized::ObligationOptimized;
use crate::constants::*;
use std::collections::HashMap;

/// Batch operation types for optimized processing
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum BatchOperationType {
    UpdateCollateral,
    UpdateBorrow,
    UpdateHealthFactors,
    LiquidationCheck,
    InterestAccrual,
}

/// Single operation in a batch
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BatchOperation {
    pub operation_type: BatchOperationType,
    pub obligation_key: Pubkey,
    pub reserve_key: Option<Pubkey>,
    pub amount: Option<u64>,
    pub decimal_amount: Option<Decimal>,
}

/// Result of a batch operation
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BatchOperationResult {
    pub operation_id: u32,
    pub success: bool,
    pub error_code: Option<u32>,
    pub gas_used: u64,
}

/// Batch operation context for tracking performance
#[derive(Debug)]
pub struct BatchContext {
    pub start_time: i64,
    pub operations_processed: u32,
    pub operations_failed: u32,
    pub total_gas_used: u64,
    pub cache_hits: u32,
    pub cache_misses: u32,
}

impl BatchContext {
    pub fn new() -> Self {
        Self {
            start_time: Clock::get().unwrap().unix_timestamp,
            operations_processed: 0,
            operations_failed: 0,
            total_gas_used: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn record_operation(&mut self, success: bool, gas_used: u64) {
        self.operations_processed += 1;
        if !success {
            self.operations_failed += 1;
        }
        self.total_gas_used += gas_used;
    }
}

/// Batch processor for efficient multi-operation handling
pub struct BatchProcessor {
    /// Cache for frequently accessed obligations
    obligation_cache: std::collections::HashMap<Pubkey, ObligationOptimized>,
    /// Maximum batch size to prevent excessive gas usage
    max_batch_size: usize,
    /// Statistics for performance monitoring
    stats: BatchContext,
}

impl BatchProcessor {
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            obligation_cache: std::collections::HashMap::new(),
            max_batch_size,
            stats: BatchContext::new(),
        }
    }

    /// Process multiple operations in a single transaction for efficiency
    pub fn process_batch_operations(
        &mut self,
        operations: &[BatchOperation],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        if operations.len() > self.max_batch_size {
            return Err(LendingError::BatchSizeExceeded.into());
        }

        let mut results = Vec::with_capacity(operations.len());
        
        // Group operations by type for better cache locality
        let grouped_ops = self.group_operations_by_type(operations);
        
        // Process each group to maximize cache reuse
        for (op_type, ops) in grouped_ops.into_iter() {
            let group_results = self.process_operation_group(&op_type, &ops, accounts)?;
            results.extend(group_results);
        }

        Ok(results)
    }

    /// Group operations by type for better processing efficiency
    fn group_operations_by_type(
        &self,
        operations: &[BatchOperation],
    ) -> std::collections::HashMap<BatchOperationType, Vec<(usize, &BatchOperation)>> {
        let mut grouped = std::collections::HashMap::new();
        
        for (index, op) in operations.iter().enumerate() {
            grouped
                .entry(op.operation_type.clone())
                .or_insert_with(Vec::new)
                .push((index, op));
        }
        
        grouped
    }

    /// Process a group of operations of the same type
    fn process_operation_group(
        &mut self,
        op_type: &BatchOperationType,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        match op_type {
            BatchOperationType::UpdateCollateral => {
                results = self.batch_update_collateral(operations, accounts)?;
            }
            BatchOperationType::UpdateBorrow => {
                results = self.batch_update_borrows(operations, accounts)?;
            }
            BatchOperationType::UpdateHealthFactors => {
                results = self.batch_update_health_factors(operations, accounts)?;
            }
            BatchOperationType::LiquidationCheck => {
                results = self.batch_liquidation_check(operations, accounts)?;
            }
            BatchOperationType::InterestAccrual => {
                results = self.batch_interest_accrual(operations, accounts)?;
            }
        }
        
        Ok(results)
    }

    /// Batch update collateral positions - O(k log n) where k is batch size
    fn batch_update_collateral(
        &mut self,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        // Pre-load all required obligations into cache
        self.preload_obligations(operations, accounts)?;
        
        for (op_index, operation) in operations {
            let start_gas = 0; // Would measure actual gas usage
            
            let result = self.update_single_collateral(operation, accounts);
            let success = result.is_ok();
            
            if let Err(e) = result {
                msg!("Collateral update failed for operation {}: {:?}", op_index, e);
            }
            
            let gas_used = 1000; // Would calculate actual gas usage
            self.stats.record_operation(success, gas_used);
            
            results.push(BatchOperationResult {
                operation_id: *op_index as u32,
                success,
                error_code: if success { None } else { Some(1001) },
                gas_used,
            });
        }
        
        Ok(results)
    }

    /// Batch update borrow positions
    fn batch_update_borrows(
        &mut self,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        self.preload_obligations(operations, accounts)?;
        
        for (op_index, operation) in operations {
            let start_gas = 0;
            
            let result = self.update_single_borrow(operation, accounts);
            let success = result.is_ok();
            
            let gas_used = 1200; // Borrows are slightly more expensive
            self.stats.record_operation(success, gas_used);
            
            results.push(BatchOperationResult {
                operation_id: *op_index as u32,
                success,
                error_code: if success { None } else { Some(1002) },
                gas_used,
            });
        }
        
        Ok(results)
    }

    /// Batch health factor updates - vectorized calculation
    fn batch_update_health_factors(
        &mut self,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        // Collect all obligation keys for batch processing
        let obligation_keys: Vec<Pubkey> = operations
            .iter()
            .map(|(_, op)| op.obligation_key)
            .collect();
        
        // Vectorized health factor calculation
        let health_factors = self.calculate_health_factors_vectorized(&obligation_keys, accounts)?;
        
        for ((op_index, operation), health_factor) in operations.iter().zip(health_factors.iter()) {
            let success = health_factor.is_some();
            let gas_used = 800; // Health factor calculation is relatively cheap when batched
            
            self.stats.record_operation(success, gas_used);
            
            results.push(BatchOperationResult {
                operation_id: *op_index as u32,
                success,
                error_code: if success { None } else { Some(1003) },
                gas_used,
            });
        }
        
        Ok(results)
    }

    /// Batch liquidation checks with early termination
    fn batch_liquidation_check(
        &mut self,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        // First pass: quick health factor screening
        let obligation_keys: Vec<Pubkey> = operations
            .iter()
            .map(|(_, op)| op.obligation_key)
            .collect();
        
        let health_factors = self.calculate_health_factors_vectorized(&obligation_keys, accounts)?;
        
        for ((op_index, _operation), health_factor) in operations.iter().zip(health_factors.iter()) {
            let success = health_factor.is_some();
            let is_liquidatable = health_factor
                .map(|hf| hf.value < Decimal::one().value)
                .unwrap_or(false);
            
            let gas_used = if is_liquidatable { 1500 } else { 500 }; // Liquidation prep is more expensive
            
            self.stats.record_operation(success, gas_used);
            
            results.push(BatchOperationResult {
                operation_id: *op_index as u32,
                success,
                error_code: if success { None } else { Some(1004) },
                gas_used,
            });
        }
        
        Ok(results)
    }

    /// Batch interest accrual - compound calculations
    fn batch_interest_accrual(
        &mut self,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        // Group by reserve for compound interest calculations
        let mut reserve_groups: std::collections::HashMap<Pubkey, Vec<(usize, &BatchOperation)>> = 
            std::collections::HashMap::new();
        
        for (op_index, operation) in operations {
            if let Some(reserve_key) = operation.reserve_key {
                reserve_groups
                    .entry(reserve_key)
                    .or_insert_with(Vec::new)
                    .push((*op_index, operation));
            }
        }
        
        // Process each reserve group for efficient interest calculation
        for (reserve_key, reserve_ops) in reserve_groups {
            let reserve_results = self.process_reserve_interest_batch(&reserve_key, &reserve_ops, accounts)?;
            results.extend(reserve_results);
        }
        
        Ok(results)
    }

    /// Process interest accrual for a single reserve batch
    fn process_reserve_interest_batch(
        &mut self,
        reserve_key: &Pubkey,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<Vec<BatchOperationResult>> {
        let mut results = Vec::new();
        
        // Get current interest rate for the reserve (would fetch from reserve account)
        let current_rate = Decimal::from_scaled_val(50000000000000000); // 5% APR example
        let time_delta = 3600; // 1 hour example
        
        for (op_index, operation) in operations {
            let start_gas = 0;
            
            // Apply compound interest to the position
            let result = self.apply_compound_interest(operation, current_rate, time_delta, accounts);
            let success = result.is_ok();
            
            let gas_used = 900; // Interest calculation gas cost
            self.stats.record_operation(success, gas_used);
            
            results.push(BatchOperationResult {
                operation_id: *op_index as u32,
                success,
                error_code: if success { None } else { Some(1005) },
                gas_used,
            });
        }
        
        Ok(results)
    }

    /// Preload obligations into cache for batch processing
    fn preload_obligations(
        &mut self,
        operations: &[(usize, &BatchOperation)],
        accounts: &[AccountInfo],
    ) -> Result<()> {
        let unique_obligations: std::collections::HashSet<Pubkey> = operations
            .iter()
            .map(|(_, op)| op.obligation_key)
            .collect();
        
        for obligation_key in unique_obligations {
            if !self.obligation_cache.contains_key(&obligation_key) {
                // Load obligation from account (would implement actual loading)
                let obligation = self.load_obligation_from_accounts(&obligation_key, accounts)?;
                self.obligation_cache.insert(obligation_key, obligation);
                self.stats.cache_misses += 1;
            } else {
                self.stats.cache_hits += 1;
            }
        }
        
        Ok(())
    }

    /// Vectorized health factor calculation for multiple obligations
    fn calculate_health_factors_vectorized(
        &self,
        obligation_keys: &[Pubkey],
        accounts: &[AccountInfo],
    ) -> Result<Vec<Option<Decimal>>> {
        let mut health_factors = Vec::with_capacity(obligation_keys.len());
        
        for &obligation_key in obligation_keys {
            if let Some(obligation) = self.obligation_cache.get(&obligation_key) {
                let health_factor = obligation.calculate_health_factor().ok();
                health_factors.push(health_factor);
            } else {
                health_factors.push(None);
            }
        }
        
        Ok(health_factors)
    }

    /// Helper functions for individual operations
    fn update_single_collateral(
        &mut self,
        operation: &BatchOperation,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        if let Some(obligation) = self.obligation_cache.get_mut(&operation.obligation_key) {
            if let (Some(reserve_key), Some(amount)) = (operation.reserve_key, operation.amount) {
                // Find and update collateral
                if let Some(collateral) = obligation.find_collateral_deposit_mut(&reserve_key) {
                    collateral.deposited_amount = collateral.deposited_amount
                        .checked_add(amount)
                        .ok_or(LendingError::MathOverflow)?;
                }
            }
        }
        Ok(())
    }

    fn update_single_borrow(
        &mut self,
        operation: &BatchOperation,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        if let Some(obligation) = self.obligation_cache.get_mut(&operation.obligation_key) {
            if let (Some(reserve_key), Some(amount)) = (operation.reserve_key, operation.decimal_amount) {
                if let Some(borrow) = obligation.find_liquidity_borrow_mut(&reserve_key) {
                    borrow.borrowed_amount_wads = borrow.borrowed_amount_wads.try_add(amount)?;
                }
            }
        }
        Ok(())
    }

    fn apply_compound_interest(
        &mut self,
        operation: &BatchOperation,
        rate: Decimal,
        time_delta: u64,
        accounts: &[AccountInfo],
    ) -> Result<()> {
        if let Some(obligation) = self.obligation_cache.get_mut(&operation.obligation_key) {
            if let Some(reserve_key) = operation.reserve_key {
                if let Some(borrow) = obligation.find_liquidity_borrow_mut(&reserve_key) {
                    // Apply compound interest: A = P(1 + r)^t
                    let interest_factor = Decimal::one().try_add(rate)?;
                    let compound_factor = crate::utils::math_optimized::fast_math::fast_pow(
                        interest_factor.value,
                        time_delta as u32,
                    )?;
                    
                    let new_amount = borrow.borrowed_amount_wads.try_mul(
                        Decimal::from_scaled_val(compound_factor)
                    )?;
                    
                    borrow.borrowed_amount_wads = new_amount;
                }
            }
        }
        Ok(())
    }

    fn load_obligation_from_accounts(
        &self,
        obligation_key: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<ObligationOptimized> {
        // In a real implementation, this would find and deserialize the obligation account
        // For now, return a default obligation
        ObligationOptimized::new(*obligation_key, *obligation_key)
    }

    /// Get batch processing statistics
    pub fn get_statistics(&self) -> &BatchContext {
        &self.stats
    }

    /// Clear cache to free memory
    pub fn clear_cache(&mut self) {
        self.obligation_cache.clear();
    }

    /// Get cache efficiency metrics
    pub fn cache_efficiency(&self) -> f64 {
        let total_accesses = self.stats.cache_hits + self.stats.cache_misses;
        if total_accesses == 0 {
            return 0.0;
        }
        (self.stats.cache_hits as f64) / (total_accesses as f64)
    }
}

/// Batch instruction for Anchor
#[derive(Accounts)]
pub struct BatchProcessInstruction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(mut)]
    pub market: AccountInfo<'info>,
}

/// Process batch operations instruction
pub fn process_batch_operations(
    ctx: Context<BatchProcessInstruction>,
    operations: Vec<BatchOperation>,
) -> Result<Vec<BatchOperationResult>> {
    let mut processor = BatchProcessor::new(MAX_BATCH_OPERATIONS);
    
    // Collect all accounts including remaining accounts
    let mut all_accounts = vec![
        ctx.accounts.market.clone(),
    ];
    all_accounts.extend(ctx.remaining_accounts.iter().cloned());
    
    let results = processor.process_batch_operations(&operations, &all_accounts)?;
    
    // Log performance metrics
    let stats = processor.get_statistics();
    msg!(
        "Batch processed: {} operations, {} failed, cache efficiency: {:.2}%",
        stats.operations_processed,
        stats.operations_failed,
        processor.cache_efficiency() * 100.0
    );
    
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_processor() {
        let mut processor = BatchProcessor::new(10);
        
        let operations = vec![
            BatchOperation {
                operation_type: BatchOperationType::UpdateCollateral,
                obligation_key: Pubkey::new_unique(),
                reserve_key: Some(Pubkey::new_unique()),
                amount: Some(1000),
                decimal_amount: None,
            }
        ];
        
        // Test operation grouping
        let grouped = processor.group_operations_by_type(&operations);
        assert_eq!(grouped.len(), 1);
        assert!(grouped.contains_key(&BatchOperationType::UpdateCollateral));
    }

    #[test]
    fn test_batch_context() {
        let mut context = BatchContext::new();
        
        context.record_operation(true, 1000);
        context.record_operation(false, 1200);
        
        assert_eq!(context.operations_processed, 2);
        assert_eq!(context.operations_failed, 1);
        assert_eq!(context.total_gas_used, 2200);
    }
}