use anchor_lang::prelude::*;
use crate::error::LendingError;
use crate::constants::*;
use std::cmp::min;

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

    /// Add two Decimals
    pub fn try_add(self, rhs: Decimal) -> Result<Decimal> {
        Ok(Decimal {
            value: self
                .value
                .checked_add(rhs.value)
                .ok_or(LendingError::MathOverflow)?,
        })
    }

    /// Subtract two Decimals
    pub fn try_sub(self, rhs: Decimal) -> Result<Decimal> {
        Ok(Decimal {
            value: self
                .value
                .checked_sub(rhs.value)
                .ok_or(LendingError::MathUnderflow)?,
        })
    }

    /// Multiply two Decimals
    pub fn try_mul(self, rhs: Decimal) -> Result<Decimal> {
        let result = self
            .value
            .checked_mul(rhs.value)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?;

        Ok(Decimal { value: result })
    }

    /// Divide two Decimals
    pub fn try_div(self, rhs: Decimal) -> Result<Decimal> {
        if rhs.value == 0 {
            return Err(LendingError::DivisionByZero.into());
        }

        let result = self
            .value
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(rhs.value)
            .ok_or(LendingError::DivisionByZero)?;

        Ok(Decimal { value: result })
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

    /// Convert Decimal to u128
    pub fn to_scaled_val(self) -> u128 {
        self.value
    }

    /// Check if Decimal is zero
    pub fn is_zero(self) -> bool {
        self.value == 0
    }

    /// Get the minimum of two Decimals
    pub fn min(self, other: Decimal) -> Decimal {
        if self.value <= other.value {
            self
        } else {
            other
        }
    }

    /// Get the maximum of two Decimals
    pub fn max(self, other: Decimal) -> Decimal {
        if self.value >= other.value {
            self
        } else {
            other
        }
    }
}

/// Rate calculations for lending protocol
pub struct Rate;

impl Rate {
    /// Calculate compound interest rate
    /// Formula: A = P * (1 + r/n)^(n*t)
    /// Where: P = principal, r = annual rate, n = compounds per year, t = time in years
    pub fn compound_interest(
        principal: Decimal,
        rate: Decimal,
        compounds_per_year: u64,
        time_fraction: Decimal,
    ) -> Result<Decimal> {
        if rate.is_zero() {
            return Ok(principal);
        }

        // Validate inputs to prevent overflow
        principal.validate()?;
        rate.validate()?;
        time_fraction.validate()?;
        
        if compounds_per_year == 0 {
            return Err(LendingError::DivisionByZero.into());
        }
        
        // High-precision compound interest calculation with overflow protection
        let rate_per_compound = rate.try_div(Decimal::from_integer(compounds_per_year)?)?;
        let compound_factor = Decimal::one().try_add(rate_per_compound)?;
        let total_compounds = Decimal::from_integer(compounds_per_year)?.try_mul(time_fraction)?;
        
        // Validate compound factor is reasonable
        if compound_factor.value > MAX_SAFE_VALUE / 1000 {
            return Err(LendingError::MathOverflow.into());
        }
        
        // Use precise calculation based on compounding frequency
        let result = if total_compounds.value <= PRECISION as u128 {
            // For small compound periods, use high-precision linear approximation
            let precise_interest = principal
                .try_mul(rate)?
                .try_mul(time_fraction)?;
            
            // Add compound adjustment for better accuracy
            let compound_adjustment = precise_interest
                .try_mul(rate_per_compound)?
                .try_div(Decimal::from_integer(2)?)?; // Second order correction
            
            principal.try_add(precise_interest)?.try_add(compound_adjustment)?
        } else {
            // For larger periods, use enhanced power approximation with precision preservation
            Self::enhanced_power_approximation(compound_factor, total_compounds)?.try_mul(principal)?
        };
        
        // Validate result is reasonable
        if result.value > principal.value.saturating_mul(1000) {
            return Err(LendingError::MathOverflow.into());
        }
        
        Ok(result)
    }

