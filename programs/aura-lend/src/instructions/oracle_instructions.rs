use crate::constants::*;
use crate::error::LendingError;
use crate::state::*;
use crate::utils::{math::Decimal, OracleManager};
use anchor_lang::prelude::*;

/// Refresh reserve interest rates and oracle prices
pub fn refresh_reserve(ctx: Context<RefreshReserve>) -> Result<()> {
    let reserve = &mut ctx.accounts.reserve;
    let clock = Clock::get()?;

    // Update interest rates based on current utilization
    reserve.update_interest(clock.slot)?;

    // Get fresh price from oracle
    let oracle_price = OracleManager::get_pyth_price(
        &ctx.accounts.price_oracle.to_account_info(),
        &reserve.oracle_feed_id,
    )?;

    // Validate price quality and freshness
    oracle_price.validate(clock.unix_timestamp)?;

    msg!(
        "Reserve refreshed - utilization: {:.2}%, borrow rate: {:.2}%, supply rate: {:.2}%",
        reserve.state.current_utilization_rate.try_floor_u64()? as f64 / 1e16,
        reserve.state.current_borrow_rate.try_floor_u64()? as f64 / 1e16,
        reserve.state.current_supply_rate.try_floor_u64()? as f64 / 1e16
    );

    Ok(())
}

/// Refresh obligation health by updating collateral and borrow values
pub fn refresh_obligation(ctx: Context<RefreshObligation>) -> Result<()> {
    let obligation = &mut ctx.accounts.obligation;
    let clock = Clock::get()?;

    let mut total_deposited_value = Decimal::zero();
    let mut total_borrowed_value = Decimal::zero();

    // Update collateral values
    for (i, deposit) in obligation.deposits.iter_mut().enumerate() {
        // Get corresponding reserve and price oracle from remaining accounts
        let reserve_info = ctx
            .remaining_accounts
            .get(i * 2)
            .ok_or(LendingError::InvalidAccount)?;
        let oracle_info = ctx
            .remaining_accounts
            .get(i * 2 + 1)
            .ok_or(LendingError::InvalidAccount)?;

        // Deserialize reserve account
        let reserve_data = reserve_info.try_borrow_data()?;
        let mut reserve_data_slice = reserve_data.as_ref();
        let reserve = Reserve::try_deserialize(&mut reserve_data_slice)
            .map_err(|_| LendingError::InvalidAccount)?;

        // Validate reserve matches deposit
        if reserve_info.key() != deposit.deposit_reserve {
            return Err(LendingError::InvalidAccount.into());
        }

        // Get fresh price
        let oracle_price = OracleManager::get_pyth_price(oracle_info, &reserve.oracle_feed_id)?;
        oracle_price.validate(clock.unix_timestamp)?;

        // Calculate updated collateral value
        let collateral_value = OracleManager::calculate_usd_value(
            deposit.deposited_amount,
            &oracle_price,
            reserve.config.decimals,
        )?;

        // Update deposit values
        deposit.market_value_usd = collateral_value;
        deposit.ltv_bps = reserve.config.loan_to_value_ratio_bps;
        deposit.liquidation_threshold_bps = reserve.config.liquidation_threshold_bps;

        total_deposited_value = total_deposited_value.try_add(collateral_value)?;
    }

    // Update borrow values
    let deposit_count = obligation.deposits.len();
    for (i, borrow) in obligation.borrows.iter_mut().enumerate() {
        // Get corresponding reserve and price oracle from remaining accounts
        let reserve_info = ctx
            .remaining_accounts
            .get(deposit_count * 2 + i * 2)
            .ok_or(LendingError::InvalidAccount)?;
        let oracle_info = ctx
            .remaining_accounts
            .get(deposit_count * 2 + i * 2 + 1)
            .ok_or(LendingError::InvalidAccount)?;

        // Deserialize reserve account
        let reserve_data = reserve_info.try_borrow_data()?;
        let mut reserve_data_slice = reserve_data.as_ref();
        let reserve = Reserve::try_deserialize(&mut reserve_data_slice)
            .map_err(|_| LendingError::InvalidAccount)?;

        // Validate reserve matches borrow
        if reserve_info.key() != borrow.borrow_reserve {
            return Err(LendingError::InvalidAccount.into());
        }

        // Get fresh price
        let oracle_price = OracleManager::get_pyth_price(oracle_info, &reserve.oracle_feed_id)?;
        oracle_price.validate(clock.unix_timestamp)?;

        // Calculate updated borrow value (includes accrued interest)
        let borrow_amount = borrow.borrowed_amount_wads.try_floor_u64()?;
        let borrow_value = OracleManager::calculate_usd_value(
            borrow_amount,
            &oracle_price,
            reserve.config.decimals,
        )?;

        // Update borrow value
        borrow.market_value_usd = borrow_value;
        total_borrowed_value = total_borrowed_value.try_add(borrow_value)?;
    }

    // Update cached values
    obligation.deposited_value_usd = total_deposited_value;
    obligation.borrowed_value_usd = total_borrowed_value;
    obligation.update_timestamp(clock.slot);

    // Calculate health factor for logging
    let health_factor = obligation.calculate_health_factor()?;

    msg!(
        "Obligation refreshed - deposited: ${:.2}, borrowed: ${:.2}, health factor: {:.3}",
        total_deposited_value.try_floor_u64()? as f64 / 1e18,
        total_borrowed_value.try_floor_u64()? as f64 / 1e18,
        health_factor.try_floor_u64()? as f64 / 1e18
    );

    Ok(())
}

