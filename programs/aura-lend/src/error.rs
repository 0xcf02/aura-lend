use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    // Math errors
    #[msg("Math operation overflow")]
    MathOverflow,
    #[msg("Math operation underflow")]
    MathUnderflow,
    #[msg("Division by zero")]
    DivisionByZero,

    // Market errors
    #[msg("Market is paused")]
    MarketPaused,
    #[msg("Market owner mismatch")]
    MarketOwnerMismatch,
    #[msg("Market authority mismatch")]
    MarketAuthorityMismatch,
    #[msg("Invalid market state")]
    InvalidMarketState,

    // Reserve errors
    #[msg("Reserve is not initialized")]
    ReserveNotInitialized,
    #[msg("Reserve liquidity is insufficient")]
    InsufficientLiquidity,
    #[msg("Reserve collateral is insufficient")]
    InsufficientCollateral,
    #[msg("Invalid reserve configuration")]
    InvalidReserveConfig,
    #[msg("Reserve is stale and must be refreshed")]
    ReserveStale,
    #[msg("Invalid reserve state")]
    InvalidReserveState,
    #[msg("Reserve liquidity mint mismatch")]
    ReserveLiquidityMintMismatch,
    #[msg("Reserve collateral mint mismatch")]
    ReserveCollateralMintMismatch,

    // Obligation errors
    #[msg("Obligation is not healthy")]
    ObligationUnhealthy,
    #[msg("Obligation collateral is empty")]
    ObligationCollateralEmpty,
    #[msg("Obligation liquidity is empty")]
    ObligationLiquidityEmpty,
    #[msg("Obligation deposits are full")]
    ObligationDepositsMaxed,
    #[msg("Obligation borrows are full")]
    ObligationBorrowsMaxed,
    #[msg("Obligation reserve not found")]
    ObligationReserveNotFound,
    #[msg("Obligation is stale and must be refreshed")]
    ObligationStale,
    #[msg("Cannot liquidate healthy obligation")]
    ObligationHealthy,
    #[msg("Liquidation amount too large")]
    LiquidationTooLarge,
    
    // Oracle errors
    #[msg("Oracle price is stale")]
    OraclePriceStale,
    #[msg("Oracle price is invalid")]
    OraclePriceInvalid,
    #[msg("Oracle account mismatch")]
    OracleAccountMismatch,
    #[msg("Oracle confidence too wide")]
    OracleConfidenceTooWide,

    // Token errors
    #[msg("Insufficient token balance")]
    InsufficientTokenBalance,
    #[msg("Token account owner mismatch")]
    TokenAccountOwnerMismatch,
    #[msg("Token mint mismatch")]
    TokenMintMismatch,
    #[msg("Invalid token program")]
    InvalidTokenProgram,

    // Authority errors
    #[msg("Insufficient authority")]
    InsufficientAuthority,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("Authority signer missing")]
    AuthoritySignerMissing,

    // Amount errors
    #[msg("Amount is too small")]
    AmountTooSmall,
    #[msg("Amount is too large")]
    AmountTooLarge,
    #[msg("Invalid amount")]
    InvalidAmount,

    // Rate errors
    #[msg("Utilization rate exceeds maximum")]
    UtilizationRateExceedsMax,
    #[msg("Interest rate is invalid")]
    InvalidInterestRate,
    #[msg("Loan to value ratio exceeds maximum")]
    LoanToValueRatioExceedsMax,

    // Flash loan errors
    #[msg("Flash loan not repaid")]
    FlashLoanNotRepaid,
    #[msg("Flash loan fee not paid")]
    FlashLoanFeeNotPaid,
    #[msg("Flash loan amount too large")]
    FlashLoanAmountTooLarge,

    // General validation errors
    #[msg("Invalid instruction")]
    InvalidInstruction,
    #[msg("Invalid account")]
    InvalidAccount,
    #[msg("Account already initialized")]
    AccountAlreadyInitialized,
    #[msg("Account not initialized")]
    AccountNotInitialized,
    #[msg("Invalid account owner")]
    InvalidAccountOwner,
    #[msg("Invalid account size")]
    InvalidAccountSize,

    // Time errors
    #[msg("Operation expired")]
    OperationExpired,
    #[msg("Operation too early")]
    OperationTooEarly,

    // Protocol state errors
    #[msg("Protocol is in emergency mode")]
    ProtocolEmergencyMode,
    #[msg("Feature is disabled")]
    FeatureDisabled,
    #[msg("Operation not permitted")]
    OperationNotPermitted,
    
    // Reentrancy protection errors
    #[msg("Operation already in progress - reentrancy detected")]
    OperationInProgress,
    #[msg("Invalid unlock operation - not currently locked")]
    InvalidUnlockOperation,
    #[msg("Reentrant call detected")]
    ReentrantCall,

    // MultiSig errors
    #[msg("Invalid multisig threshold")]
    InvalidMultisigThreshold,
    #[msg("Invalid signatory count")]
    InvalidSignatoryCount,
    #[msg("Duplicate signatory found")]
    DuplicateSignatory,
    #[msg("Invalid signatory")]
    InvalidSignatory,
    #[msg("Multisig threshold not met")]
    MultisigThresholdNotMet,
    #[msg("Already signed this proposal")]
    AlreadySigned,
    #[msg("Invalid nonce")]
    InvalidNonce,
    #[msg("Proposal not active")]
    ProposalNotActive,
    #[msg("Proposal expired")]
    ProposalExpired,
    #[msg("Proposal not executed")]
    ProposalNotExecuted,
    #[msg("Invalid operation type")]
    InvalidOperationType,
    #[msg("Unauthorized cancellation")]
    UnauthorizedCancellation,
    #[msg("Instruction too large")]
    InstructionTooLarge,

    // Timelock errors
    #[msg("Timelock not ready for execution")]
    TimelockNotReady,
    #[msg("Too many active proposals")]
    TooManyActiveProposals,
    #[msg("Proposal already active")]
    ProposalAlreadyActive,
    #[msg("Proposal not found")]
    ProposalNotFound,
    #[msg("Proposal not pending")]
    ProposalNotPending,
    #[msg("Delay too short for operation type")]
    DelayTooShort,
    #[msg("Too many target accounts")]
    TooManyTargetAccounts,

    // Governance/Role errors
    #[msg("Too many roles")]
    TooManyRoles,
    #[msg("Account already has active role")]
    AccountAlreadyHasRole,
    #[msg("Invalid permissions")]
    InvalidPermissions,
    #[msg("Insufficient permissions")]
    InsufficientPermissions,
    #[msg("Role not found")]
    RoleNotFound,
    #[msg("Role expired")]
    RoleExpired,
    #[msg("Cannot delegate permissions not held")]
    CannotDelegatePermissionsNotHeld,
    #[msg("Emergency role must have expiration")]
    EmergencyRoleMustHaveExpiration,
    #[msg("Emergency role duration too long")]
    EmergencyRoleTooLong,
    #[msg("Invalid emergency permissions")]
    InvalidEmergencyPermissions,

    // Migration/Upgrade errors
    #[msg("Unsupported migration version")]
    UnsupportedMigration,
    #[msg("Invalid migration - cannot downgrade")]
    InvalidMigration,
    #[msg("Partial migration failure")]
    PartialMigrationFailure,
    #[msg("Migration already completed")]
    MigrationAlreadyCompleted,
    #[msg("Migration in progress")]
    MigrationInProgress,

    // Configuration errors
    #[msg("Invalid configuration parameter")]
    InvalidConfiguration,
    #[msg("Configuration parameter out of range")]
    ConfigurationOutOfRange,
    #[msg("Configuration validation failed")]
    ConfigurationValidationFailed,
    #[msg("Configuration requires higher permissions")]
    ConfigurationInsufficientPermissions,
}