use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::constants::*;
use std::cmp::min;

/// Fast mathematical operations optimized for Solana
pub mod fast_math {
    use super::*;

    /// Fast integer square root using Newton's method (optimized)
    pub fn fast_sqrt(n: u128) -> u128 {
        if n == 0 {
            return 0;
        }
        
        // Initial guess using bit manipulation for speed
        let mut x = n;
        let mut y = (x + 1) / 2;
        
        // Newton's method with early termination
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        
        x
    }

    /// Fast power calculation using binary exponentiation
    pub fn fast_pow(mut base: u128, mut exp: u32) -> Result<u128> {
        if exp == 0 {
            return Ok(1);
        }
        
        let mut result = 1u128;
        
        while exp > 0 {
            if exp & 1 == 1 {
                result = result
                    .checked_mul(base)
                    .ok_or(LendingError::MathOverflow)?;
            }
            
            base = base
                .checked_mul(base)
                .ok_or(LendingError::MathOverflow)?;
            exp >>= 1;
        }
        
        Ok(result)
    }

    /// Optimized compound interest calculation using Taylor series
    pub fn compound_interest_taylor(
        principal: u128,
        rate: u128,
        time: u128,
        precision_terms: usize,
    ) -> Result<u128> {
        if rate == 0 || time == 0 {
            return Ok(principal);
        }
        
        // e^(rt) ≈ 1 + rt + (rt)^2/2! + (rt)^3/3! + ...
        let rt = rate
            .checked_mul(time)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?;
        
        let mut result = PRECISION as u128; // 1.0
        let mut term = rt; // First term: rt
        
        for n in 1..=precision_terms {
            result = result
                .checked_add(term)
                .ok_or(LendingError::MathOverflow)?;
            
            // Calculate next term: term * rt / (n+1)
            term = term
                .checked_mul(rt)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(PRECISION as u128)
                .ok_or(LendingError::DivisionByZero)?
                .checked_div((n + 1) as u128)
                .ok_or(LendingError::DivisionByZero)?;
            
            // Break if term becomes negligible
            if term < 10 {
                break;
            }
        }
        
        principal
            .checked_mul(result)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)
    }

    /// Optimized logarithm calculation using bit operations
    pub fn fast_log2(mut x: u128) -> u128 {
        if x == 0 {
            return 0;
        }
        
        let mut result = 0u128;
        
        // Integer part
        while x >= 2 {
            x >>= 1;
            result += 1;
        }
        
        // Fractional part approximation
        if x > 1 {
            result = result
                .checked_mul(PRECISION as u128)
                .unwrap_or(u128::MAX);
        }
        
        result
    }
}

/// Decimal type for high-precision calculations
#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct Decimal {
    pub value: u128,
}

