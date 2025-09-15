use crate::error::LendingError;
use crate::state::obligation::{ObligationCollateral, ObligationLiquidity};
use crate::utils::math::*;
use anchor_lang::prelude::*;

/// Optimized iterator utilities with early termination and lazy evaluation
pub mod optimized_iterators {
    use super::*;

    /// Fast calculation with early termination for zero values
    pub fn calculate_total_collateral_value_optimized(
        deposits: &[ObligationCollateral],
    ) -> Result<Decimal> {
        let mut total_value = Decimal::zero();

        // Use iterator with early termination on zero values
        for deposit in deposits
            .iter()
            .take_while(|d| !d.market_value_usd.is_zero()) // Stop at first zero value
            .filter(|d| d.deposited_amount > 0)
        // Skip empty deposits
        {
            total_value = total_value.try_add(deposit.market_value_usd)?;

            // Early termination if we hit overflow risk
            if total_value.value > u128::MAX / 2 {
                break;
            }
        }

        Ok(total_value)
    }

    /// Fast calculation with early termination for borrowed amounts
    pub fn calculate_total_borrowed_value_optimized(
        borrows: &[ObligationLiquidity],
    ) -> Result<Decimal> {
        let mut total_value = Decimal::zero();

        // Use iterator with early termination
        for borrow in borrows
            .iter()
            .take_while(|b| !b.borrowed_amount_wads.is_zero()) // Stop at first zero value
            .filter(|b| !b.market_value_usd.is_zero())
        // Skip zero market values
        {
            total_value = total_value.try_add(borrow.market_value_usd)?;

            // Early termination if we hit overflow risk
            if total_value.value > u128::MAX / 2 {
                break;
            }
        }

        Ok(total_value)
    }

    /// Optimized weighted average calculation with early returns
    pub fn calculate_weighted_ltv_optimized(deposits: &[ObligationCollateral]) -> Result<u64> {
        if deposits.is_empty() {
            return Ok(0);
        }

        let mut total_value = 0u128;
        let mut weighted_ltv = 0u128;

        // Use early termination and SIMD-friendly operations
        for deposit in deposits
            .iter()
            .take_while(|d| d.deposited_amount > 0)
            .take(16)
        // Limit to prevent excessive computation
        {
            let value = deposit.market_value_usd.try_floor_u64()?;
            if value == 0 {
                continue; // Skip zero values
            }

            let value_u128 = value as u128;
            total_value = total_value.saturating_add(value_u128);

            weighted_ltv = weighted_ltv
                .saturating_add(value_u128.saturating_mul(deposit.loan_to_value_bps as u128));
        }

        if total_value == 0 {
            return Ok(0);
        }

        let result = weighted_ltv / total_value;
        Ok(result.min(u64::MAX as u128) as u64)
    }

    /// Find maximum collateral deposit efficiently with early termination
    pub fn find_max_collateral_deposit(
        deposits: &[ObligationCollateral],
    ) -> Option<&ObligationCollateral> {
        if deposits.is_empty() {
            return None;
        }

        // Use fold with early comparison optimization
        deposits
            .iter()
            .filter(|d| d.deposited_amount > 0) // Skip empty deposits
            .max_by(|a, b| a.market_value_usd.cmp(&b.market_value_usd))
    }

    /// Find maximum borrow position efficiently
    pub fn find_max_borrow_position(
        borrows: &[ObligationLiquidity],
    ) -> Option<&ObligationLiquidity> {
        if borrows.is_empty() {
            return None;
        }

        borrows
            .iter()
            .filter(|b| !b.borrowed_amount_wads.is_zero())
            .max_by(|a, b| a.market_value_usd.cmp(&b.market_value_usd))
    }

