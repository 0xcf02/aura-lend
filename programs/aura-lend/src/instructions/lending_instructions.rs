use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer, MintTo, Burn};
use crate::state::*;
use crate::error::LendingError;
use crate::constants::*;
use crate::utils::{TokenUtils, validate_signer};

/// Deposit liquidity into a reserve and receive collateral tokens (aTokens)
pub fn deposit_reserve_liquidity(
    ctx: Context<DepositReserveLiquidity>,
    liquidity_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let reserve = &mut ctx.accounts.reserve;
    let clock = Clock::get()?;

    // Check if market allows deposits
    if market.is_paused() || market.is_lending_disabled() {
        return Err(LendingError::MarketPaused.into());
    }

    // Check if reserve allows deposits
    if reserve.config.flags.contains(ReserveConfigFlags::DEPOSITS_DISABLED) {
        return Err(LendingError::FeatureDisabled.into());
    }

    // Validate minimum deposit amount
    if liquidity_amount < MIN_DEPOSIT_AMOUNT {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Lock reserve to prevent reentrancy
    reserve.lock()?;
    
    // Refresh reserve interest before deposit
    reserve.update_interest(clock.slot)?;

    // Calculate collateral amount to mint
    let collateral_amount = reserve.liquidity_to_collateral(liquidity_amount)?;
    
    if collateral_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Transfer liquidity from user to reserve
    let authority_seeds = &[
        LIQUIDITY_TOKEN_SEED,
        reserve.liquidity_mint.as_ref(),
        b"authority",
        &[ctx.bumps.liquidity_supply_authority],
    ];

    TokenUtils::transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.source_liquidity,
        &ctx.accounts.destination_liquidity,
        &ctx.accounts.user_transfer_authority.to_account_info(),
        &[],
        liquidity_amount,
    )?;

    // Mint collateral tokens to user
    let collateral_mint_authority_seeds = &[
        COLLATERAL_TOKEN_SEED,
        reserve.liquidity_mint.as_ref(),
        b"authority",
        &[ctx.bumps.collateral_mint_authority],
    ];

    TokenUtils::mint_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.collateral_mint,
        &ctx.accounts.destination_collateral,
        &ctx.accounts.collateral_mint_authority.to_account_info(),
        &[collateral_mint_authority_seeds],
        collateral_amount,
    )?;

    // Update reserve state
    reserve.add_liquidity(liquidity_amount)?;
    reserve.state.collateral_mint_supply = reserve.state.collateral_mint_supply
        .checked_add(collateral_amount)
        .ok_or(LendingError::MathOverflow)?;

    // Unlock reserve after successful operation
    reserve.unlock();

    msg!(
        "Deposited {} liquidity, minted {} collateral tokens",
        liquidity_amount,
        collateral_amount
    );

    Ok(())
}

/// Redeem collateral tokens (aTokens) for underlying liquidity
pub fn redeem_reserve_collateral(
    ctx: Context<RedeemReserveCollateral>,
    collateral_amount: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let reserve = &mut ctx.accounts.reserve;
    let clock = Clock::get()?;

    // Check if market allows withdrawals
    if market.is_paused() && !market.is_emergency() {
        return Err(LendingError::MarketPaused.into());
    }

    // Check if reserve allows withdrawals
    if reserve.config.flags.contains(ReserveConfigFlags::WITHDRAWALS_DISABLED) {
        return Err(LendingError::FeatureDisabled.into());
    }

    // Validate collateral amount
    if collateral_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Lock reserve to prevent reentrancy
    reserve.lock()?;
    
    // Refresh reserve interest before withdrawal
    reserve.update_interest(clock.slot)?;

    // Calculate liquidity amount to withdraw
    let liquidity_amount = reserve.collateral_to_liquidity(collateral_amount)?;

    if liquidity_amount == 0 {
        return Err(LendingError::AmountTooSmall.into());
    }

    // Check if reserve has sufficient liquidity
    if reserve.state.available_liquidity < liquidity_amount {
        return Err(LendingError::InsufficientLiquidity.into());
    }

    // Burn collateral tokens from user
    TokenUtils::burn_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.collateral_mint,
        &ctx.accounts.source_collateral,
        &ctx.accounts.user_transfer_authority.to_account_info(),
        &[],
        collateral_amount,
    )?;

    // Transfer liquidity from reserve to user
    let authority_seeds = &[
        LIQUIDITY_TOKEN_SEED,
        reserve.liquidity_mint.as_ref(),
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

    // Update reserve state
    reserve.remove_liquidity(liquidity_amount)?;
    reserve.state.collateral_mint_supply = reserve.state.collateral_mint_supply
        .checked_sub(collateral_amount)
        .ok_or(LendingError::MathUnderflow)?;

    // Unlock reserve after successful operation
    reserve.unlock();

    msg!(
        "Redeemed {} collateral tokens for {} liquidity",
        collateral_amount,
        liquidity_amount
    );

    Ok(())
}

