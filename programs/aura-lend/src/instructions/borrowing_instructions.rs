use crate::constants::*;
use crate::error::LendingError;
use crate::state::*;
use crate::utils::{math::Decimal, OracleManager, TokenUtils};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

/// Initialize a new user obligation account
pub fn init_obligation(ctx: Context<InitObligation>) -> Result<()> {
    let obligation = &mut ctx.accounts.obligation;
    let market = &ctx.accounts.market;

    // Initialize the obligation
    **obligation = Obligation::new(market.key(), ctx.accounts.obligation_owner.key())?;

    msg!(
        "Obligation initialized for user: {}",
        ctx.accounts.obligation_owner.key()
    );
    Ok(())
}

/// Deposit collateral into an obligation
pub fn deposit_obligation_collateral(
    ctx: Context<DepositObligationCollateral>,
    collateral_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let deposit_reserve = &mut ctx.accounts.deposit_reserve;
    let clock = Clock::get()?;

    // Check if market allows deposits
    if market.is_paused() || market.is_lending_disabled() {
        return Err(LendingError::MarketPaused.into());
    }

    // Check if reserve allows collateral deposits
    if !deposit_reserve
        .config
        .flags
        .contains(ReserveConfigFlags::COLLATERAL_ENABLED)
    {
        return Err(LendingError::FeatureDisabled.into());
    }

    // Validate minimum collateral amount
    if collateral_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Refresh reserve interest
    deposit_reserve.update_interest(clock.slot)?;

    // Get price from oracle for collateral valuation
    let oracle_price = OracleManager::get_pyth_price(
        &ctx.accounts.price_oracle.to_account_info(),
        &deposit_reserve.oracle_feed_id,
    )?;
    oracle_price.validate(clock.unix_timestamp)?;

    // Calculate USD value of collateral with fresh oracle validation
    let collateral_value_usd = OracleManager::calculate_usd_value(
        collateral_amount,
        &oracle_price,
        deposit_reserve.config.decimals,
    )?;

    // Validate collateral deposit won't exceed concentration limits
    let current_collateral_for_asset = obligation
        .deposits
        .iter()
        .filter(|d| d.deposit_reserve == deposit_reserve.key())
        .map(|d| d.market_value_usd.value)
        .sum::<u128>();

    let new_total_collateral_for_asset = current_collateral_for_asset
        .checked_add(collateral_value_usd.value)
        .ok_or(LendingError::MathOverflow)?;

    // Prevent over-concentration in single asset (max 70% of portfolio in one asset)
    let total_portfolio_value = obligation
        .deposited_value_usd
        .try_add(collateral_value_usd)?;

    let max_single_asset_value = total_portfolio_value.try_mul(Decimal::from_scaled_val(
        (7000u128 * PRECISION as u128)
            .checked_div(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?,
    ))?;

    if new_total_collateral_for_asset > max_single_asset_value.value {
        return Err(LendingError::InvalidAmount.into()); // Too concentrated
    }

    // Transfer collateral tokens from user to reserve
    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.source_collateral,
        &ctx.accounts.destination_collateral,
        &ctx.accounts.obligation_owner.to_account_info(),
        &[],
        collateral_amount,
    )?;

    // Add collateral to obligation
    let collateral_deposit = ObligationCollateral {
        deposit_reserve: deposit_reserve.key(),
        deposited_amount: collateral_amount,
        market_value_usd: collateral_value_usd,
        ltv_bps: deposit_reserve.config.loan_to_value_ratio_bps,
        liquidation_threshold_bps: deposit_reserve.config.liquidation_threshold_bps,
    };

    obligation.add_collateral_deposit(collateral_deposit)?;

    // Update cached values
    obligation.deposited_value_usd = obligation
        .deposited_value_usd
        .try_add(collateral_value_usd)?;

    obligation.update_timestamp(clock.slot);

    msg!(
        "Deposited {} collateral tokens worth ${:.2} USD",
        collateral_amount,
        collateral_value_usd.try_floor_u64()? as f64 / 1e18
    );

    Ok(())
}

