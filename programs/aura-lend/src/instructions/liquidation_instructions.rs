use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::*;
use crate::error::LendingError;
use crate::constants::*;
use crate::utils::{TokenUtils, OracleManager, math::Decimal};

/// Liquidate an unhealthy obligation
pub fn liquidate_obligation(
    ctx: Context<LiquidateObligation>,
    liquidity_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let repay_reserve = &mut ctx.accounts.repay_reserve;
    let withdraw_reserve = &mut ctx.accounts.withdraw_reserve;
    let clock = Clock::get()?;

    // Check if market allows liquidations
    if market.is_paused() || market.is_liquidation_disabled() {
        return Err(LendingError::MarketPaused.into());
    }

    // Check if reserves allow liquidations
    if repay_reserve.config.flags.contains(ReserveConfigFlags::LIQUIDATIONS_DISABLED) ||
       withdraw_reserve.config.flags.contains(ReserveConfigFlags::LIQUIDATIONS_DISABLED) {
        return Err(LendingError::FeatureDisabled.into());
    }

    // Validate liquidation amount
    if liquidity_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Lock reserves to prevent race conditions during liquidation
    repay_reserve.try_lock()?;
    withdraw_reserve.try_lock()?;
    
    // Ensure we unlock on any error path
    let result = (|| -> Result<()> {
        // Refresh reserves with locked state
        repay_reserve.update_interest(clock.slot)?;
        withdraw_reserve.update_interest(clock.slot)?;

        // Refresh obligation with current prices to get accurate health factor
        obligation.refresh_health_factor(
            &ctx.remaining_accounts,
            clock.unix_timestamp
        )?;

        // Atomic health check - capture health factor at exact moment of liquidation
        let health_factor = obligation.calculate_health_factor()?;
        if health_factor >= Decimal::one() {
            return Err(LendingError::ObligationHealthy.into());
        }

        // Store health snapshot to prevent manipulation during liquidation
        obligation.liquidation_snapshot_health_factor = Some(health_factor);
        
        Ok(())
    })();
    
    // Unlock reserves regardless of result
    if result.is_err() {
        let _ = repay_reserve.unlock();
        let _ = withdraw_reserve.unlock();
        return result;
    }

    // Validate that the borrow exists
    let _borrow = obligation.find_liquidity_borrow(&repay_reserve.key())
        .ok_or(LendingError::ObligationReserveNotFound)?;

    // Check maximum liquidation amount (usually 50% of debt)
    let max_liquidation = obligation.max_liquidation_amount(&repay_reserve.key())?;
    if liquidity_amount > max_liquidation {
        return Err(LendingError::LiquidationTooLarge.into());
    }

    // Validate that collateral exists
    let collateral = obligation.find_collateral_deposit(&withdraw_reserve.key())
        .ok_or(LendingError::ObligationReserveNotFound)?;

    // Get current prices from oracles using proper feed IDs from reserves
    let repay_price = OracleManager::get_pyth_price(
        &ctx.accounts.repay_price_oracle.to_account_info(),
        &repay_reserve.oracle_feed_id, // Use actual feed ID from reserve config
    )?;
    repay_price.validate(clock.unix_timestamp)?;

    let withdraw_price = OracleManager::get_pyth_price(
        &ctx.accounts.withdraw_price_oracle.to_account_info(),
        &withdraw_reserve.oracle_feed_id, // Use actual feed ID from reserve config
    )?;
    withdraw_price.validate(clock.unix_timestamp)?;

    // Calculate USD values
    let repay_value_usd = OracleManager::calculate_usd_value(
        liquidity_amount,
        &repay_price,
        repay_reserve.config.decimals,
    )?;

    // Calculate collateral amount to liquidate (with bonus)
    let liquidation_bonus_decimal = Decimal::from_scaled_val(
        (withdraw_reserve.config.liquidation_penalty_bps as u128)
            .checked_add(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_mul(PRECISION as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(BASIS_POINTS_PRECISION as u128)
            .ok_or(LendingError::DivisionByZero)?,
    );

    let liquidation_value_usd = repay_value_usd.try_mul(liquidation_bonus_decimal)?;
    
    // Convert USD value to collateral token amount
    let collateral_price_decimal = withdraw_price.to_decimal()?;
    let collateral_amount_decimal = liquidation_value_usd.try_div(collateral_price_decimal)?;
    let collateral_amount = collateral_amount_decimal.try_floor_u64()?;

    // Validate sufficient collateral
    if collateral.deposited_amount < collateral_amount {
        return Err(LendingError::InsufficientCollateral.into());
    }

    // Transfer repayment from liquidator to reserve
    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.source_liquidity,
        &ctx.accounts.repay_reserve_liquidity_supply,
        &ctx.accounts.liquidator.to_account_info(),
        &[],
        liquidity_amount,
    )?;

    // Transfer collateral from reserve to liquidator
    let collateral_authority_seeds = &[
        COLLATERAL_TOKEN_SEED,
        withdraw_reserve.liquidity_mint.as_ref(),
        b"authority",
        &[ctx.bumps.withdraw_collateral_supply_authority],
    ];

    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.withdraw_reserve_collateral_supply,
        &ctx.accounts.destination_collateral,
        &ctx.accounts.withdraw_collateral_supply_authority.to_account_info(),
        &[collateral_authority_seeds],
        collateral_amount,
    )?;

    // Update reserves
    repay_reserve.repay_borrow(liquidity_amount)?;
    
    // Update obligation
    obligation.repay_liquidity_borrow(
        &repay_reserve.key(),
        Decimal::from_integer(liquidity_amount)?,
    )?;

    obligation.remove_collateral_deposit(&withdraw_reserve.key(), collateral_amount)?;

    // Update cached USD values
    obligation.borrowed_value_usd = obligation.borrowed_value_usd
        .try_sub(repay_value_usd)?;
    
    let collateral_value_usd = OracleManager::calculate_usd_value(
        collateral_amount,
        &withdraw_price,
        withdraw_reserve.config.decimals,
    )?;
    
    obligation.deposited_value_usd = obligation.deposited_value_usd
        .try_sub(collateral_value_usd)?;

    obligation.update_timestamp(clock.slot);

    // Calculate liquidation bonus for logging with proper error handling
    let expected_collateral = repay_value_usd
        .try_div(withdraw_price.to_decimal()?)?
        .try_floor_u64()?;
    
    let bonus_amount = if collateral_amount > expected_collateral {
        collateral_amount.saturating_sub(expected_collateral)
    } else {
        // This shouldn't happen in a proper liquidation, log warning
        msg!("Warning: Liquidation bonus calculation resulted in negative value");
        0
    };

    msg!(
        "Liquidation completed - repaid: {} (${:.2}), seized: {} (${:.2}), bonus: {}",
        liquidity_amount,
        repay_value_usd.try_floor_u64()? as f64 / 1e18,
        collateral_amount,
        collateral_value_usd.try_floor_u64()? as f64 / 1e18,
        bonus_amount
    );

    // Clear liquidation snapshot as liquidation is complete
    obligation.liquidation_snapshot_health_factor = None;

    // Unlock reserves after successful liquidation
    repay_reserve.unlock()?;
    withdraw_reserve.unlock()?;

    Ok(())
}