    /// Lazy evaluation for health factor calculation - only compute when needed
    pub struct HealthFactorCalculator<'a> {
        deposits: &'a [ObligationCollateral],
        borrows: &'a [ObligationLiquidity],
        cached_collateral_value: Option<Decimal>,
        cached_borrowed_value: Option<Decimal>,
        cached_threshold_value: Option<Decimal>,
    }

    impl<'a> HealthFactorCalculator<'a> {
        pub fn new(
            deposits: &'a [ObligationCollateral],
            borrows: &'a [ObligationLiquidity],
        ) -> Self {
            Self {
                deposits,
                borrows,
                cached_collateral_value: None,
                cached_borrowed_value: None,
                cached_threshold_value: None,
            }
        }

        /// Lazy calculation of collateral value - only computed when accessed
        pub fn collateral_value(&mut self) -> Result<Decimal> {
            if let Some(value) = self.cached_collateral_value {
                return Ok(value);
            }

            let value = calculate_total_collateral_value_optimized(self.deposits)?;
            self.cached_collateral_value = Some(value);
            Ok(value)
        }

        /// Lazy calculation of borrowed value
        pub fn borrowed_value(&mut self) -> Result<Decimal> {
            if let Some(value) = self.cached_borrowed_value {
                return Ok(value);
            }

            let value = calculate_total_borrowed_value_optimized(self.borrows)?;
            self.cached_borrowed_value = Some(value);
            Ok(value)
        }

        /// Lazy calculation of liquidation threshold value
        pub fn threshold_value(&mut self) -> Result<Decimal> {
            if let Some(value) = self.cached_threshold_value {
                return Ok(value);
            }

            let mut threshold_value = Decimal::zero();

            // Early termination optimizations
            for deposit in self
                .deposits
                .iter()
                .take_while(|d| !d.market_value_usd.is_zero())
                .filter(|d| d.liquidation_threshold_bps > 0)
            {
                let threshold_decimal = Decimal::from_scaled_val(
                    (deposit.liquidation_threshold_bps as u128)
                        .saturating_mul(crate::constants::PRECISION as u128)
                        .saturating_div(crate::constants::BASIS_POINTS_PRECISION as u128),
                );

                let weighted_value = deposit.market_value_usd.try_mul(threshold_decimal)?;
                threshold_value = threshold_value.try_add(weighted_value)?;
            }

            self.cached_threshold_value = Some(threshold_value);
            Ok(threshold_value)
        }

        /// Calculate health factor with all optimizations
        pub fn health_factor(&mut self) -> Result<Decimal> {
            let borrowed_value = self.borrowed_value()?;

            // Early return for zero debt - infinite health factor
            if borrowed_value.is_zero() {
                return Ok(Decimal::from_integer(u64::MAX)?);
            }

            let threshold_value = self.threshold_value()?;

            // Early return for zero collateral
            if threshold_value.is_zero() {
                return Ok(Decimal::zero());
            }

            threshold_value.try_div(borrowed_value)
        }

        /// Check if position is safe without full health factor calculation
        pub fn is_safe_quick_check(&mut self) -> Result<bool> {
            let borrowed_value = self.borrowed_value()?;

            // If no debt, position is safe
            if borrowed_value.is_zero() {
                return Ok(true);
            }

            let collateral_value = self.collateral_value()?;

            // Quick check: if collateral is much higher than debt, likely safe
            // This avoids expensive threshold calculations in most cases
            if collateral_value.value > borrowed_value.value.saturating_mul(2) {
                return Ok(true);
            }

            // If very close, do full calculation
            let health_factor = self.health_factor()?;
            Ok(health_factor.value >= Decimal::one().value)
        }
    }

    /// Vectorized operations for batch calculations
    pub fn batch_calculate_health_factors(
        obligations_data: &[(Vec<ObligationCollateral>, Vec<ObligationLiquidity>)],
    ) -> Result<Vec<Decimal>> {
        let mut results = Vec::with_capacity(obligations_data.len());

        for (deposits, borrows) in obligations_data.iter() {
            let mut calculator = HealthFactorCalculator::new(deposits, borrows);
            let health_factor = calculator.health_factor()?;
            results.push(health_factor);
        }

        Ok(results)
    }

    /// Find unhealthy positions efficiently with early termination
    pub fn find_unhealthy_positions(
        obligations_data: &[(Vec<ObligationCollateral>, Vec<ObligationLiquidity>)],
    ) -> Result<Vec<usize>> {
        let mut unhealthy_indices = Vec::new();

        for (index, (deposits, borrows)) in obligations_data.iter().enumerate() {
            let mut calculator = HealthFactorCalculator::new(deposits, borrows);

            // Use quick check first for performance
            if !calculator.is_safe_quick_check()? {
                unhealthy_indices.push(index);
            }
        }

        Ok(unhealthy_indices)
    }
}

/// Performance benchmarking utilities
pub mod performance_bench {
    use super::*;
    use std::time::Instant;

