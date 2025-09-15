use crate::error::LendingError;
use crate::utils::math::Decimal;
use anchor_lang::prelude::*;

/// Protocol metrics for monitoring and analytics
#[account]
pub struct ProtocolMetrics {
    /// Version for upgradability
    pub version: u8,

    /// Market this metrics belongs to
    pub market: Pubkey,

    /// Total Value Locked (TVL) across all reserves
    pub total_value_locked_usd: u64,

    /// Total borrowed amount across all reserves
    pub total_borrowed_usd: u64,

    /// Total fees collected by the protocol
    pub total_fees_collected_usd: u64,

    /// Number of active users
    pub active_users: u32,

    /// Number of active reserves
    pub active_reserves: u32,

    /// Number of liquidations in the last 24h
    pub liquidations_24h: u32,

    /// Average health factor of all obligations
    pub average_health_factor: u64, // In basis points

    /// Protocol utilization rate (borrowed / supplied)
    pub protocol_utilization_rate: u64, // In basis points

    /// Last update timestamp
    pub last_update_timestamp: u64,

    /// Last update slot
    pub last_update_slot: u64,

    /// Reserved space for future metrics
    pub reserved: [u8; 128],
}

impl ProtocolMetrics {
    /// Size of the ProtocolMetrics account
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // market
        8 + // total_value_locked_usd
        8 + // total_borrowed_usd
        8 + // total_fees_collected_usd
        4 + // active_users
        4 + // active_reserves
        4 + // liquidations_24h
        8 + // average_health_factor
        8 + // protocol_utilization_rate
        8 + // last_update_timestamp
        8 + // last_update_slot
        128; // reserved

    /// Create new protocol metrics
    pub fn new(market: Pubkey) -> Result<Self> {
        let clock = Clock::get()?;

        Ok(Self {
            version: 1,
            market,
            total_value_locked_usd: 0,
            total_borrowed_usd: 0,
            total_fees_collected_usd: 0,
            active_users: 0,
            active_reserves: 0,
            liquidations_24h: 0,
            average_health_factor: 10000, // 100% healthy
            protocol_utilization_rate: 0,
            last_update_timestamp: clock.unix_timestamp as u64,
            last_update_slot: clock.slot,
            reserved: [0; 128],
        })
    }

    /// Update metrics with new data
    pub fn update(
        &mut self,
        tvl_usd: u64,
        borrowed_usd: u64,
        fees_collected_usd: u64,
        active_users: u32,
        active_reserves: u32,
        avg_health_factor: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;

        self.total_value_locked_usd = tvl_usd;
        self.total_borrowed_usd = borrowed_usd;
        self.total_fees_collected_usd = fees_collected_usd;
        self.active_users = active_users;
        self.active_reserves = active_reserves;
        self.average_health_factor = avg_health_factor;

        // Calculate utilization rate
        if tvl_usd > 0 {
            self.protocol_utilization_rate = ((borrowed_usd as u128)
                .checked_mul(10000)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(tvl_usd as u128)
                .ok_or(LendingError::DivisionByZero)?)
                as u64;
        } else {
            self.protocol_utilization_rate = 0;
        }

        self.last_update_timestamp = clock.unix_timestamp as u64;
        self.last_update_slot = clock.slot;

        Ok(())
    }

    /// Increment liquidation counter
    pub fn record_liquidation(&mut self) -> Result<()> {
        self.liquidations_24h = self.liquidations_24h.saturating_add(1);
        Ok(())
    }

    /// Reset 24h counters (should be called daily)
    pub fn reset_daily_counters(&mut self) -> Result<()> {
        self.liquidations_24h = 0;
        Ok(())
    }
}

/// Reserve-specific metrics
#[account]
pub struct ReserveMetrics {
    /// Version for upgradability
    pub version: u8,

    /// Reserve this metrics belongs to
    pub reserve: Pubkey,

    /// Total supplied amount in native units
    pub total_supplied: u64,

    /// Total borrowed amount in native units
    pub total_borrowed: u64,

    /// Current utilization rate in basis points
    pub utilization_rate: u64,

    /// Current supply APY in basis points
    pub supply_apy: u64,

    /// Current borrow APY in basis points
    pub borrow_apy: u64,

    /// Volume traded in the last 24h
    pub volume_24h: u64,

    /// Number of suppliers
    pub supplier_count: u32,

    /// Number of borrowers
    pub borrower_count: u32,

    /// Largest single deposit
    pub largest_deposit: u64,

    /// Largest single borrow
    pub largest_borrow: u64,

    /// Last update timestamp
    pub last_update_timestamp: u64,

    /// Last update slot
    pub last_update_slot: u64,

    /// Reserved space
    pub reserved: [u8; 64],
}

impl ReserveMetrics {
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // reserve
        8 + // total_supplied
        8 + // total_borrowed
        8 + // utilization_rate
        8 + // supply_apy
        8 + // borrow_apy
        8 + // volume_24h
        4 + // supplier_count
        4 + // borrower_count
        8 + // largest_deposit
        8 + // largest_borrow
        8 + // last_update_timestamp
        8 + // last_update_slot
        64; // reserved

    pub fn new(reserve: Pubkey) -> Result<Self> {
        let clock = Clock::get()?;

        Ok(Self {
            version: 1,
            reserve,
            total_supplied: 0,
            total_borrowed: 0,
            utilization_rate: 0,
            supply_apy: 0,
            borrow_apy: 0,
            volume_24h: 0,
            supplier_count: 0,
            borrower_count: 0,
            largest_deposit: 0,
            largest_borrow: 0,
            last_update_timestamp: clock.unix_timestamp as u64,
            last_update_slot: clock.slot,
            reserved: [0; 64],
        })
    }

