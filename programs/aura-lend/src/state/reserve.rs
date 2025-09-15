use crate::constants::*;
use crate::error::LendingError;
use crate::utils::math::*;
use anchor_lang::prelude::*;

/// Reserve state account for each supported asset
/// Contains all information about a specific asset's lending pool
#[account]
pub struct Reserve {
    /// Version of the reserve account structure
    pub version: u8,

    /// Market this reserve belongs to
    pub market: Pubkey,

    /// Mint of the liquidity token (e.g., USDC, SOL)
    pub liquidity_mint: Pubkey,

    /// Mint of the collateral token (aTokens, e.g., aUSDC, aSOL)
    pub collateral_mint: Pubkey,

    /// Supply token account - holds the reserve's liquidity
    pub liquidity_supply: Pubkey,

    /// Fee receiver token account
    pub fee_receiver: Pubkey,

    /// Pyth price oracle account
    pub price_oracle: Pubkey,

    /// Pyth price feed ID for this asset
    pub oracle_feed_id: [u8; 32],

    /// Configuration parameters for this reserve
    pub config: ReserveConfig,

    /// Current state of the reserve (amounts, rates, etc.)
    pub state: ReserveState,

    /// Timestamp of the last state update
    pub last_update_timestamp: u64,

    /// Slot of the last state update
    pub last_update_slot: u64,

    /// Reentrancy guard - prevents concurrent operations
    pub reentrancy_guard: bool,

    /// Reserved space for future upgrades
    pub reserved: [u8; 255],
}

impl Reserve {
    /// Size of the Reserve account in bytes
    pub const SIZE: usize = 8 + // discriminator
        1 + // version
        32 + // market
        32 + // liquidity_mint
        32 + // collateral_mint
        32 + // liquidity_supply
        32 + // fee_receiver
        32 + // price_oracle
        std::mem::size_of::<ReserveConfig>() + // config
        std::mem::size_of::<ReserveState>() + // state
        8 + // last_update_timestamp
        8 + // last_update_slot
        256; // reserved

    /// Create a new reserve with the given parameters
    pub fn new(
        market: Pubkey,
        liquidity_mint: Pubkey,
        collateral_mint: Pubkey,
        liquidity_supply: Pubkey,
        fee_receiver: Pubkey,
        price_oracle: Pubkey,
        oracle_feed_id: [u8; 32],
        config: ReserveConfig,
    ) -> Result<Self> {
        let clock = Clock::get()?;

        Ok(Self {
            version: PROGRAM_VERSION,
            market,
            liquidity_mint,
            collateral_mint,
            liquidity_supply,
            fee_receiver,
            price_oracle,
            oracle_feed_id,
            config,
            state: ReserveState::default(),
            last_update_timestamp: clock.unix_timestamp as u64,
            last_update_slot: clock.slot,
            reentrancy_guard: false,
            reserved: [0; 255],
        })
    }