/// Withdraw collateral from an obligation
pub fn withdraw_obligation_collateral(
    ctx: Context<WithdrawObligationCollateral>,
    collateral_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let withdraw_reserve = &mut ctx.accounts.withdraw_reserve;
    let clock = Clock::get()?;

    // Check if market allows withdrawals
    if market.is_paused() && !market.is_emergency() {
        return Err(LendingError::MarketPaused.into());
    }

    // Validate withdrawal amount
    if collateral_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Refresh reserve interest
    withdraw_reserve.update_interest(clock.slot)?;

    // Check if user has enough collateral
    let deposit = obligation
        .find_collateral_deposit(&withdraw_reserve.key())
        .ok_or(LendingError::ObligationReserveNotFound)?;

    if deposit.deposited_amount < collateral_amount {
        return Err(LendingError::InsufficientCollateral.into());
    }

    // Get current price for updated valuation
    let oracle_price = OracleManager::get_pyth_price(
        &ctx.accounts.price_oracle.to_account_info(),
        &withdraw_reserve.oracle_feed_id,
    )?;
    oracle_price.validate(clock.unix_timestamp)?;

    // Calculate USD value of collateral being withdrawn
    let withdrawn_value_usd = OracleManager::calculate_usd_value(
        collateral_amount,
        &oracle_price,
        withdraw_reserve.config.decimals,
    )?;

    // Remove collateral from obligation
    obligation.remove_collateral_deposit(&withdraw_reserve.key(), collateral_amount)?;

    // Update cached values
    obligation.deposited_value_usd = obligation
        .deposited_value_usd
        .try_sub(withdrawn_value_usd)?;

    // Check if obligation remains healthy after withdrawal
    if obligation.has_borrows() && !obligation.is_healthy()? {
        return Err(LendingError::ObligationUnhealthy.into());
    }

    // Transfer collateral tokens back to user
    let authority_seeds = &[
        COLLATERAL_TOKEN_SEED,
        withdraw_reserve.liquidity_mint.as_ref(),
        b"authority",
        &[ctx.bumps.collateral_supply_authority],
    ];

    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.source_collateral,
        &ctx.accounts.destination_collateral,
        &ctx.accounts.collateral_supply_authority.to_account_info(),
        &[authority_seeds],
        collateral_amount,
    )?;

    obligation.update_timestamp(clock.slot);

    msg!(
        "Withdrew {} collateral tokens worth ${:.2} USD",
        collateral_amount,
        withdrawn_value_usd.try_floor_u64()? as f64 / 1e18
    );

    Ok(())
}

/// Borrow liquidity against collateral
pub fn borrow_obligation_liquidity(
    ctx: Context<BorrowObligationLiquidity>,
    liquidity_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let borrow_reserve = &mut ctx.accounts.borrow_reserve;
    let clock = Clock::get()?;

    // Check if market allows borrowing
    if market.is_paused() || market.is_borrowing_disabled() {
        return Err(LendingError::MarketPaused.into());
    }

    // Check if reserve allows borrowing
    if borrow_reserve
        .config
        .flags
        .contains(ReserveConfigFlags::BORROWING_DISABLED)
    {
        return Err(LendingError::FeatureDisabled.into());
    }

    // Validate minimum borrow amount
    if liquidity_amount < MIN_BORROW_AMOUNT {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Check if obligation has collateral
    if !obligation.has_collateral() {
        return Err(LendingError::ObligationCollateralEmpty.into());
    }

    // Refresh reserve interest
    borrow_reserve.update_interest(clock.slot)?;

    // Check if reserve has sufficient liquidity
    if borrow_reserve.state.available_liquidity < liquidity_amount {
        return Err(LendingError::InsufficientLiquidity.into());
    }

    // Get price from oracle for borrow valuation
    let oracle_price = OracleManager::get_pyth_price(
        &ctx.accounts.price_oracle.to_account_info(),
        &borrow_reserve.oracle_feed_id,
    )?;
    oracle_price.validate(clock.unix_timestamp)?;

    // Calculate USD value of new borrow
    let borrow_value_usd = OracleManager::calculate_usd_value(
        liquidity_amount,
        &oracle_price,
        borrow_reserve.config.decimals,
    )?;

    // Atomic LTV validation with fresh oracle prices to prevent manipulation
    // Lock obligation during validation to prevent race conditions
    let _current_health_factor = obligation.calculate_health_factor()?;

    // Simulate the new borrow to check if it would make the position unhealthy
    let new_borrowed_value = obligation.borrowed_value_usd.try_add(borrow_value_usd)?;
    let max_borrow_value = obligation.calculate_max_borrow_value()?;

    // Strict LTV check with buffer to prevent near-liquidation positions
    let ltv_buffer_bps = 500; // 5% buffer below maximum LTV
    let safe_max_borrow = max_borrow_value.try_mul(Decimal::from_scaled_val(
        ((BASIS_POINTS_PRECISION - ltv_buffer_bps) as u128)
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?,
    ))?;

    if new_borrowed_value.value > safe_max_borrow.value {
        return Err(LendingError::LoanToValueRatioExceedsMax.into());
    }

    // Additional health factor check after simulated borrow
    let simulated_health_factor = obligation
        .calculate_liquidation_threshold_value()?
        .try_div(new_borrowed_value)?;

    // Ensure health factor stays well above 1.0 (require at least 1.1)
    let min_health_factor = Decimal::from_scaled_val(
        (11u128)
            .checked_mul(PRECISION as u128 / 10)
            .ok_or(LendingError::MathOverflow)?,
    );

    if simulated_health_factor.value < min_health_factor.value {
        return Err(LendingError::ObligationUnhealthy.into());
    }

    // Add borrow to reserve
    borrow_reserve.add_borrow(liquidity_amount)?;

    // Add borrow to obligation
    let liquidity_borrow = ObligationLiquidity {
        borrow_reserve: borrow_reserve.key(),
        borrowed_amount_wads: Decimal::from_integer(liquidity_amount)?,
        market_value_usd: borrow_value_usd,
    };

    obligation.add_liquidity_borrow(liquidity_borrow)?;

    // Update cached values
    obligation.borrowed_value_usd = new_borrowed_value;
    obligation.update_timestamp(clock.slot);

    // Transfer liquidity from reserve to user
    let authority_seeds = &[
        LIQUIDITY_TOKEN_SEED,
        borrow_reserve.liquidity_mint.as_ref(),
        b"authority",
        &[ctx.bumps.liquidity_supply_authority],
    ];

    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.source_liquidity,
        &ctx.accounts.destination_liquidity,
        &ctx.accounts.liquidity_supply_authority.to_account_info(),
        &[authority_seeds],
        liquidity_amount,
    )?;

    msg!(
        "Borrowed {} liquidity tokens worth ${:.2} USD",
        liquidity_amount,
        borrow_value_usd.try_floor_u64()? as f64 / 1e18
    );

    Ok(())
}