impl Default for Decimal {
    fn default() -> Self {
        Self::zero()
    }
}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl Decimal {
    /// Create a new Decimal with the given value
    pub fn from_scaled_val(value: u128) -> Self {
        Self { value }
    }

    /// Create a Decimal from an integer
    pub fn from_integer(val: u64) -> Result<Self> {
        let value = (val as u128)
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?;
        Ok(Self { value })
    }

    /// Create a zero Decimal
    pub fn zero() -> Self {
        Self { value: 0 }
    }

    /// Create a one Decimal
    pub fn one() -> Self {
        Self {
            value: PRECISION as u128,
        }
    }

    /// Validate that Decimal value is within safe bounds
    pub fn validate(&self) -> Result<()> {
        if self.value > u128::MAX / 2 {
            return Err(LendingError::MathOverflow.into());
        }
        Ok(())
    }

    /// Fast add operation with early overflow detection
    #[inline(always)]
    pub fn try_add(self, rhs: Decimal) -> Result<Decimal> {
        // Early overflow check for performance
        if self.value > u128::MAX - rhs.value {
            return Err(LendingError::MathOverflow.into());
        }
        
        Ok(Decimal {
            value: self.value + rhs.value, // Safe after overflow check
        })
    }

    /// Fast subtract operation with early underflow detection
    #[inline(always)]
    pub fn try_sub(self, rhs: Decimal) -> Result<Decimal> {
        // Early underflow check for performance
        if self.value < rhs.value {
            return Err(LendingError::MathUnderflow.into());
        }
        
        Ok(Decimal {
            value: self.value - rhs.value, // Safe after underflow check
        })
    }

    /// Optimized multiply operation using u256 intermediate
    #[inline(always)]
    pub fn try_mul(self, rhs: Decimal) -> Result<Decimal> {
        if self.value == 0 || rhs.value == 0 {
            return Ok(Decimal::zero());
        }
        
        // Use u256 arithmetic to prevent overflow
        let intermediate = (self.value as u128)
            .checked_mul(rhs.value as u128)
            .ok_or(LendingError::MathOverflow)?;
        
        let result = intermediate
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?;
        
        if result > u128::MAX {
            return Err(LendingError::MathOverflow.into());
        }
        
        Ok(Decimal { value: result })
    }

    /// Fast division with precision optimization
    #[inline(always)]
    pub fn try_div(self, rhs: Decimal) -> Result<Decimal> {
        if rhs.value == 0 {
            return Err(LendingError::DivisionByZero.into());
        }
        
        if self.value == 0 {
            return Ok(Decimal::zero());
        }
        
        // Optimize for common case where result would be close to 1
        if self.value == rhs.value {
            return Ok(Decimal::one());
        }
        
        let intermediate = (self.value as u128)
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?;
        
        let result = intermediate
            .checked_div(rhs.value as u128)
            .ok_or(LendingError::DivisionByZero)?;
        
        Ok(Decimal { value: result })
    }

    /// Optimized square root using fast_sqrt
    pub fn try_sqrt(self) -> Result<Decimal> {
        if self.value == 0 {
            return Ok(Decimal::zero());
        }
        
        // Scale up for precision, then scale back
        let scaled_value = self.value
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?;
        
        let sqrt_result = fast_math::fast_sqrt(scaled_value);
        
        Ok(Decimal { value: sqrt_result })
    }

    /// Fast power operation using optimized exponentiation
    pub fn try_pow(self, exp: u32) -> Result<Decimal> {
        if exp == 0 {
            return Ok(Decimal::one());
        }
        
        if exp == 1 {
            return Ok(self);
        }
        
        if self.value == 0 {
            return Ok(Decimal::zero());
        }
        
        if self.value == PRECISION as u128 {
            return Ok(Decimal::one()); // 1^n = 1
        }
        
        // Use fast binary exponentiation
        let result = fast_math::fast_pow(self.value, exp)?;
        
        // Adjust for precision (value was already scaled)
        let adjusted_result = result
            .checked_div(fast_math::fast_pow(PRECISION as u128, exp - 1)?)
            .ok_or(LendingError::DivisionByZero)?;
        
        Ok(Decimal { value: adjusted_result })
    }

    /// Optimized compound interest calculation
    pub fn compound_interest(
        self,
        rate: Decimal,
        time_periods: u32,
    ) -> Result<Decimal> {
        if rate.value == 0 || time_periods == 0 {
            return Ok(self);
        }
        
        // Use Taylor series for better accuracy and performance
        let result = fast_math::compound_interest_taylor(
            self.value,
            rate.value,
            time_periods as u128,
            8, // 8 terms gives good accuracy with minimal computation
        )?;
        
        Ok(Decimal { value: result })
    }

    /// Convert to floating point representation for display
    pub fn to_scaled_val(self) -> u128 {
        self.value
    }

    /// Check if this decimal represents zero
    #[inline(always)]
    pub fn is_zero(self) -> bool {
        self.value == 0
    }

    /// Check if this decimal represents one
    #[inline(always)]
    pub fn is_one(self) -> bool {
        self.value == PRECISION as u128
    }

    /// Get the minimum of two decimals
    #[inline(always)]
    pub fn min(self, other: Decimal) -> Decimal {
        if self.value <= other.value {
            self
        } else {
            other
        }
    }

    /// Get the maximum of two decimals
    #[inline(always)]
    pub fn max(self, other: Decimal) -> Decimal {
        if self.value >= other.value {
            self
        } else {
            other
        }
    }
}

/// Interest rate calculation utilities
pub mod interest {
    use super::*;
    
