use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod error;
pub mod utils;
pub mod constants;

use instructions::*;

declare_id!("AuRa1Lend1111111111111111111111111111111111");

#[program]
pub mod aura_lend {
    use super::*;

    // Market management
    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        params: InitializeMarketParams,
    ) -> Result<()> {
        instructions::initialize_market(ctx, params)
    }

    // Reserve management
    pub fn initialize_reserve(
        ctx: Context<InitializeReserve>,
        params: InitializeReserveParams,
    ) -> Result<()> {
        instructions::initialize_reserve(ctx, params)
    }

    pub fn update_reserve_config(
        ctx: Context<UpdateReserveConfig>,
        params: UpdateReserveConfigParams,
    ) -> Result<()> {
        instructions::update_reserve_config(ctx, params)
    }

    // Lending operations
    pub fn deposit_reserve_liquidity(
        ctx: Context<DepositReserveLiquidity>,
        liquidity_amount: u64,
    ) -> Result<()> {
        instructions::deposit_reserve_liquidity(ctx, liquidity_amount)
    }

    pub fn redeem_reserve_collateral(
        ctx: Context<RedeemReserveCollateral>,
        collateral_amount: u64,
    ) -> Result<()> {
        instructions::redeem_reserve_collateral(ctx, collateral_amount)
    }

    // Borrowing operations
    pub fn init_obligation(ctx: Context<InitObligation>) -> Result<()> {
        instructions::init_obligation(ctx)
    }

    pub fn deposit_obligation_collateral(
        ctx: Context<DepositObligationCollateral>,
        collateral_amount: u64,
    ) -> Result<()> {
        instructions::deposit_obligation_collateral(ctx, collateral_amount)
    }

    pub fn withdraw_obligation_collateral(
        ctx: Context<WithdrawObligationCollateral>,
        collateral_amount: u64,
    ) -> Result<()> {
        instructions::withdraw_obligation_collateral(ctx, collateral_amount)
    }

    pub fn borrow_obligation_liquidity(
        ctx: Context<BorrowObligationLiquidity>,
        liquidity_amount: u64,
    ) -> Result<()> {
        instructions::borrow_obligation_liquidity(ctx, liquidity_amount)
    }

    pub fn repay_obligation_liquidity(
        ctx: Context<RepayObligationLiquidity>,
        liquidity_amount: u64,
    ) -> Result<()> {
        instructions::repay_obligation_liquidity(ctx, liquidity_amount)
    }

    // Liquidation
    pub fn liquidate_obligation(
        ctx: Context<LiquidateObligation>,
        liquidity_amount: u64,
    ) -> Result<()> {
        instructions::liquidate_obligation(ctx, liquidity_amount)
    }

    // Oracle operations
    pub fn refresh_reserve(ctx: Context<RefreshReserve>) -> Result<()> {
        instructions::refresh_reserve(ctx)
    }

    pub fn refresh_obligation(ctx: Context<RefreshObligation>) -> Result<()> {
        instructions::refresh_obligation(ctx)
    }
}