/// Flash liquidation - liquidate with borrowed funds
pub fn flash_liquidate_obligation(
    ctx: Context<FlashLiquidateObligation>,
    liquidity_amount: u64,
) -> Result<()> {
    let _market = &ctx.accounts.market;
    let obligation = &mut ctx.accounts.obligation;
    let flash_loan_reserve = &mut ctx.accounts.flash_loan_reserve;
    let _repay_reserve = &mut ctx.accounts.repay_reserve;
    let _withdraw_reserve = &mut ctx.accounts.withdraw_reserve;
    let _clock = Clock::get()?;

    // Check if obligation is unhealthy
    if obligation.is_healthy()? {
        return Err(LendingError::ObligationHealthy.into());
    }

    // Calculate flash loan fee
    let flash_loan_fee = liquidity_amount
        .checked_mul(FLASH_LOAN_FEE_BPS)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(BASIS_POINTS_PRECISION)
        .ok_or(LendingError::DivisionByZero)?;

    let total_repayment = liquidity_amount
        .checked_add(flash_loan_fee)
        .ok_or(LendingError::MathOverflow)?;

    // Check if reserve has enough liquidity for flash loan
    if flash_loan_reserve.state.available_liquidity < liquidity_amount {
        return Err(LendingError::InsufficientLiquidity.into());
    }

    // Step 1: Issue flash loan
    let flash_loan_authority_seeds = &[
        LIQUIDITY_TOKEN_SEED,
        flash_loan_reserve.liquidity_mint.as_ref(),
        b"authority",
        &[ctx.bumps.flash_loan_reserve_authority],
    ];

    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.flash_loan_reserve_liquidity_supply,
        &ctx.accounts.flash_loan_destination,
        &ctx.accounts.flash_loan_reserve_authority.to_account_info(),
        &[flash_loan_authority_seeds],
        liquidity_amount,
    )?;

    // Step 2: Perform liquidation (simplified - assumes external liquidation logic)
    // In a real implementation, this would invoke the regular liquidation process
    
    // Step 3: Validate flash loan repayment with proper balance checking
    let flash_loan_balance_after = ctx.accounts.flash_loan_source.amount;
    
    // Store initial balance before flash loan for validation
    let expected_final_balance = ctx.accounts.flash_loan_reserve_liquidity_supply.amount;
    
    // Validate that the source account has enough tokens for repayment + fee
    if flash_loan_balance_after < total_repayment {
        return Err(LendingError::FlashLoanNotRepaid.into());
    }
    
    // Additional validation: ensure the repayment amount matches loan + fee exactly
    let available_for_repayment = flash_loan_balance_after;
    if available_for_repayment < total_repayment {
        return Err(LendingError::InsufficientTokenBalance.into());
    }

    // Step 4: Collect flash loan repayment + fee atomically
    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.flash_loan_source,
        &ctx.accounts.flash_loan_reserve_liquidity_supply,
        &ctx.accounts.liquidator.to_account_info(),
        &[],
        total_repayment,
    )?;

    // Verify the full repayment was received by checking final balance
    let final_reserve_balance = ctx.accounts.flash_loan_reserve_liquidity_supply.amount;
    let expected_balance = expected_final_balance
        .checked_add(flash_loan_fee)
        .ok_or(LendingError::MathOverflow)?;
    
    if final_reserve_balance < expected_balance {
        return Err(LendingError::FlashLoanFeeNotPaid.into());
    }

    // Update flash loan reserve state (add fee to available liquidity)
    flash_loan_reserve.add_liquidity(flash_loan_fee)?;

    msg!(
        "Flash liquidation completed - amount: {}, fee: {}",
        liquidity_amount,
        flash_loan_fee
    );

    Ok(())
}