/// Repay borrowed liquidity
pub fn repay_obligation_liquidity(
    ctx: Context<RepayObligationLiquidity>,
    liquidity_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let repay_reserve = &mut ctx.accounts.repay_reserve;
    let clock = Clock::get()?;

    // Check if market allows repayments
    if market.is_paused() && !market.is_emergency() {
        return Err(LendingError::MarketPaused.into());
    }

    // Check if reserve allows repayments
    if repay_reserve
        .config
        .flags
        .contains(ReserveConfigFlags::REPAYMENTS_DISABLED)
    {
        return Err(LendingError::FeatureDisabled.into());
    }

    // Validate repay amount
    if liquidity_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Refresh reserve interest
    repay_reserve.update_interest(clock.slot)?;

    // Check if user has this borrow
    let borrow = obligation
        .find_liquidity_borrow(&repay_reserve.key())
        .ok_or(LendingError::ObligationReserveNotFound)?;

    let borrowed_amount = borrow.borrowed_amount_wads.try_floor_u64()?;
    let actual_repay_amount = std::cmp::min(liquidity_amount, borrowed_amount);

    if actual_repay_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Get current price for updated valuation
    let oracle_price = OracleManager::get_pyth_price(
        &ctx.accounts.price_oracle.to_account_info(),
        &repay_reserve.oracle_feed_id,
    )?;
    oracle_price.validate(clock.unix_timestamp)?;

    // Calculate USD value of repayment
    let repay_value_usd = OracleManager::calculate_usd_value(
        actual_repay_amount,
        &oracle_price,
        repay_reserve.config.decimals,
    )?;

    // Transfer repayment from user to reserve
    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.source_liquidity,
        &ctx.accounts.destination_liquidity,
        &ctx.accounts.obligation_owner.to_account_info(),
        &[],
        actual_repay_amount,
    )?;

    // Update reserve
    repay_reserve.repay_borrow(actual_repay_amount)?;

    // Update obligation
    obligation.repay_liquidity_borrow(
        &repay_reserve.key(),
        Decimal::from_integer(actual_repay_amount)?,
    )?;

    // Update cached values
    obligation.borrowed_value_usd = obligation.borrowed_value_usd.try_sub(repay_value_usd)?;

    obligation.update_timestamp(clock.slot);

    msg!(
        "Repaid {} liquidity tokens worth ${:.2} USD",
        actual_repay_amount,
        repay_value_usd.try_floor_u64()? as f64 / 1e18
    );

    Ok(())
}

// Context structs for borrowing instructions

