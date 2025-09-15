use crate::constants::*;
use crate::error::LendingError;
use anchor_lang::prelude::*;
use std::cmp::min;

/// Fast mathematical operations optimized for Solana
pub mod fast_math {
    use super::*;

    /// Fast integer square root using Newton's method (optimized)
    #[inline]
    pub fn fast_sqrt(n: u128) -> Result<u128> {
        if n == 0 {
            return Ok(0);
        }

        // Initial guess using bit manipulation for speed
        let mut x = n;
        let mut y = x
            .checked_add(1)
            .ok_or(crate::error::LendingError::MathOverflow)?
            .checked_div(2)
            .ok_or(crate::error::LendingError::DivisionByZero)?;

        // Newton's method with early termination (with overflow protection)
        while y < x {
            x = y;
            y = x
                .checked_add(
                    n.checked_div(x)
                        .ok_or(crate::error::LendingError::DivisionByZero)?,
                )
                .ok_or(crate::error::LendingError::MathOverflow)?
                .checked_div(2)
                .ok_or(crate::error::LendingError::DivisionByZero)?;
        }

        Ok(x)
    }

    /// Fast power calculation using binary exponentiation
    #[inline]
    pub fn fast_pow(mut base: u128, mut exp: u32) -> Result<u128> {
        if exp == 0 {
            return Ok(1);
        }

        let mut result = 1u128;

        while exp > 0 {
            if exp & 1 == 1 {
                result = result.checked_mul(base).ok_or(LendingError::MathOverflow)?;
            }

            if exp > 1 {
                base = base.checked_mul(base).ok_or(LendingError::MathOverflow)?;
            }
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
            result = result.checked_add(term).ok_or(LendingError::MathOverflow)?;

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
    #[inline(always)]
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
    #[inline(always)]
    pub fn zero() -> Self {
        Self { value: 0 }
    }

    /// Create a one Decimal
    #[inline(always)]
    pub fn one() -> Self {
        Self {
            value: PRECISION as u128,
        }
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

    /// Optimized multiply operation
    #[inline(always)]
    pub fn try_mul(self, rhs: Decimal) -> Result<Decimal> {
        if self.value == 0 || rhs.value == 0 {
            return Ok(Decimal::zero());
        }

        if self.value == PRECISION as u128 {
            return Ok(rhs); // 1.0 * x = x
        }

        if rhs.value == PRECISION as u128 {
            return Ok(self); // x * 1.0 = x
        }

        // Use checked arithmetic for safety
        let intermediate = (self.value as u128)
            .checked_mul(rhs.value as u128)
            .ok_or(LendingError::MathOverflow)?;

        let result = intermediate
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?;

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

        // Optimize for common case where result would be 1
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
        let scaled_value = self
            .value
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?;

        let sqrt_result = fast_math::fast_sqrt(scaled_value)?;

        Ok(Decimal { value: sqrt_result })
    }

    /// Convert Decimal to u64
    pub fn try_floor_u64(self) -> Result<u64> {
        let result = self
            .value
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?;

        if result > u64::MAX as u128 {
            return Err(LendingError::MathOverflow.into());
        }

        Ok(result as u64)
    }

    /// Multiply Decimal by u64
    pub fn try_mul_u64(self, rhs: u64) -> Result<u64> {
        let result = self
            .value
            .checked_mul(rhs as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?;

        if result > u64::MAX as u128 {
            return Err(LendingError::MathOverflow.into());
        }

        Ok(result as u64)
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

    /// Convert to u128 representation
    pub fn to_scaled_val(self) -> u128 {
        self.value
    }
}

/// Interest rate calculation utilities
pub mod interest {
    use super::*;

    /// Calculate utilization rate (borrowed / supplied)
    #[inline]
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
    #[inline]
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
                        .ok_or(LendingError::DivisionByZero)? as u64,
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
    #[inline]
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
                    .ok_or(LendingError::DivisionByZero)? as u64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_operations() {
        let a = Decimal::from_integer(10).unwrap();
        let b = Decimal::from_integer(5).unwrap();

        // Test addition
        let sum = a.try_add(b).unwrap();
        assert_eq!(sum.try_floor_u64().unwrap(), 15);

        // Test subtraction
        let diff = a.try_sub(b).unwrap();
        assert_eq!(diff.try_floor_u64().unwrap(), 5);

        // Test multiplication
        let product = a.try_mul(b).unwrap();
        assert_eq!(product.try_floor_u64().unwrap(), 50);

        // Test division
        let quotient = a.try_div(b).unwrap();
        assert_eq!(quotient.try_floor_u64().unwrap(), 2);
    }

    #[test]
    fn test_interest_calculations() {
        // Test utilization rate
        let utilization = interest::calculate_utilization_rate(8000, 10000).unwrap();
        assert_eq!(utilization, 8000); // 80%

        // Test borrow rate calculation
        let borrow_rate = interest::calculate_borrow_rate(
            8000, // 80% utilization
            100,  // 1% base rate
            1000, // 10% multiplier
            5000, // 50% jump multiplier
            8000, // 80% optimal utilization
        )
        .unwrap();
        assert_eq!(borrow_rate, 1100); // 11% at optimal utilization
    }

    #[test]
    fn test_health_factor() {
        let collateral = Decimal::from_integer(1000).unwrap();
        let debt = Decimal::from_integer(500).unwrap();
        let threshold = Decimal::from_scaled_val(800 * PRECISION as u128 / 10000); // 80%

        let health = health::calculate_health_factor(collateral, debt, threshold).unwrap();
        assert!(health.try_floor_u64().unwrap() >= 1); // Should be healthy

        assert!(!health::is_liquidatable(health));
    }
}