    /// Calculate utilization rate: borrowed_amount / (borrowed_amount + available_amount)
    pub fn utilization_rate(borrowed_amount: u64, available_amount: u64) -> Result<Decimal> {
        // Validate input bounds
        if borrowed_amount > u64::MAX / 2 || available_amount > u64::MAX / 2 {
            return Err(LendingError::MathOverflow.into());
        }

        let total_amount = borrowed_amount
            .checked_add(available_amount)
            .ok_or(LendingError::MathOverflow)?;

        if total_amount == 0 {
            return Ok(Decimal::zero());
        }

        let borrowed_decimal = Decimal::from_integer(borrowed_amount)?;
        let total_decimal = Decimal::from_integer(total_amount)?;

        let result = borrowed_decimal.try_div(total_decimal)?;
        result.validate()?;
        Ok(result)
    }

    /// Calculate interest rate based on utilization
    /// Uses a kinked rate model: low rate up to optimal utilization, then high rate
    pub fn calculate_interest_rate(
        base_rate_bps: u64,
        multiplier_bps: u64,
        jump_multiplier_bps: u64,
        optimal_utilization_bps: u64,
        current_utilization: Decimal,
    ) -> Result<Decimal> {
        let base_rate = Decimal::from_scaled_val(
            (base_rate_bps as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(BASIS_POINTS_PRECISION as u128)
                .ok_or(LendingError::DivisionByZero)?,
        );

        let optimal_utilization = Decimal::from_scaled_val(
            (optimal_utilization_bps as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(BASIS_POINTS_PRECISION as u128)
                .ok_or(LendingError::DivisionByZero)?,
        );

        if current_utilization.value <= optimal_utilization.value {
            // Below optimal utilization: base_rate + (utilization * multiplier)
            let rate_slope = Decimal::from_scaled_val(
                (multiplier_bps as u128)
                    .checked_mul(PRECISION as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)?,
            );
            let variable_rate = current_utilization.try_mul(rate_slope)?;
            base_rate.try_add(variable_rate)
        } else {
            // Above optimal utilization: base_rate + (optimal * multiplier) + (excess * jump_multiplier)
            let normal_rate = optimal_utilization.try_mul(Decimal::from_scaled_val(
                (multiplier_bps as u128)
                    .checked_mul(PRECISION as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)?,
            ))?;

            let excess_utilization = current_utilization.try_sub(optimal_utilization)?;
            let jump_rate = excess_utilization.try_mul(Decimal::from_scaled_val(
                (jump_multiplier_bps as u128)
                    .checked_mul(PRECISION as u128)
                    .ok_or(LendingError::MathOverflow)?
                    .checked_div(BASIS_POINTS_PRECISION as u128)
                    .ok_or(LendingError::DivisionByZero)?,
            ))?;

            base_rate.try_add(normal_rate)?.try_add(jump_rate)
        }
    }

    /// Power approximation for compound interest calculations with overflow protection
    /// Uses Taylor series expansion: (1+x)^n ≈ 1 + nx + n(n-1)x²/2! + ...
    fn power_approximation(base: Decimal, exponent: Decimal) -> Result<Decimal> {
        // Validate inputs to prevent overflow
        if base.value > MAX_SAFE_VALUE || exponent.value > MAX_SAFE_VALUE {
            return Err(LendingError::MathOverflow.into());
        }

        if exponent.is_zero() {
            return Ok(Decimal::one());
        }

        if base == Decimal::one() {
            return Ok(Decimal::one());
        }

        // Check for extreme values that could cause overflow
        if base.value == 0 {
            return Ok(Decimal::zero());
        }

        // For small exponents, use linear approximation with bounds checking
        let x = base.try_sub(Decimal::one())?;
        
        // Validate x is within safe bounds
        if x.value > MAX_SAFE_VALUE / 2 {
            return Err(LendingError::MathOverflow.into());
        }

        if exponent.value < PRECISION as u128 / 10 {
            // Linear: 1 + n*x with overflow check
            let linear_term = Self::checked_mul_with_bounds(exponent, x)?;
            return Decimal::one().try_add(linear_term);
        }

        // For larger exponents, add quadratic term with rigorous bounds checking
        // 1 + n*x + n*(n-1)*x²/2
        let linear_term = Self::checked_mul_with_bounds(exponent, x)?;
        
        // Check bounds before computing quadratic coefficient
        let exponent_minus_one = exponent.try_sub(Decimal::one())?;
        let quadratic_coefficient = Self::checked_mul_with_bounds(exponent, exponent_minus_one)?;
        
        // Compute x² with overflow protection
        let x_squared = Self::checked_mul_with_bounds(x, x)?;
        
        // Compute quadratic term: coefficient * x² / 2
        let quadratic_product = Self::checked_mul_with_bounds(quadratic_coefficient, x_squared)?;
        let quadratic_term = quadratic_product.try_div(Decimal::from_integer(2)?)?;

        // Final assembly with overflow checks
        let intermediate = Decimal::one().try_add(linear_term)?;
        intermediate.try_add(quadratic_term)
    }

    /// Checked multiplication with bounds validation
    fn checked_mul_with_bounds(a: Decimal, b: Decimal) -> Result<Decimal> {
        // Pre-check for potential overflow
        if a.value > 0 && b.value > 0 {
            let max_allowed = MAX_SAFE_VALUE / std::cmp::max(a.value, 1);
            if b.value > max_allowed {
                return Err(LendingError::MathOverflow.into());
            }
        }
        a.try_mul(b)
    }

    /// Enhanced power approximation with higher precision for compound interest
    /// Uses Taylor series with more terms for better accuracy: (1+x)^n ≈ 1 + nx + n(n-1)x²/2! + n(n-1)(n-2)x³/3! + ...
    fn enhanced_power_approximation(base: Decimal, exponent: Decimal) -> Result<Decimal> {
        // Validate inputs to prevent overflow
        if base.value > MAX_SAFE_VALUE || exponent.value > MAX_SAFE_VALUE {
            return Err(LendingError::MathOverflow.into());
        }

        if exponent.is_zero() {
            return Ok(Decimal::one());
        }

        if base == Decimal::one() {
            return Ok(Decimal::one());
        }

        if base.value == 0 {
            return Ok(Decimal::zero());
        }

        let x = base.try_sub(Decimal::one())?;
        
        // Validate x is within safe bounds
        if x.value > MAX_SAFE_VALUE / 10 {
            return Err(LendingError::MathOverflow.into());
        }

        // Calculate terms of Taylor series with precision preservation
        // Term 0: 1
        let mut result = Decimal::one();
        
        // Term 1: nx
        let term1 = Self::checked_mul_with_bounds(exponent, x)?;
        result = result.try_add(term1)?;
        
        // Term 2: n(n-1)x²/2
        if exponent.value > Decimal::one().value {
            let n_minus_1 = exponent.try_sub(Decimal::one())?;
            let coefficient2 = Self::checked_mul_with_bounds(exponent, n_minus_1)?;
            let x_squared = Self::checked_mul_with_bounds(x, x)?;
            let term2_numerator = Self::checked_mul_with_bounds(coefficient2, x_squared)?;
            let term2 = term2_numerator.try_div(Decimal::from_integer(2)?)?;
            result = result.try_add(term2)?;
            
            // Term 3: n(n-1)(n-2)x³/6 for even higher precision
            if exponent.value > Decimal::from_integer(2)?.value {
                let n_minus_2 = exponent.try_sub(Decimal::from_integer(2)?)?;
                let coefficient3 = Self::checked_mul_with_bounds(coefficient2, n_minus_2)?;
                let x_cubed = Self::checked_mul_with_bounds(x_squared, x)?;
                let term3_numerator = Self::checked_mul_with_bounds(coefficient3, x_cubed)?;
                let term3 = term3_numerator.try_div(Decimal::from_integer(6)?)?;
                result = result.try_add(term3)?;
            }
        }

        // Validate result is within reasonable bounds
        if result.value > MAX_SAFE_VALUE / 10 {
            return Err(LendingError::MathOverflow.into());
        }

        Ok(result)
    }
}