    /// Update interest rates and accrued interest
    pub fn update_interest(&mut self, current_slot: u64) -> Result<()> {
        if current_slot <= self.last_update_slot {
            return Ok(()); // Already updated or invalid slot
        }

        let slots_elapsed = current_slot - self.last_update_slot;

        // Calculate current utilization rate
        let utilization_rate =
            Rate::utilization_rate(self.state.total_borrows, self.state.available_liquidity)?;

        // Calculate new borrow interest rate
        let borrow_rate = Rate::calculate_interest_rate(
            self.config.base_borrow_rate_bps,
            self.config.borrow_rate_multiplier_bps,
            self.config.jump_rate_multiplier_bps,
            self.config.optimal_utilization_rate_bps,
            utilization_rate,
        )?;

        // Calculate supply interest rate (borrow rate * utilization * (1 - protocol fee))
        let protocol_fee_rate = Decimal::from_scaled_val(
            (self.config.protocol_fee_bps as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(BASIS_POINTS_PRECISION as u128)
                .ok_or(LendingError::DivisionByZero)?,
        );

        let fee_complement = Decimal::one().try_sub(protocol_fee_rate)?;
        let supply_rate = borrow_rate
            .try_mul(utilization_rate)?
            .try_mul(fee_complement)?;

        // Compound interest over the time period
        let time_fraction = Decimal::from_scaled_val(
            (slots_elapsed as u128)
                .checked_mul(PRECISION as u128)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(SLOTS_PER_YEAR as u128)
                .ok_or(LendingError::DivisionByZero)?,
        );

        // Update borrow interest
        if !borrow_rate.is_zero() && self.state.total_borrows > 0 {
            let borrow_interest = Rate::compound_interest(
                Decimal::from_integer(self.state.total_borrows)?,
                borrow_rate,
                SLOTS_PER_YEAR / 365, // Daily compounding
                time_fraction,
            )?;

            let interest_earned =
                borrow_interest.try_sub(Decimal::from_integer(self.state.total_borrows)?)?;
            let _interest_amount = interest_earned.try_floor_u64()?;

            self.state.total_borrows = borrow_interest.try_floor_u64()?;

            // Protocol fee on interest
            let protocol_fee = interest_earned
                .try_mul(protocol_fee_rate)?
                .try_floor_u64()?;
            self.state.accumulated_protocol_fees = self
                .state
                .accumulated_protocol_fees
                .checked_add(protocol_fee)
                .ok_or(LendingError::MathOverflow)?;
        }

        // Update supply interest (collateral exchange rate)
        if !supply_rate.is_zero() && self.state.total_liquidity > 0 {
            let supply_interest = Rate::compound_interest(
                Decimal::from_integer(self.state.total_liquidity)?,
                supply_rate,
                SLOTS_PER_YEAR / 365, // Daily compounding
                time_fraction,
            )?;

            self.state.total_liquidity = supply_interest.try_floor_u64()?;
        }

        // Update stored rates
        self.state.current_borrow_rate = borrow_rate;
        self.state.current_supply_rate = supply_rate;
        self.state.current_utilization_rate = utilization_rate;

        // Update timestamps
        self.last_update_slot = current_slot;
        self.last_update_timestamp = Clock::get()?.unix_timestamp as u64;

        Ok(())
    }

    /// Calculate the exchange rate between collateral and liquidity
    pub fn collateral_exchange_rate(&self) -> Result<Decimal> {
        if self.state.collateral_mint_supply == 0 {
            return Ok(Decimal::one());
        }

        let total_liquidity = Decimal::from_integer(self.state.total_liquidity);
        let collateral_supply = Decimal::from_integer(self.state.collateral_mint_supply);

        total_liquidity?.try_div(collateral_supply?)
    }

    /// Calculate collateral tokens to mint for a liquidity deposit
    pub fn liquidity_to_collateral(&self, liquidity_amount: u64) -> Result<u64> {
        if self.state.collateral_mint_supply == 0 {
            return Ok(liquidity_amount); // 1:1 for first deposit
        }

        let exchange_rate = self.collateral_exchange_rate()?;
        let liquidity_decimal = Decimal::from_integer(liquidity_amount);

        liquidity_decimal?.try_div(exchange_rate)?.try_floor_u64()
    }

    /// Calculate liquidity tokens to withdraw for collateral redemption
    pub fn collateral_to_liquidity(&self, collateral_amount: u64) -> Result<u64> {
        let exchange_rate = self.collateral_exchange_rate()?;
        let collateral_decimal = Decimal::from_integer(collateral_amount);

        collateral_decimal?.try_mul(exchange_rate)?.try_floor_u64()
    }

    /// Check if the reserve needs to be refreshed
    pub fn is_stale(&self, current_slot: u64) -> bool {
        current_slot.saturating_sub(self.last_update_slot) > MAX_ORACLE_STALENESS_SLOTS
    }

    /// Add liquidity to the reserve
    pub fn add_liquidity(&mut self, amount: u64) -> Result<()> {
        self.state.available_liquidity = self
            .state
            .available_liquidity
            .checked_add(amount)
            .ok_or(LendingError::MathOverflow)?;

        self.state.total_liquidity = self
            .state
            .total_liquidity
            .checked_add(amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(())
    }

    /// Remove liquidity from the reserve
    pub fn remove_liquidity(&mut self, amount: u64) -> Result<()> {
        if self.state.available_liquidity < amount {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.state.available_liquidity = self
            .state
            .available_liquidity
            .checked_sub(amount)
            .ok_or(LendingError::MathUnderflow)?;

        self.state.total_liquidity = self
            .state
            .total_liquidity
            .checked_sub(amount)
            .ok_or(LendingError::MathUnderflow)?;

        Ok(())
    }

    /// Add a borrow to the reserve
    pub fn add_borrow(&mut self, amount: u64) -> Result<()> {
        if self.state.available_liquidity < amount {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.state.available_liquidity = self
            .state
            .available_liquidity
            .checked_sub(amount)
            .ok_or(LendingError::MathUnderflow)?;

        self.state.total_borrows = self
            .state
            .total_borrows
            .checked_add(amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(())
    }

    /// Repay a borrow to the reserve
    pub fn repay_borrow(&mut self, amount: u64) -> Result<()> {
        let actual_repay = std::cmp::min(amount, self.state.total_borrows);

        self.state.available_liquidity = self
            .state
            .available_liquidity
            .checked_add(actual_repay)
            .ok_or(LendingError::MathOverflow)?;

        self.state.total_borrows = self
            .state
            .total_borrows
            .checked_sub(actual_repay)
            .ok_or(LendingError::MathUnderflow)?;

        Ok(())
    }

    /// Atomic lock operation to prevent reentrancy - checks and sets in single operation
    pub fn try_lock(&mut self) -> Result<()> {
        // Atomic check-and-set operation
        match self.reentrancy_guard {
            false => {
                self.reentrancy_guard = true;
                Ok(())
            }
            true => Err(LendingError::OperationInProgress.into()),
        }
    }

    /// Unlock the reserve after operation completion with validation
    pub fn unlock(&mut self) -> Result<()> {
        if !self.reentrancy_guard {
            return Err(LendingError::InvalidUnlockOperation.into());
        }
        self.reentrancy_guard = false;
        Ok(())
    }

    /// Check if reserve is currently locked
    pub fn is_locked(&self) -> bool {
        self.reentrancy_guard
    }

    /// Force unlock (emergency only - requires admin authority)
    pub fn force_unlock(&mut self) {
        self.reentrancy_guard = false;
    }
}

/// Configuration parameters for a reserve
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ReserveConfig {
    /// Loan-to-value ratio in basis points (max borrowable amount / collateral value)
    pub loan_to_value_ratio_bps: u64,

    /// Liquidation threshold in basis points (liquidation trigger / collateral value)
    pub liquidation_threshold_bps: u64,

    /// Liquidation penalty in basis points (bonus for liquidators)
    pub liquidation_penalty_bps: u64,

    /// Base borrow rate in basis points (annual)
    pub base_borrow_rate_bps: u64,

    /// Borrow rate multiplier in basis points
    pub borrow_rate_multiplier_bps: u64,

    /// Jump rate multiplier in basis points (kicks in after optimal utilization)
    pub jump_rate_multiplier_bps: u64,

    /// Optimal utilization rate in basis points
    pub optimal_utilization_rate_bps: u64,

    /// Protocol fee in basis points (taken from interest)
    pub protocol_fee_bps: u64,

    /// Maximum borrow rate in basis points
    pub max_borrow_rate_bps: u64,

    /// Asset decimals (6 for USDC, 9 for SOL, etc.)
    pub decimals: u8,

    /// Reserve flags
    pub flags: ReserveConfigFlags,
}

/// Current state of a reserve
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ReserveState {
    /// Total liquidity available for borrowing
    pub available_liquidity: u64,

    /// Total amount borrowed from this reserve
    pub total_borrows: u64,

    /// Total liquidity in the reserve (available + borrowed)
    pub total_liquidity: u64,

    /// Total supply of collateral tokens (aTokens)
    pub collateral_mint_supply: u64,

    /// Current borrow interest rate (annual)
    pub current_borrow_rate: Decimal,

    /// Current supply interest rate (annual)
    pub current_supply_rate: Decimal,

    /// Current utilization rate
    pub current_utilization_rate: Decimal,

    /// Protocol fees accumulated but not yet collected
    pub accumulated_protocol_fees: u64,
}

/// Reserve configuration flags
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct ReserveConfigFlags {
    bits: u32,
}

impl ReserveConfigFlags {
    /// Deposits are disabled
    pub const DEPOSITS_DISABLED: Self = Self { bits: 1 << 0 };

    /// Withdrawals are disabled
    pub const WITHDRAWALS_DISABLED: Self = Self { bits: 1 << 1 };

    /// Borrowing is disabled
    pub const BORROWING_DISABLED: Self = Self { bits: 1 << 2 };

    /// Repayments are disabled
    pub const REPAYMENTS_DISABLED: Self = Self { bits: 1 << 3 };

    /// Liquidations are disabled
    pub const LIQUIDATIONS_DISABLED: Self = Self { bits: 1 << 4 };

    /// Reserve can be used as collateral
    pub const COLLATERAL_ENABLED: Self = Self { bits: 1 << 5 };

    pub fn contains(&self, flag: Self) -> bool {
        (self.bits & flag.bits) == flag.bits
    }
}

/// Parameters for initializing a reserve
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeReserveParams {
    pub liquidity_mint: Pubkey,
    pub price_oracle: Pubkey,
    pub oracle_feed_id: [u8; 32], // Pyth or Switchboard feed ID
    pub config: ReserveConfig,
}

/// Parameters for updating reserve configuration
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateReserveConfigParams {
    pub config: ReserveConfig,
}