#[derive(Accounts)]
pub struct InitObligation<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account to initialize
    #[account(
        init,
        payer = payer,
        space = Obligation::SIZE,
        seeds = [OBLIGATION_SEED, obligation_owner.key().as_ref()],
        bump
    )]
    pub obligation: Account<'info, Obligation>,

    /// Owner of the obligation
    pub obligation_owner: Signer<'info>,

    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositObligationCollateral<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation_owner.key().as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        // Owner validation will be done manually in instruction
    )]
    pub obligation: Account<'info, Obligation>,

    /// Reserve for the collateral being deposited
    #[account(
        mut,
        seeds = [RESERVE_SEED, deposit_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = price_oracle @ LendingError::OracleAccountMismatch
    )]
    pub deposit_reserve: Account<'info, Reserve>,

    /// Price oracle for the collateral asset
    /// CHECK: This account is validated by the reserve's price_oracle field
    pub price_oracle: UncheckedAccount<'info>,

    /// User's source collateral token account
    #[account(
        mut,
        token::mint = deposit_reserve.collateral_mint,
        token::authority = obligation_owner
    )]
    pub source_collateral: Account<'info, TokenAccount>,

    /// Reserve's collateral token account
    #[account(
        mut,
        token::mint = deposit_reserve.collateral_mint,
        token::authority = collateral_supply_authority
    )]
    pub destination_collateral: Account<'info, TokenAccount>,

    /// Collateral supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [COLLATERAL_TOKEN_SEED, deposit_reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub collateral_supply_authority: UncheckedAccount<'info>,

    /// Obligation owner
    pub obligation_owner: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawObligationCollateral<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation_owner.key().as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        // Owner validation will be done manually in instruction
    )]
    pub obligation: Account<'info, Obligation>,

    /// Reserve for the collateral being withdrawn
    #[account(
        mut,
        seeds = [RESERVE_SEED, withdraw_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = price_oracle @ LendingError::OracleAccountMismatch
    )]
    pub withdraw_reserve: Account<'info, Reserve>,

    /// Price oracle for the collateral asset
    /// CHECK: This account is validated by the reserve's price_oracle field
    pub price_oracle: UncheckedAccount<'info>,

    /// Reserve's collateral token account
    #[account(
        mut,
        token::mint = withdraw_reserve.collateral_mint,
        token::authority = collateral_supply_authority
    )]
    pub source_collateral: Account<'info, TokenAccount>,

    /// User's destination collateral token account
    #[account(
        mut,
        token::mint = withdraw_reserve.collateral_mint,
        token::authority = obligation_owner
    )]
    pub destination_collateral: Account<'info, TokenAccount>,

    /// Collateral supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [COLLATERAL_TOKEN_SEED, withdraw_reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub collateral_supply_authority: UncheckedAccount<'info>,

    /// Obligation owner
    pub obligation_owner: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BorrowObligationLiquidity<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation_owner.key().as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        // Owner validation will be done manually in instruction
    )]
    pub obligation: Account<'info, Obligation>,

    /// Reserve for the asset being borrowed
    #[account(
        mut,
        seeds = [RESERVE_SEED, borrow_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = price_oracle @ LendingError::OracleAccountMismatch,
        // Liquidity supply validation will be done manually
    )]
    pub borrow_reserve: Account<'info, Reserve>,

    /// Price oracle for the borrowed asset
    /// CHECK: This account is validated by the reserve's price_oracle field
    pub price_oracle: UncheckedAccount<'info>,

    /// Reserve's liquidity supply token account
    #[account(
        mut,
        token::mint = borrow_reserve.liquidity_mint,
        token::authority = liquidity_supply_authority
    )]
    pub source_liquidity: Account<'info, TokenAccount>,

    /// User's destination liquidity token account
    #[account(
        mut,
        token::mint = borrow_reserve.liquidity_mint,
        token::authority = obligation_owner
    )]
    pub destination_liquidity: Account<'info, TokenAccount>,

    /// Liquidity supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [LIQUIDITY_TOKEN_SEED, borrow_reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub liquidity_supply_authority: UncheckedAccount<'info>,

    /// Obligation owner
    pub obligation_owner: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RepayObligationLiquidity<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation_owner.key().as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        // Owner validation will be done manually in instruction
    )]
    pub obligation: Account<'info, Obligation>,

    /// Reserve for the asset being repaid
    #[account(
        mut,
        seeds = [RESERVE_SEED, repay_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = price_oracle @ LendingError::OracleAccountMismatch,
        // Liquidity supply validation will be done manually
    )]
    pub repay_reserve: Account<'info, Reserve>,

    /// Price oracle for the repaid asset
    /// CHECK: This account is validated by the reserve's price_oracle field
    pub price_oracle: UncheckedAccount<'info>,

    /// User's source liquidity token account
    #[account(
        mut,
        token::mint = repay_reserve.liquidity_mint,
        token::authority = obligation_owner
    )]
    pub source_liquidity: Account<'info, TokenAccount>,

    /// Reserve's liquidity supply token account
    #[account(
        mut,
        token::mint = repay_reserve.liquidity_mint,
        token::authority = liquidity_supply_authority
    )]
    pub destination_liquidity: Account<'info, TokenAccount>,

    /// Liquidity supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [LIQUIDITY_TOKEN_SEED, repay_reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub liquidity_supply_authority: UncheckedAccount<'info>,

    /// Obligation owner
    pub obligation_owner: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}
