use crate::constants::*;
use crate::error::LendingError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use solana_program::program_option::COption;

/// Initialize the lending market
pub fn initialize_market(
    ctx: Context<InitializeMarket>,
    params: InitializeMarketParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let aura_mint_authority = &ctx.accounts.aura_mint_authority;

    // Validate that the market authority can mint AURA tokens
    if ctx.accounts.aura_token_mint.mint_authority != COption::Some(aura_mint_authority.key()) {
        return Err(LendingError::InvalidAuthority.into());
    }

    // Initialize the market
    **market = Market::new(
        params.multisig_owner,
        params.emergency_authority,
        params.governance,
        params.timelock_controller,
        params.quote_currency,
        params.aura_token_mint,
        aura_mint_authority.key(),
    )?;

    msg!("Market initialized successfully");
    Ok(())
}

/// Initialize a new reserve for an asset
pub fn initialize_reserve(
    ctx: Context<InitializeReserve>,
    params: InitializeReserveParams,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let reserve = &mut ctx.accounts.reserve;

    // Validate reserve configuration
    validate_reserve_config(&params.config)?;

    // Validate oracle feed ID is not empty
    if params.oracle_feed_id == [0u8; 32] {
        return Err(LendingError::OracleAccountMismatch.into());
    }

    // Increment market reserves count
    market.increment_reserves_count()?;
    market.update_timestamp()?;

    // Initialize the reserve with proper oracle feed ID from parameters
    **reserve = Reserve::new(
        market.key(),
        params.liquidity_mint,
        ctx.accounts.collateral_mint.key(),
        ctx.accounts.liquidity_supply.key(),
        ctx.accounts.fee_receiver.key(),
        params.price_oracle,
        params.oracle_feed_id, // Use oracle feed ID from parameters
        params.config,
    )?;

    msg!(
        "Reserve initialized successfully for mint: {}",
        params.liquidity_mint
    );
    Ok(())
}

/// Update reserve configuration (owner only)
pub fn update_reserve_config(
    ctx: Context<UpdateReserveConfig>,
    params: UpdateReserveConfigParams,
) -> Result<()> {
    let reserve = &mut ctx.accounts.reserve;

    // Validate new configuration
    validate_reserve_config(&params.config)?;

    // Update configuration
    reserve.config = params.config;
    reserve.last_update_timestamp = Clock::get()?.unix_timestamp as u64;

    msg!("Reserve configuration updated successfully");
    Ok(())
}

/// Validate reserve configuration parameters
fn validate_reserve_config(config: &ReserveConfig) -> Result<()> {
    // Validate loan-to-value ratio
    if config.loan_to_value_ratio_bps > MAX_LOAN_TO_VALUE_RATIO_BPS {
        return Err(LendingError::InvalidReserveConfig.into());
    }

    // Validate liquidation threshold is higher than LTV
    if config.liquidation_threshold_bps <= config.loan_to_value_ratio_bps {
        return Err(LendingError::InvalidReserveConfig.into());
    }

    // Validate liquidation penalty
    if config.liquidation_penalty_bps > MAX_LIQUIDATION_BONUS_BPS {
        return Err(LendingError::InvalidReserveConfig.into());
    }

    // Validate interest rate parameters
    if config.optimal_utilization_rate_bps > BASIS_POINTS_PRECISION {
        return Err(LendingError::InvalidReserveConfig.into());
    }

    if config.max_borrow_rate_bps < config.base_borrow_rate_bps {
        return Err(LendingError::InvalidReserveConfig.into());
    }

    // Validate protocol fee
    if config.protocol_fee_bps > BASIS_POINTS_PRECISION / 2 {
        // Max 50% protocol fee
        return Err(LendingError::InvalidReserveConfig.into());
    }

    Ok(())
}

// Context structs for each instruction

#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    /// Market account to initialize
    #[account(
        init,
        payer = payer,
        space = Market::SIZE,
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Quote currency mint (e.g., USDC)
    pub quote_currency_mint: Account<'info, Mint>,

    /// AURA governance token mint
    pub aura_token_mint: Account<'info, Mint>,

    /// Authority for minting AURA tokens (PDA)
    /// CHECK: This account will be validated in the instruction
    pub aura_mint_authority: UncheckedAccount<'info>,

    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(params: InitializeReserveParams)]
pub struct InitializeReserve<'info> {
    /// Market account
    #[account(
        mut,
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Reserve account to initialize
    #[account(
        init,
        payer = payer,
        space = Reserve::SIZE,
        seeds = [RESERVE_SEED, liquidity_mint.key().as_ref()],
        bump
    )]
    pub reserve: Account<'info, Reserve>,

    /// Liquidity token mint (e.g., USDC, SOL)
    pub liquidity_mint: Account<'info, Mint>,

    /// Collateral token mint (aToken)
    #[account(
        init,
        payer = payer,
        mint::decimals = liquidity_mint.decimals,
        mint::authority = collateral_mint_authority,
        seeds = [COLLATERAL_TOKEN_SEED, liquidity_mint.key().as_ref()],
        bump
    )]
    pub collateral_mint: Account<'info, Mint>,

    /// Authority for collateral mint (PDA)
    /// CHECK: This is a PDA derived from seeds
    #[account(seeds = [COLLATERAL_TOKEN_SEED, liquidity_mint.key().as_ref(), b"authority"], bump)]
    pub collateral_mint_authority: UncheckedAccount<'info>,

    /// Liquidity supply token account
    #[account(
        init,
        payer = payer,
        token::mint = liquidity_mint,
        token::authority = liquidity_supply_authority,
        seeds = [LIQUIDITY_TOKEN_SEED, liquidity_mint.key().as_ref()],
        bump
    )]
    pub liquidity_supply: Account<'info, anchor_spl::token::TokenAccount>,

    /// Authority for liquidity supply (PDA)
    /// CHECK: This is a PDA derived from seeds
    #[account(seeds = [LIQUIDITY_TOKEN_SEED, liquidity_mint.key().as_ref(), b"authority"], bump)]
    pub liquidity_supply_authority: UncheckedAccount<'info>,

    /// Fee receiver token account
    #[account(
        init,
        payer = payer,
        token::mint = liquidity_mint,
        token::authority = owner,
    )]
    pub fee_receiver: Account<'info, anchor_spl::token::TokenAccount>,

    /// Market owner (must sign for reserve creation)
    pub owner: Signer<'info>,

    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateReserveConfig<'info> {
    /// Market account
    #[account(
        seeds = [MARKET_SEED],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Reserve account to update
    #[account(
        mut,
        seeds = [RESERVE_SEED, reserve.liquidity_mint.as_ref()],
        bump,
        has_one = market @ LendingError::InvalidMarketState
    )]
    pub reserve: Account<'info, Reserve>,

    /// Market owner (must sign for configuration changes)
    pub owner: Signer<'info>,
}