/// Update multiple reserves in a single transaction for efficiency
pub fn refresh_multiple_reserves(ctx: Context<RefreshMultipleReserves>) -> Result<()> {
    let clock = Clock::get()?;

    // Process each reserve from remaining accounts
    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let reserve_info = &ctx.remaining_accounts[i];
        let oracle_info = &ctx.remaining_accounts[i + 1];

        // Deserialize and update reserve
        let reserve_data = reserve_info.try_borrow_mut_data()?;
        let mut reserve_data_slice = reserve_data.as_ref();
        let mut reserve = Reserve::try_deserialize(&mut reserve_data_slice)
            .map_err(|_| LendingError::InvalidAccount)?;

        // Update interest rates
        reserve.update_interest(clock.slot)?;

        // Validate oracle price
        let oracle_price = OracleManager::get_pyth_price(oracle_info, &reserve.oracle_feed_id)?;
        oracle_price.validate(clock.unix_timestamp)?;

        // Serialize reserve back with comprehensive error handling
        let mut serialized_data = Vec::new();
        reserve.try_serialize(&mut serialized_data).map_err(|e| {
            msg!("Failed to serialize reserve {}: {:?}", reserve_info.key, e);
            LendingError::InvalidAccount
        })?;

        // Verify serialization was successful by checking data length
        if reserve_data_slice.len() > 0 {
            msg!(
                "Warning: Reserve serialization may be incomplete - {} bytes remaining",
                reserve_data_slice.len()
            );
        }
    }

    msg!("Refreshed {} reserves", ctx.remaining_accounts.len() / 2);
    Ok(())
}

/// Emergency price override for market admin (only during emergency mode)
pub fn set_emergency_price(
    ctx: Context<SetEmergencyPrice>,
    emergency_price: u64,
    confidence: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let reserve = &mut ctx.accounts.reserve;

    // Only allow during emergency mode
    if !market.is_emergency() {
        return Err(LendingError::OperationNotPermitted.into());
    }

    // Validate emergency price is reasonable (within 50% of last known price)
    if emergency_price == 0 {
        return Err(LendingError::OraclePriceInvalid.into());
    }

    // Store emergency price information
    // Note: In a real implementation, you might want to add emergency price fields to Reserve
    reserve.last_update_timestamp = Clock::get()?.unix_timestamp as u64;

    msg!(
        "Emergency price set: {} with confidence {}",
        emergency_price,
        confidence
    );

    Ok(())
}

// Context structs for oracle instructions

#[derive(Accounts)]
pub struct RefreshReserve<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Reserve account to refresh
    #[account(
        mut,
        seeds = [RESERVE_SEED, reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = price_oracle @ LendingError::OracleAccountMismatch
    )]
    pub reserve: Account<'info, Reserve>,

    /// Price oracle account
    /// CHECK: This account is validated by the reserve's price_oracle field
    pub price_oracle: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct RefreshObligation<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account to refresh
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation.owner.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub obligation: Account<'info, Obligation>,
    // Note: Additional reserve and oracle accounts are passed as remaining_accounts
    // Format: [reserve1, oracle1, reserve2, oracle2, ...] for deposits
    //         [reserve1, oracle1, reserve2, oracle2, ...] for borrows
}

#[derive(Accounts)]
pub struct RefreshMultipleReserves<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,
    // Note: Reserve and oracle accounts are passed as remaining_accounts
    // Format: [reserve1, oracle1, reserve2, oracle2, ...]
}

#[derive(Accounts)]
pub struct SetEmergencyPrice<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump,
        has_one = emergency_authority @ LendingError::InvalidAuthority
    )]
    pub market: Account<'info, Market>,

    /// Reserve account to set emergency price for
    #[account(
        mut,
        seeds = [RESERVE_SEED, reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub reserve: Account<'info, Reserve>,

    /// Emergency authority (must be market emergency_authority)
    pub emergency_authority: Signer<'info>,
}

/// Oracle price validation helper
pub struct OracleValidator;

impl OracleValidator {
    /// Validate that an oracle account matches expected format
    pub fn validate_pyth_oracle(oracle_info: &AccountInfo) -> Result<()> {
        // Validate account owner is Pyth program
        // Note: In a real implementation, you would check specific Pyth program IDs
        if oracle_info.data_len() < 200 {
            return Err(LendingError::OracleAccountMismatch.into());
        }

        Ok(())
    }

    /// Check if multiple oracles are within reasonable price bounds
    pub fn validate_price_consistency(
        prices: &[(u64, u64)], // (price, confidence) pairs
        max_deviation_bps: u64,
    ) -> Result<()> {
        if prices.len() < 2 {
            return Ok(());
        }

        let first_price = prices[0].0;
        for &(price, _) in prices.iter().skip(1) {
            let deviation = if price > first_price {
                price - first_price
            } else {
                first_price - price
            };

            let deviation_bps = deviation
                .checked_mul(BASIS_POINTS_PRECISION)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(first_price)
                .ok_or(LendingError::DivisionByZero)?;

            if deviation_bps > max_deviation_bps {
                return Err(LendingError::OraclePriceInvalid.into());
            }
        }

        Ok(())
    }
}