    /// Update reserve metrics
    pub fn update(
        &mut self,
        supplied: u64,
        borrowed: u64,
        supply_apy: u64,
        borrow_apy: u64,
        supplier_count: u32,
        borrower_count: u32,
    ) -> Result<()> {
        let clock = Clock::get()?;

        self.total_supplied = supplied;
        self.total_borrowed = borrowed;
        self.supply_apy = supply_apy;
        self.borrow_apy = borrow_apy;
        self.supplier_count = supplier_count;
        self.borrower_count = borrower_count;

        // Calculate utilization rate
        if supplied > 0 {
            self.utilization_rate = ((borrowed as u128)
                .checked_mul(10000)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(supplied as u128)
                .ok_or(LendingError::DivisionByZero)?) as u64;
        } else {
            self.utilization_rate = 0;
        }

        self.last_update_timestamp = clock.unix_timestamp as u64;
        self.last_update_slot = clock.slot;

        Ok(())
    }

    /// Record transaction volume
    pub fn record_volume(&mut self, amount: u64) -> Result<()> {
        self.volume_24h = self.volume_24h.saturating_add(amount);
        Ok(())
    }

    /// Update largest deposit if new amount is larger
    pub fn update_largest_deposit(&mut self, amount: u64) {
        if amount > self.largest_deposit {
            self.largest_deposit = amount;
        }
    }

    /// Update largest borrow if new amount is larger
    pub fn update_largest_borrow(&mut self, amount: u64) {
        if amount > self.largest_borrow {
            self.largest_borrow = amount;
        }
    }
}

/// Metrics aggregator for calculating protocol-wide statistics
pub struct MetricsAggregator;

impl MetricsAggregator {
    /// Calculate average health factor from a list of obligations
    pub fn calculate_average_health_factor(health_factors: &[u64]) -> u64 {
        if health_factors.is_empty() {
            return 10000; // 100% if no obligations
        }

        let sum: u128 = health_factors.iter().map(|&hf| hf as u128).sum();
        (sum / health_factors.len() as u128) as u64
    }

    /// Calculate protocol utilization rate
    pub fn calculate_protocol_utilization(total_supplied: u64, total_borrowed: u64) -> u64 {
        if total_supplied == 0 {
            return 0;
        }

        ((total_borrowed as u128)
            .saturating_mul(10000)
            .saturating_div(total_supplied as u128)) as u64
    }

    /// Detect anomalies in metrics
    pub fn detect_anomalies(
        current_metrics: &ProtocolMetrics,
        previous_metrics: &ProtocolMetrics,
    ) -> Vec<String> {
        let mut anomalies = Vec::new();

        // Check for sudden TVL drop (>20%)
        if current_metrics.total_value_locked_usd < previous_metrics.total_value_locked_usd {
            let drop_percentage = ((previous_metrics.total_value_locked_usd
                - current_metrics.total_value_locked_usd)
                as u128)
                .saturating_mul(100)
                .saturating_div(previous_metrics.total_value_locked_usd as u128);

            if drop_percentage > 20 {
                anomalies.push(format!("TVL dropped by {}%", drop_percentage));
            }
        }

        // Check for high liquidation activity
        if current_metrics.liquidations_24h > 100 {
            anomalies.push(format!(
                "High liquidation activity: {} liquidations",
                current_metrics.liquidations_24h
            ));
        }

        // Check for low average health factor
        if current_metrics.average_health_factor < 11000 {
            // Below 110%
            anomalies.push(format!(
                "Low average health factor: {}%",
                current_metrics.average_health_factor / 100
            ));
        }

        // Check for very high utilization
        if current_metrics.protocol_utilization_rate > 9000 {
            // Above 90%
            anomalies.push(format!(
                "Very high utilization: {}%",
                current_metrics.protocol_utilization_rate / 100
            ));
        }

        anomalies
    }
}

/// PDA seeds for metrics accounts
pub const PROTOCOL_METRICS_SEED: &[u8] = b"protocol_metrics";
pub const RESERVE_METRICS_SEED: &[u8] = b"reserve_metrics";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_factor_calculation() {
        let health_factors = vec![12000, 15000, 11000, 13000]; // 120%, 150%, 110%, 130%
        let average = MetricsAggregator::calculate_average_health_factor(&health_factors);
        assert_eq!(average, 12750); // 127.5%
    }

    #[test]
    fn test_utilization_calculation() {
        let utilization = MetricsAggregator::calculate_protocol_utilization(1000000, 800000);
        assert_eq!(utilization, 8000); // 80%
    }

    #[test]
    fn test_anomaly_detection() {
        let previous = ProtocolMetrics {
            version: 1,
            market: Pubkey::default(),
            total_value_locked_usd: 1000000,
            total_borrowed_usd: 500000,
            total_fees_collected_usd: 10000,
            active_users: 100,
            active_reserves: 5,
            liquidations_24h: 10,
            average_health_factor: 12000,
            protocol_utilization_rate: 5000,
            last_update_timestamp: 0,
            last_update_slot: 0,
            reserved: [0; 128],
        };

        let current = ProtocolMetrics {
            total_value_locked_usd: 700000, // 30% drop
            liquidations_24h: 150,          // High liquidations
            average_health_factor: 10500,   // Low health factor
            ..previous
        };

        let anomalies = MetricsAggregator::detect_anomalies(&current, &previous);
        assert!(anomalies.len() >= 2); // Should detect multiple anomalies
    }
}
