use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, VerificationLevel};
use crate::error::LendingError;
use crate::constants::*;
use crate::utils::math::Decimal;

/// Oracle price information
#[derive(Clone, Copy, Debug)]
pub struct OraclePrice {
    pub price: i64,
    pub confidence: u64,
    pub exponent: i32,
    pub publish_time: i64,
}

impl OraclePrice {
    /// Convert oracle price to Decimal with 18 decimal places
    pub fn to_decimal(&self) -> Result<Decimal> {
        let price_abs = (self.price.abs() as u128);
        let confidence = self.confidence as u128;
        
        // Check confidence interval - price should be within reasonable bounds
        // Only check confidence if price is non-zero
        if price_abs > 0 {
            let confidence_ratio = confidence
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(price_abs)
                .ok_or(LendingError::DivisionByZero)?;
                
            // Reject price if confidence interval is too wide (>3% for tighter control)
            if confidence_ratio > (PRECISION / 33) as u128 {
                return Err(LendingError::OracleConfidenceTooWide.into());
            }
        }

        // Normalize price to 18 decimal places
        let decimal_price = if self.exponent >= 0 {
            price_abs
                .checked_mul(10u128.pow(self.exponent as u32))
                .ok_or(LendingError::MathOverflow)?
        } else {
            let divisor = 10u128.pow((-self.exponent) as u32);
            price_abs
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(divisor)
                .ok_or(LendingError::DivisionByZero)?
        };

        // Handle negative prices (should not happen for asset prices)
        if self.price < 0 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        Ok(Decimal::from_scaled_val(decimal_price))
    }

    /// Check if the price is stale based on current slot
    pub fn is_stale(&self, current_timestamp: i64, max_staleness_seconds: u64) -> bool {
        let age = current_timestamp - self.publish_time;
        age > max_staleness_seconds as i64 || age < 0
    }

