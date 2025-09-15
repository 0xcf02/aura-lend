use anchor_lang::prelude::*;

// Module declarations in alphabetical order
pub mod constants;
pub mod error;
pub mod instructions;
pub mod migration;
pub mod state;
pub mod utils;

use instructions::*;
use state::governance::{GrantRoleParams, InitializeGovernanceParams};
use state::market::InitializeMarketParams;
use state::multisig::{CreateProposalParams, InitializeMultisigParams};
use state::reserve::{InitializeReserveParams, UpdateReserveConfigParams};
use state::timelock::CreateTimelockProposalParams;
use state::timelock::TimelockDelay;

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

    // RBAC Management - MultiSig operations
    pub fn initialize_multisig(
        ctx: Context<InitializeMultisig>,
        params: InitializeMultisigParams,
    ) -> Result<()> {
        instructions::initialize_multisig(ctx, params)
    }

    pub fn create_multisig_proposal(
        ctx: Context<CreateMultisigProposal>,
        params: CreateProposalParams,
    ) -> Result<()> {
        instructions::create_multisig_proposal(ctx, params)
    }

    pub fn sign_multisig_proposal(ctx: Context<SignMultisigProposal>) -> Result<()> {
        instructions::sign_multisig_proposal(ctx)
    }

    pub fn execute_multisig_proposal(ctx: Context<ExecuteMultisigProposal>) -> Result<()> {
        instructions::execute_multisig_proposal(ctx)
    }

    pub fn cancel_multisig_proposal(ctx: Context<CancelMultisigProposal>) -> Result<()> {
        instructions::cancel_multisig_proposal(ctx)
    }

    pub fn update_multisig_config(
        ctx: Context<UpdateMultisigConfig>,
        params: InitializeMultisigParams,
    ) -> Result<()> {
        instructions::update_multisig_config(ctx, params)
    }

    // Timelock operations
    pub fn initialize_timelock(ctx: Context<InitializeTimelock>) -> Result<()> {
        instructions::initialize_timelock(ctx)
    }

    pub fn create_timelock_proposal(
        ctx: Context<CreateTimelockProposal>,
        params: CreateTimelockProposalParams,
    ) -> Result<()> {
        instructions::create_timelock_proposal(ctx, params)
    }

    pub fn execute_timelock_proposal(ctx: Context<ExecuteTimelockProposal>) -> Result<()> {
        instructions::execute_timelock_proposal(ctx)
    }

    pub fn cancel_timelock_proposal(ctx: Context<CancelTimelockProposal>) -> Result<()> {
        instructions::cancel_timelock_proposal(ctx)
    }

    pub fn update_timelock_delays(
        ctx: Context<UpdateTimelockDelays>,
        new_delays: Vec<TimelockDelay>,
    ) -> Result<()> {
        instructions::update_timelock_delays(ctx, new_delays)
    }

    pub fn cleanup_expired_proposals(ctx: Context<CleanupExpiredProposals>) -> Result<()> {
        instructions::cleanup_expired_proposals(ctx)
    }

    // Governance operations
    pub fn initialize_governance(
        ctx: Context<InitializeGovernance>,
        params: InitializeGovernanceParams,
    ) -> Result<()> {
        instructions::initialize_governance(ctx, params)
    }

    pub fn grant_role(ctx: Context<GrantRole>, params: GrantRoleParams) -> Result<()> {
        instructions::grant_role(ctx, params)
    }

    pub fn revoke_role(ctx: Context<RevokeRole>, target_holder: Pubkey) -> Result<()> {
        instructions::revoke_role(ctx, target_holder)
    }

    pub fn delegate_permissions(
        ctx: Context<DelegatePermissions>,
        params: DelegatePermissionsParams,
    ) -> Result<()> {
        instructions::delegate_permissions(ctx, params)
    }

    pub fn cleanup_expired_roles(ctx: Context<CleanupExpiredRoles>) -> Result<()> {
        instructions::cleanup_expired_roles(ctx)
    }

    pub fn update_governance_config(
        ctx: Context<UpdateGovernanceConfig>,
        new_available_permissions: u64,
    ) -> Result<()> {
        instructions::update_governance_config(ctx, new_available_permissions)
    }

    pub fn emergency_grant_role(
        ctx: Context<EmergencyGrantRole>,
        params: GrantRoleParams,
    ) -> Result<()> {
        instructions::emergency_grant_role(ctx, params)
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

    // Program upgrade operations
    pub fn set_upgrade_authority(ctx: Context<SetUpgradeAuthority>) -> Result<()> {
        instructions::set_upgrade_authority(ctx)
    }

    pub fn upgrade_program(ctx: Context<UpgradeProgram>) -> Result<()> {
        instructions::upgrade_program(ctx)
    }

    pub fn freeze_program(ctx: Context<FreezeProgram>) -> Result<()> {
        instructions::freeze_program(ctx)
    }

    // Data migration operations
    pub fn migrate_market(ctx: Context<MigrateMarket>) -> Result<()> {
        instructions::migrate_market(ctx)
    }

    pub fn migrate_reserve(ctx: Context<MigrateReserve>) -> Result<()> {
        instructions::migrate_reserve(ctx)
    }

    pub fn migrate_obligation(ctx: Context<MigrateObligation>) -> Result<()> {
        instructions::migrate_obligation(ctx)
    }

    pub fn migrate_multisig(ctx: Context<MigrateMultisig>) -> Result<()> {
        instructions::migrate_multisig(ctx)
    }

    pub fn migrate_timelock(ctx: Context<MigrateTimelock>) -> Result<()> {
        instructions::migrate_timelock(ctx)
    }

    pub fn migrate_governance(ctx: Context<MigrateGovernance>) -> Result<()> {
        instructions::migrate_governance(ctx)
    }

    pub fn batch_migrate_reserves<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchMigrateReserves<'info>>,
    ) -> Result<()> {
        instructions::batch_migrate_reserves(ctx)
    }

    // Configuration management
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        params: utils::config::ConfigUpdateParams,
    ) -> Result<()> {
        instructions::initialize_config(ctx, params)
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        params: utils::config::ConfigUpdateParams,
        timelock_priority: utils::config::TimelockPriority,
    ) -> Result<()> {
        instructions::update_config(ctx, params, timelock_priority)
    }

    pub fn emergency_config_update(
        ctx: Context<EmergencyConfigUpdate>,
        emergency_params: instructions::config_instructions::EmergencyConfigParams,
    ) -> Result<()> {
        instructions::emergency_config_update(ctx, emergency_params)
    }

    pub fn get_config(ctx: Context<GetConfig>) -> Result<utils::config::ProtocolConfig> {
        instructions::get_config(ctx)
    }
}