// Context structs for lending instructions

#[derive(Accounts)]
pub struct DepositReserveLiquidity<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Reserve account
    #[account(
        mut,
        seeds = [RESERVE_SEED, reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = liquidity_supply @ LendingError::ReserveLiquidityMintMismatch,
        has_one = collateral_mint @ LendingError::ReserveCollateralMintMismatch
    )]
    pub reserve: Account<'info, Reserve>,

    /// Reserve liquidity supply token account
    #[account(mut)]
    pub destination_liquidity: Account<'info, TokenAccount>,

    /// Liquidity supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [LIQUIDITY_TOKEN_SEED, reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub liquidity_supply_authority: UncheckedAccount<'info>,

    /// Collateral mint (aToken mint)
    #[account(mut)]
    pub collateral_mint: Account<'info, Mint>,

    /// Collateral mint authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [COLLATERAL_TOKEN_SEED, reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub collateral_mint_authority: UncheckedAccount<'info>,

    /// User's source liquidity token account
    #[account(
        mut,
        token::mint = reserve.liquidity_mint,
        token::authority = user_transfer_authority
    )]
    pub source_liquidity: Account<'info, TokenAccount>,

    /// User's destination collateral token account
    #[account(
        mut,
        token::mint = collateral_mint,
        token::authority = user_transfer_authority
    )]
    pub destination_collateral: Account<'info, TokenAccount>,

    /// User's transfer authority
    pub user_transfer_authority: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RedeemReserveCollateral<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Reserve account
    #[account(
        mut,
        seeds = [RESERVE_SEED, reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState,
        has_one = liquidity_supply @ LendingError::ReserveLiquidityMintMismatch,
        has_one = collateral_mint @ LendingError::ReserveCollateralMintMismatch
    )]
    pub reserve: Account<'info, Reserve>,

    /// Reserve liquidity supply token account
    #[account(
        mut,
        token::mint = reserve.liquidity_mint,
        token::authority = liquidity_supply_authority
    )]
    pub source_liquidity: Account<'info, TokenAccount>,

    /// Liquidity supply authority (PDA)
    /// CHECK: This is validated by the seeds constraint
    #[account(
        seeds = [LIQUIDITY_TOKEN_SEED, reserve.liquidity_mint.as_ref(), b"authority"],
        bump
    )]
    pub liquidity_supply_authority: UncheckedAccount<'info>,

    /// Collateral mint (aToken mint)
    #[account(mut)]
    pub collateral_mint: Account<'info, Mint>,

    /// User's source collateral token account
    #[account(
        mut,
        token::mint = collateral_mint,
        token::authority = user_transfer_authority
    )]
    pub source_collateral: Account<'info, TokenAccount>,

    /// User's destination liquidity token account
    #[account(
        mut,
        token::mint = reserve.liquidity_mint,
        token::authority = user_transfer_authority
    )]
    pub destination_liquidity: Account<'info, TokenAccount>,

    /// User's transfer authority
    pub user_transfer_authority: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}