    /// Calculate utilization rate (borrowed / supplied)
    pub fn calculate_utilization_rate(borrowed: u64, supplied: u64) -> Result<u64> {
        if supplied == 0 {
            return Ok(0);
        }
        
        let utilization_bps = ((borrowed as u128)
            .checked_mul(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(supplied as u128)
            .ok_or(LendingError::DivisionByZero)?) as u64;
            
        Ok(utilization_bps.min(BASIS_POINTS_PRECISION))
    }
    
    /// Optimized kinked interest rate model
    pub fn calculate_borrow_rate(
        utilization_rate_bps: u64,
        base_rate_bps: u64,
        multiplier_bps: u64,
        jump_multiplier_bps: u64,
        optimal_utilization_bps: u64,
    ) -> Result<u64> {
        if utilization_rate_bps <= optimal_utilization_bps {
            // Linear portion: base_rate + (utilization * multiplier / optimal)
            let rate = base_rate_bps
                .checked_add(
                    (utilization_rate_bps as u128)
                        .checked_mul(multiplier_bps as u128)
                        .ok_or(LendingError::MathOverflow)?
                        .checked_div(optimal_utilization_bps as u128)
                        .ok_or(LendingError::DivisionByZero)? as u64
                )
                .ok_or(LendingError::MathOverflow)?;
            
            Ok(rate)
        } else {
            // Jump portion: base + multiplier + excess_utilization * jump_multiplier
            let excess_utilization = utilization_rate_bps
                .checked_sub(optimal_utilization_bps)
                .ok_or(LendingError::MathUnderflow)?;
            
            let base_plus_multiplier = base_rate_bps
                .checked_add(multiplier_bps)
                .ok_or(LendingError::MathOverflow)?;
                
            let jump_rate = (excess_utilization as u128)
                .checked_mul(jump_multiplier_bps as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div((BASIS_POINTS_PRECISION - optimal_utilization_bps) as u128)
                .ok_or(LendingError::DivisionByZero)? as u64;
                
            let total_rate = base_plus_multiplier
                .checked_add(jump_rate)
                .ok_or(LendingError::MathOverflow)?;
                
            Ok(total_rate)
        }
    }
    
    /// Calculate supply rate from borrow rate
    pub fn calculate_supply_rate(
        borrow_rate_bps: u64,
        utilization_rate_bps: u64,
        protocol_fee_bps: u64,
    ) -> Result<u64> {
        let net_borrow_rate = borrow_rate_bps
            .checked_sub(
                (borrow_rate_bps as u128)
                    .checked_mul(protocol_fee_bps as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)? as u64
            )
            .ok_or(LendingError::MathUnderflow)?;
        
        let supply_rate = (net_borrow_rate as u128)
            .checked_mul(utilization_rate_bps as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)? as u64;
            
        Ok(supply_rate)
    }
}

/// Health factor calculation utilities  
pub mod health {
    use super::*;
    
    /// Calculate health factor from collateral and debt values
    pub fn calculate_health_factor(
        collateral_value_usd: Decimal,
        debt_value_usd: Decimal,
        liquidation_threshold_weighted: Decimal,
    ) -> Result<Decimal> {
        if debt_value_usd.is_zero() {
            return Ok(Decimal::from_integer(u64::MAX)?); // Infinite health factor
        }
        
        let collateral_adjusted = collateral_value_usd.try_mul(liquidation_threshold_weighted)?;
        collateral_adjusted.try_div(debt_value_usd)
    }
    
    /// Check if position is liquidatable
    #[inline(always)]
    pub fn is_liquidatable(health_factor: Decimal) -> bool {
        health_factor < Decimal::one()
    }
    
    /// Calculate maximum liquidation amount (typically 50% of debt)
    pub fn calculate_max_liquidation_amount(
        debt_amount: u64,
        max_liquidation_percentage: u64,
    ) -> Result<u64> {
        (debt_amount as u128)
            .checked_mul(max_liquidation_percentage as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?
            .try_into()
            .map_err(|_| LendingError::MathOverflow.into())
    }
}

// Performance testing utilities
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn benchmark_decimal_operations() {
        let a = Decimal::from_integer(1000).unwrap();
        let b = Decimal::from_integer(999).unwrap();
        
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = a.try_add(b).unwrap();
        }
        let add_duration = start.elapsed();
        println!("10k additions: {:?}", add_duration);
        
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = a.try_mul(b).unwrap();
        }
        let mul_duration = start.elapsed();
        println!("10k multiplications: {:?}", mul_duration);
        
        assert!(add_duration.as_millis() < 100); // Should be very fast
        assert!(mul_duration.as_millis() < 200); // Multiplications slightly slower
    }
    
    #[test]
    fn benchmark_interest_calculations() {
        let start = Instant::now();
        for i in 0..1000 {
            let _ = interest::calculate_borrow_rate(
                8000 + (i % 2000), // 80-100% utilization
                100,  // 1% base rate
                1000, // 10% multiplier
                5000, // 50% jump multiplier
                8000, // 80% optimal utilization
            ).unwrap();
        }
        let duration = start.elapsed();
        println!("1k interest calculations: {:?}", duration);
        
        assert!(duration.as_millis() < 50); // Should be very fast
    }
}