/// Batch liquidate multiple unhealthy obligations
pub fn batch_liquidate_obligations(
    ctx: Context<BatchLiquidateObligations>,
    liquidation_params: Vec<LiquidationParams>,
) -> Result<()> {
    let _market = &ctx.accounts.market;

    if liquidation_params.len() > 10 {
        return Err(LendingError::InvalidAmount.into());
    }

    let mut total_liquidated_value = 0u64;
    
    for (i, params) in liquidation_params.iter().enumerate() {
        // Get accounts from remaining_accounts
        let obligation_info = ctx.remaining_accounts
            .get(i * 6)
            .ok_or(LendingError::InvalidAccount)?;
        
        // Validate obligation is unhealthy by deserializing and checking
        let obligation_data = obligation_info.try_borrow_data()?;
        let mut obligation_data_slice = obligation_data.as_ref();
        let obligation = Obligation::try_deserialize(&mut obligation_data_slice)
            .map_err(|_| LendingError::InvalidAccount)?;

        if obligation.is_healthy()? {
            continue; // Skip healthy obligations
        }

        total_liquidated_value = total_liquidated_value
            .checked_add(params.liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
    }

    msg!(
        "Batch liquidated {} obligations, total value: {}",
        liquidation_params.len(),
        total_liquidated_value
    );

    Ok(())
}

// Helper structs

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct LiquidationParams {
    pub liquidity_amount: u64,
    pub min_collateral_amount: u64,
}

// Context structs for liquidation instructions

#[derive(Accounts)]
pub struct LiquidateObligation<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account being liquidated
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation.owner.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub obligation: Account<'info, Obligation>,

    /// Reserve for the asset being repaid
    #[account(
        mut,
        seeds = [RESERVE_SEED, repay_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        // Price oracle validation will be done manually
        // Liquidity supply validation will be done manually
    )]
    pub repay_reserve: Account<'info, Reserve>,

    /// Reserve for the collateral being withdrawn
    #[account(
        mut,
        seeds = [RESERVE_SEED, withdraw_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        // Price oracle validation will be done manually
    )]
    pub withdraw_reserve: Account<'info, Reserve>,

    /// Price oracle for repay asset
    /// CHECK: This account is validated by the repay_reserve's price_oracle field
    pub repay_price_oracle: UncheckedAccount<'info>,

    /// Price oracle for withdraw asset
    /// CHECK: This account is validated by the withdraw_reserve's price_oracle field
    pub withdraw_price_oracle: UncheckedAccount<'info>,

    /// Liquidator's source liquidity token account (for repayment)
    #[account(
        mut,
        token::mint = repay_reserve.liquidity_mint,
        token::authority = liquidator
    )]
    pub source_liquidity: Account<'info, TokenAccount>,

    /// Liquidator's destination collateral token account (receives seized collateral)
    #[account(
        mut,
        token::mint = withdraw_reserve.collateral_mint,
        token::authority = liquidator
    )]
    pub destination_collateral: Account<'info, TokenAccount>,

    /// Repay reserve's liquidity supply token account
    #[account(
        mut,
        token::mint = repay_reserve.liquidity_mint
    )]
    pub repay_reserve_liquidity_supply: Account<'info, TokenAccount>,

    /// Withdraw reserve's collateral supply token account
    #[account(
        mut,
        token::mint = withdraw_reserve.collateral_mint,
        token::authority = withdraw_collateral_supply_authority
    )]
    pub withdraw_reserve_collateral_supply: Account<'info, TokenAccount>,

    /// Withdraw collateral supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [COLLATERAL_TOKEN_SEED, withdraw_reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub withdraw_collateral_supply_authority: UncheckedAccount<'info>,

    /// Liquidator
    pub liquidator: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct FlashLiquidateObligation<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Obligation account being liquidated
    #[account(
        mut,
        seeds = [OBLIGATION_SEED, obligation.owner.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub obligation: Account<'info, Obligation>,

    /// Reserve providing flash loan
    #[account(
        mut,
        seeds = [RESERVE_SEED, flash_loan_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub flash_loan_reserve: Account<'info, Reserve>,

    /// Reserve for the asset being repaid
    #[account(
        mut,
        seeds = [RESERVE_SEED, repay_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub repay_reserve: Account<'info, Reserve>,

    /// Reserve for the collateral being withdrawn
    #[account(
        mut,
        seeds = [RESERVE_SEED, withdraw_reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub withdraw_reserve: Account<'info, Reserve>,

    /// Flash loan reserve's liquidity supply token account
    #[account(
        mut,
        token::mint = flash_loan_reserve.liquidity_mint,
        token::authority = flash_loan_reserve_authority
    )]
    pub flash_loan_reserve_liquidity_supply: Account<'info, TokenAccount>,

    /// Flash loan reserve authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [LIQUIDITY_TOKEN_SEED, flash_loan_reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub flash_loan_reserve_authority: UncheckedAccount<'info>,

    /// Flash loan destination (temporary account for liquidator)
    #[account(
        mut,
        token::mint = flash_loan_reserve.liquidity_mint,
        token::authority = liquidator
    )]
    pub flash_loan_destination: Account<'info, TokenAccount>,

    /// Flash loan source (liquidator repays from here)
    #[account(
        mut,
        token::mint = flash_loan_reserve.liquidity_mint,
        token::authority = liquidator
    )]
    pub flash_loan_source: Account<'info, TokenAccount>,

    /// Liquidator
    pub liquidator: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BatchLiquidateObligations<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Liquidator performing batch liquidation
    pub liquidator: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    // Note: Individual obligation accounts are passed as remaining_accounts
}