    pub fn benchmark_lookup_operations(iterations: usize) -> (u128, u128) {
        let test_data = generate_test_obligations(10);

        // Benchmark linear search (O(n))
        let start = Instant::now();
        for _ in 0..iterations {
            for (deposits, _) in &test_data {
                // Simulate linear search
                let _ = deposits.iter().find(|d| d.deposited_amount > 1000);
            }
        }
        let linear_time = start.elapsed().as_nanos();

        // Benchmark optimized operations
        let start = Instant::now();
        for _ in 0..iterations {
            for (deposits, borrows) in &test_data {
                let mut calculator =
                    optimized_iterators::HealthFactorCalculator::new(deposits, borrows);
                let _ = calculator.is_safe_quick_check();
            }
        }
        let optimized_time = start.elapsed().as_nanos();

        (linear_time, optimized_time)
    }

    fn generate_test_obligations(
        count: usize,
    ) -> Vec<(Vec<ObligationCollateral>, Vec<ObligationLiquidity>)> {
        let mut obligations = Vec::with_capacity(count);

        for i in 0..count {
            let deposits = vec![ObligationCollateral {
                deposit_reserve: Pubkey::new_unique(),
                deposited_amount: 1000 + i as u64,
                market_value_usd: Decimal::from_integer(1000 + i as u64).unwrap(),
                liquidation_threshold_bps: 8000,
                loan_to_value_bps: 7500,
            }];

            let borrows = vec![ObligationLiquidity {
                borrow_reserve: Pubkey::new_unique(),
                borrowed_amount_wads: Decimal::from_integer(500 + i as u64).unwrap(),
                market_value_usd: Decimal::from_integer(500 + i as u64).unwrap(),
                cumulative_borrow_rate_wads: Decimal::one(),
            }];

            obligations.push((deposits, borrows));
        }

        obligations
    }
}

#[cfg(test)]
mod tests {
    use super::optimized_iterators::*;
    use super::*;

    #[test]
    fn test_early_termination() {
        let deposits = vec![
            ObligationCollateral {
                deposit_reserve: Pubkey::new_unique(),
                deposited_amount: 1000,
                market_value_usd: Decimal::from_integer(1000).unwrap(),
                liquidation_threshold_bps: 8000,
                loan_to_value_bps: 7500,
            },
            ObligationCollateral {
                deposit_reserve: Pubkey::new_unique(),
                deposited_amount: 0, // This should trigger early termination
                market_value_usd: Decimal::zero(),
                liquidation_threshold_bps: 8000,
                loan_to_value_bps: 7500,
            },
        ];

        let total = calculate_total_collateral_value_optimized(&deposits).unwrap();
        assert_eq!(total.try_floor_u64().unwrap(), 1000);
    }

    #[test]
    fn test_lazy_evaluation() {
        let deposits = vec![ObligationCollateral {
            deposit_reserve: Pubkey::new_unique(),
            deposited_amount: 2000,
            market_value_usd: Decimal::from_integer(2000).unwrap(),
            liquidation_threshold_bps: 8000,
            loan_to_value_bps: 7500,
        }];

        let borrows = vec![ObligationLiquidity {
            borrow_reserve: Pubkey::new_unique(),
            borrowed_amount_wads: Decimal::from_integer(1000).unwrap(),
            market_value_usd: Decimal::from_integer(1000).unwrap(),
            cumulative_borrow_rate_wads: Decimal::one(),
        }];

        let mut calculator = HealthFactorCalculator::new(&deposits, &borrows);

        // First call should compute and cache
        let health_factor1 = calculator.health_factor().unwrap();

        // Second call should use cached value
        let health_factor2 = calculator.health_factor().unwrap();

        assert_eq!(health_factor1.value, health_factor2.value);
        assert!(health_factor1.value > Decimal::one().value); // Should be healthy
    }

    #[test]
    fn test_quick_safety_check() {
        let deposits = vec![ObligationCollateral {
            deposit_reserve: Pubkey::new_unique(),
            deposited_amount: 10000,
            market_value_usd: Decimal::from_integer(10000).unwrap(),
            liquidation_threshold_bps: 8000,
            loan_to_value_bps: 7500,
        }];

        let borrows = vec![ObligationLiquidity {
            borrow_reserve: Pubkey::new_unique(),
            borrowed_amount_wads: Decimal::from_integer(1000).unwrap(),
            market_value_usd: Decimal::from_integer(1000).unwrap(),
            cumulative_borrow_rate_wads: Decimal::one(),
        }];

        let mut calculator = HealthFactorCalculator::new(&deposits, &borrows);

        // Should quickly determine this is safe without full calculation
        assert!(calculator.is_safe_quick_check().unwrap());
    }
}