    /// Validate price quality and freshness with comprehensive checks
    pub fn validate(&self, current_timestamp: i64) -> Result<()> {
        // Check if price is positive
        if self.price <= 0 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        // Check for extreme prices that might indicate oracle issues
        let price_abs = self.price.abs() as u128;
        if price_abs > MAX_SAFE_VALUE / 1000 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        // Validate confidence interval
        let confidence_ratio = if price_abs > 0 {
            (self.confidence as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(price_abs)
                .ok_or(LendingError::DivisionByZero)?
        } else {
            return Err(LendingError::OraclePriceInvalid.into());
        };

        // Tighter confidence check - reject if >2% uncertainty
        if confidence_ratio > (PRECISION / 50) as u128 {
            return Err(LendingError::OracleConfidenceTooWide.into());
        }

        // Check staleness - convert slots to seconds properly
        // Solana has ~400ms per slot, so max staleness in seconds = slots * 0.4
        let max_staleness_seconds = (MAX_ORACLE_STALENESS_SLOTS as f64 * 0.4) as u64;
        if self.is_stale(current_timestamp, max_staleness_seconds) {
            return Err(LendingError::OraclePriceStale.into());
        }

        // Validate publish time is not in the future (with small tolerance)
        if self.publish_time > current_timestamp + 30 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        Ok(())
    }

    /// Validate with emergency mode (looser requirements during market stress)
    pub fn validate_emergency(&self, current_timestamp: i64) -> Result<()> {
        // Basic price validity
        if self.price <= 0 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        // Looser staleness check for emergency mode
        let emergency_staleness_seconds = (EMERGENCY_ORACLE_STALENESS_SLOTS as f64 * 0.4) as u64;
        if self.is_stale(current_timestamp, emergency_staleness_seconds) {
            return Err(LendingError::OraclePriceStale.into());
        }

        // Looser confidence requirement (up to 10% in emergency)
        let price_abs = self.price.abs() as u128;
        if price_abs > 0 {
            let confidence_ratio = (self.confidence as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(price_abs)
                .ok_or(LendingError::DivisionByZero)?;

            if confidence_ratio > (PRECISION / 10) as u128 {
                return Err(LendingError::OracleConfidenceTooWide.into());
            }
        }

        Ok(())
    }
}

/// Oracle manager for handling price feeds
pub struct OracleManager;

impl OracleManager {
    /// Get price from Pyth price update account
    pub fn get_pyth_price(
        price_update_account: &AccountInfo,
        feed_id: &[u8; 32],
    ) -> Result<OraclePrice> {
        use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, get_feed_id_from_hex};
        
        // Validate account ownership
        if price_update_account.owner != &pyth_solana_receiver_sdk::ID {
            return Err(LendingError::OracleAccountMismatch.into());
        }

        // Deserialize the price update account
        let mut account_data = price_update_account.data.borrow();
        let price_update = PriceUpdateV2::try_deserialize(&mut account_data.as_ref())
            .map_err(|_| LendingError::OracleAccountMismatch)?;

        // Verify the price update has been verified
        if !price_update.verification_level.is_verified() {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        // Find the price feed for the given feed ID
        let price_feed = price_update
            .price_feeds
            .iter()
            .find(|feed| &feed.id.to_bytes() == feed_id)
            .ok_or(LendingError::OracleAccountMismatch)?;

        // Extract the current price
        let price_data = &price_feed.price;
        
        // Validate price is not negative (lending protocols typically don't handle negative prices)
        if price_data.price < 0 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        Ok(OraclePrice {
            price: price_data.price,
            confidence: price_data.conf,
            exponent: price_data.exponent,
            publish_time: price_data.publish_time,
        })
    }

    /// Calculate asset value in USD using oracle price
    pub fn calculate_usd_value(
        amount: u64,
        oracle_price: &OraclePrice,
        asset_decimals: u8,
    ) -> Result<Decimal> {
        let price_decimal = oracle_price.to_decimal()?;
        let amount_decimal = Decimal::from_scaled_val(
            (amount as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(10u128.pow(asset_decimals as u32))
                .ok_or(LendingError::DivisionByZero)?,
        );

        amount_decimal.try_mul(price_decimal)
    }

    /// Calculate liquidation threshold value
    pub fn calculate_liquidation_value(
        collateral_amount: u64,
        oracle_price: &OraclePrice,
        asset_decimals: u8,
        liquidation_threshold_bps: u64,
    ) -> Result<Decimal> {
        let usd_value = Self::calculate_usd_value(collateral_amount, oracle_price, asset_decimals)?;
        let threshold_decimal = Decimal::from_scaled_val(
            (liquidation_threshold_bps as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(BASIS_POINTS_PRECISION as u128)
                .ok_or(LendingError::DivisionByZero)?,
        );

        usd_value.try_mul(threshold_decimal)
    }

    /// Check if price movement is within acceptable bounds (circuit breaker)
    pub fn validate_price_movement(
        old_price: &OraclePrice,
        new_price: &OraclePrice,
        max_change_bps: u64,
    ) -> Result<()> {
        if old_price.price <= 0 || new_price.price <= 0 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        let old_price_abs = old_price.price.abs() as u128;
        let new_price_abs = new_price.price.abs() as u128;

        // Calculate percentage change
        let price_diff = if new_price_abs > old_price_abs {
            new_price_abs - old_price_abs
        } else {
            old_price_abs - new_price_abs
        };

        let change_bps = price_diff
            .checked_mul(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(old_price_abs)
            .ok_or(LendingError::DivisionByZero)?;

        if change_bps > max_change_bps as u128 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        Ok(())
    }

    /// Get TWAP (Time-Weighted Average Price) over multiple price updates
    pub fn calculate_twap(
        prices: &[OraclePrice],
        time_window_seconds: u64,
        current_timestamp: i64,
    ) -> Result<OraclePrice> {
        if prices.is_empty() {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        // Filter prices within the time window
        let window_start = current_timestamp - time_window_seconds as i64;
        let valid_prices: Vec<&OraclePrice> = prices
            .iter()
            .filter(|p| p.publish_time >= window_start && p.publish_time <= current_timestamp)
            .collect();

        if valid_prices.is_empty() {
            return Err(LendingError::OraclePriceStale.into());
        }

        // Calculate time-weighted average
        let mut total_weighted_price = 0u128;
        let mut total_weight = 0u64;

        for (i, price) in valid_prices.iter().enumerate() {
            let weight = if i == valid_prices.len() - 1 {
                // Last price gets weight until current time
                (current_timestamp - price.publish_time) as u64
            } else {
                // Weight is duration until next price update
                (valid_prices[i + 1].publish_time - price.publish_time) as u64
            };

            if weight > 0 {
                total_weighted_price = total_weighted_price
                    .checked_add((price.price.abs() as u128).checked_mul(weight as u128).ok_or(LendingError::MathOverflow)?)
                    .ok_or(LendingError::MathOverflow)?;
                total_weight = total_weight.checked_add(weight).ok_or(LendingError::MathOverflow)?;
            }
        }

        if total_weight == 0 {
            return Err(LendingError::OraclePriceInvalid.into());
        }

        let twap_price = total_weighted_price
            .checked_div(total_weight as u128)
            .ok_or(LendingError::DivisionByZero)? as i64;

        // Use the most recent price's metadata with TWAP price
        let latest_price = valid_prices.last()
            .ok_or(LendingError::OraclePriceInvalid)?;
        Ok(OraclePrice {
            price: twap_price,
            confidence: latest_price.confidence,
            exponent: latest_price.exponent,
            publish_time: latest_price.publish_time,
        })
    }
}