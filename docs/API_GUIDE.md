# Aura Lend Protocol - API Guide

This comprehensive guide covers all instructions, account structures, and integration patterns for the Aura Lend Protocol.

## Table of Contents

- [Overview](#overview)
- [Program Instructions](#program-instructions)
- [Account Structures](#account-structures)
- [Error Codes](#error-codes)
- [Integration Examples](#integration-examples)
- [Best Practices](#best-practices)

## Overview

The Aura Lend Protocol is a decentralized lending platform on Solana featuring:
- Over-collateralized borrowing
- Yield-bearing aTokens
- Enterprise-grade RBAC (Role-Based Access Control)
- Multi-signature governance
- Timelock controls
- Program upgradability

### Program ID
```
Localnet/Devnet: AuRa1Lend1111111111111111111111111111111111
Mainnet: TBD
```

### Key PDAs (Program Derived Addresses)

| Account Type | Seeds | Description |
|--------------|-------|-------------|
| Market | `["market"]` | Global protocol state |
| Reserve | `["reserve", <liquidity_mint>]` | Asset-specific pool |
| Obligation | `["obligation", <owner>]` | User borrowing position |
| MultiSig | `["multisig"]` | Multi-signature wallet |
| Timelock | `["timelock"]` | Timelock controller |
| Governance | `["governance"]` | Role-based access registry |

## Program Instructions

### Market Management

#### `initialize_market`
Initializes the global market state with governance controls.

**Parameters:**
```rust
pub struct InitializeMarketParams {
    pub owner: Pubkey,
    pub emergency_authority: Pubkey,
    pub quote_currency: Pubkey,
    pub aura_token_mint: Pubkey,
}
```

**Accounts:**
```rust
pub struct InitializeMarket<'info> {
    #[account(init, payer = owner, space = Market::SIZE)]
    pub market: Account<'info, Market>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

**Example:**
```typescript
const [marketPubkey] = PublicKey.findProgramAddressSync(
  [Buffer.from("market")],
  program.programId
);

await program.methods
  .initializeMarket({
    owner: wallet.publicKey,
    emergencyAuthority: emergencyAuth.publicKey,
    quoteCurrency: usdcMint,
    auraTokenMint: auraToken,
  })
  .accounts({
    market: marketPubkey,
    owner: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Reserve Management

#### `initialize_reserve`
Creates a new lending reserve for a specific asset.

**Parameters:**
```rust
pub struct InitializeReserveParams {
    pub liquidity_mint: Pubkey,
    pub collateral_mint: Pubkey,
    pub liquidity_supply: Pubkey,
    pub fee_receiver: Pubkey,
    pub price_oracle: Pubkey,
    pub oracle_feed_id: [u8; 32],
    pub config: ReserveConfig,
}
```

**Configuration Options:**
```rust
pub struct ReserveConfig {
    pub optimal_utilization_rate: u64,    // Basis points (8000 = 80%)
    pub loan_to_value_ratio: u64,         // Basis points (7500 = 75%)
    pub liquidation_threshold: u64,       // Basis points (8000 = 80%)
    pub liquidation_bonus: u64,           // Basis points (500 = 5%)
    pub min_borrow_rate: u64,             // Basis points (0 = 0%)
    pub optimal_borrow_rate: u64,         // Basis points (400 = 4%)
    pub max_borrow_rate: u64,             // Basis points (3000 = 30%)
    pub fees: ReserveFees,
    pub deposit_limit: Option<u64>,
    pub borrow_limit: Option<u64>,
    pub fee_receiver: Pubkey,
}
```

#### `update_reserve_config`
Updates reserve parameters (requires appropriate governance permissions).

### Lending Operations

#### `deposit_reserve_liquidity`
Deposits tokens into a reserve to earn yield.

**Parameters:**
- `liquidity_amount: u64` - Amount to deposit (in native token units)

**Returns:** aTokens representing the deposit plus accrued interest

**Example:**
```typescript
const depositAmount = 1000 * 10**6; // 1000 USDC

await program.methods
  .depositReserveLiquidity(new anchor.BN(depositAmount))
  .accounts({
    market: marketPubkey,
    reserve: usdcReserve,
    user: wallet.publicKey,
    userTokenAccount: userUsdcAccount,
    userCollateralAccount: userAusdcAccount,
    reserveLiquiditySupply: reserveSupply,
    collateralMint: collateralMint,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();
```

#### `redeem_reserve_collateral`
Withdraws deposited tokens from a reserve.

**Parameters:**
- `collateral_amount: u64` - Amount of aTokens to redeem

### Borrowing Operations

#### `init_obligation`
Creates a new borrowing position for a user.

#### `deposit_obligation_collateral`
Deposits aTokens as collateral for borrowing.

#### `borrow_obligation_liquidity`
Borrows tokens against collateral.

**Risk Checks:**
- Health factor must remain above 1.0
- Cannot exceed reserve borrow limits
- Must have sufficient collateral value

#### `repay_obligation_liquidity`
Repays borrowed tokens.

### Liquidation Operations

#### `liquidate_obligation`
Liquidates an unhealthy borrowing position.

**Parameters:**
- `liquidity_amount: u64` - Amount to repay

**Conditions:**
- Obligation health factor < 1.0
- Liquidation amount ≤ 50% of debt
- Sufficient collateral to seize

**Example:**
```typescript
await program.methods
  .liquidateObligation(new anchor.BN(repayAmount))
  .accounts({
    market: marketPubkey,
    obligation: obligationPubkey,
    repayReserve: usdcReserve,
    withdrawReserve: solReserve,
    repayPriceOracle: usdcOracle,
    withdrawPriceOracle: solOracle,
    sourceLiquidity: liquidatorUsdcAccount,
    destinationCollateral: liquidatorSolAccount,
    // ... additional accounts
  })
  .rpc();
```

## RBAC (Role-Based Access Control)

### MultiSig Operations

#### `initialize_multisig`
Sets up multi-signature governance.

**Parameters:**
```rust
pub struct InitializeMultisigParams {
    pub signatories: Vec<Pubkey>,  // Max 20 signatories
    pub threshold: u8,             // Required signatures
    pub nonce: u64,               // Replay protection
}
```

#### `create_multisig_proposal`
Creates a proposal requiring multi-signature approval.

#### `sign_multisig_proposal`
Signs a pending proposal.

#### `execute_multisig_proposal`
Executes a proposal after threshold is met.

### Timelock Operations

#### `create_timelock_proposal`
Creates a time-delayed proposal.

**Delay Periods:**
- Critical operations: 7 days
- High priority: 3 days  
- Medium priority: 1 day
- Low priority: 6 hours

### Governance Operations

#### `grant_role`
Grants permissions to a user.

**Available Roles:**
```rust
pub enum RoleType {
    SuperAdmin,           // Complete protocol control
    ReserveManager,       // Asset pool management
    RiskManager,          // Risk parameter control
    OracleManager,        // Price feed management
    EmergencyResponder,   // Crisis management
    FeeManager,           // Economic parameters
    GovernanceManager,    // Role delegation
    TimelockManager,      // Delayed execution control
}
```

**Example:**
```typescript
await program.methods
  .grantRole({
    holder: managerPubkey,
    roleType: "ReserveManager",
    permissions: ["RESERVE_MANAGER"],
    expiresAt: Math.floor(Date.now() / 1000) + (365 * 24 * 60 * 60), // 1 year
  })
  .accounts({
    governance: governancePubkey,
    granter: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

## Account Structures

### Market
```rust
pub struct Market {
    pub version: u8,
    pub owner: Pubkey,                    // Market owner (should be multisig)
    pub emergency_authority: Pubkey,      // Emergency controls
    pub quote_currency: Pubkey,           // USD-denominated token
    pub aura_token_mint: Pubkey,         // Protocol token
    pub aura_mint_authority: Pubkey,     // Protocol token authority
    pub reserves_count: u64,             // Number of reserves
    pub total_fees_collected: u64,       // Protocol revenue
    pub last_update_timestamp: u64,      // Last state update
    pub flags: MarketFlags,              // Market state flags
}
```

### Reserve
```rust
pub struct Reserve {
    pub version: u8,
    pub market: Pubkey,                  // Parent market
    pub liquidity_mint: Pubkey,          // Underlying token
    pub collateral_mint: Pubkey,         // aToken mint
    pub liquidity_supply: Pubkey,        // Token vault
    pub fee_receiver: Pubkey,            // Fee collection account
    pub price_oracle: Pubkey,            // Price feed account
    pub oracle_feed_id: [u8; 32],       // Pyth feed ID
    pub config: ReserveConfig,           // Parameters
    pub state: ReserveState,             // Current state
    pub last_update_timestamp: u64,      // Last update
    pub last_update_slot: u64,           // Last update slot
    pub reentrancy_guard: bool,          // Reentrancy protection
}
```

### Obligation
```rust
pub struct Obligation {
    pub version: u8,
    pub market: Pubkey,                  // Parent market
    pub owner: Pubkey,                   // Borrower
    pub deposits: Vec<ObligationCollateral>,  // Collateral deposits
    pub borrows: Vec<ObligationLiquidity>,    // Outstanding loans
    pub deposited_value_usd: Decimal,    // Total collateral value
    pub borrowed_value_usd: Decimal,     // Total debt value
    pub last_update_timestamp: u64,      // Last update
    pub last_update_slot: u64,           // Last update slot
}
```

## Error Codes

### Common Errors
```rust
pub enum LendingError {
    // Math errors
    MathOverflow = 6000,
    MathUnderflow = 6001,
    DivisionByZero = 6002,
    
    // Market errors
    MarketPaused = 6003,
    MarketOwnerMismatch = 6004,
    InvalidMarketState = 6005,
    
    // Reserve errors
    ReserveNotInitialized = 6006,
    InsufficientLiquidity = 6007,
    InsufficientCollateral = 6008,
    InvalidReserveConfig = 6009,
    ReserveStale = 6010,
    
    // Obligation errors
    ObligationUnhealthy = 6011,
    ObligationCollateralEmpty = 6012,
    ObligationHealthy = 6013,
    LiquidationTooLarge = 6014,
    
    // Oracle errors
    OraclePriceStale = 6015,
    OraclePriceInvalid = 6016,
    OracleAccountMismatch = 6017,
    
    // Authority errors
    InsufficientAuthority = 6018,
    InvalidAuthority = 6019,
    AuthoritySignerMissing = 6020,
    
    // RBAC errors
    InvalidMultisigThreshold = 6021,
    MultisigThresholdNotMet = 6022,
    ProposalExpired = 6023,
    TimelockNotReady = 6024,
    InsufficientPermissions = 6025,
    RoleExpired = 6026,
}
```

## Integration Examples

### Deposit Flow
```typescript
async function depositToReserve(
  program: Program,
  reserve: PublicKey,
  amount: number,
  userWallet: Keypair
) {
  // 1. Get reserve state
  const reserveAccount = await program.account.reserve.fetch(reserve);
  
  // 2. Calculate aToken amount
  const exchangeRate = calculateExchangeRate(reserveAccount);
  const aTokenAmount = amount * exchangeRate;
  
  // 3. Execute deposit
  const tx = await program.methods
    .depositReserveLiquidity(new anchor.BN(amount))
    .accounts({
      // ... accounts
    })
    .rpc();
    
  return { transaction: tx, aTokenAmount };
}
```

### Borrow Flow
```typescript
async function borrowFromReserve(
  program: Program,
  obligation: PublicKey,
  reserve: PublicKey,
  amount: number,
  userWallet: Keypair
) {
  // 1. Refresh obligation and reserve
  await refreshObligation(program, obligation);
  await refreshReserve(program, reserve);
  
  // 2. Check health factor
  const obligationAccount = await program.account.obligation.fetch(obligation);
  const healthFactor = calculateHealthFactor(obligationAccount);
  
  if (healthFactor < 1.2) {
    throw new Error("Insufficient collateral for borrow");
  }
  
  // 3. Execute borrow
  const tx = await program.methods
    .borrowObligationLiquidity(new anchor.BN(amount))
    .accounts({
      // ... accounts
    })
    .rpc();
    
  return tx;
}
```

### Liquidation Flow
```typescript
async function liquidatePosition(
  program: Program,
  obligation: PublicKey,
  repayReserve: PublicKey,
  withdrawReserve: PublicKey,
  repayAmount: number,
  liquidatorWallet: Keypair
) {
  // 1. Check if liquidation is profitable
  const { collateralAmount, bonus } = await calculateLiquidation(
    obligation, repayReserve, withdrawReserve, repayAmount
  );
  
  // 2. Execute liquidation
  const tx = await program.methods
    .liquidateObligation(new anchor.BN(repayAmount))
    .accounts({
      // ... accounts
    })
    .rpc();
    
  return { transaction: tx, collateralSeized: collateralAmount, bonus };
}
```

## Best Practices

### Security
1. **Always refresh accounts** before calculations
2. **Check health factors** before borrow operations
3. **Validate oracle prices** for staleness
4. **Use proper error handling** for all operations
5. **Implement slippage protection** for liquidations

### Performance
1. **Batch operations** when possible
2. **Cache account data** to reduce RPC calls
3. **Use pagination** for large datasets
4. **Monitor compute unit usage**
5. **Optimize transaction size**

### Error Handling
```typescript
try {
  await program.methods.borrowObligationLiquidity(amount).rpc();
} catch (error) {
  if (error.code === 6011) { // ObligationUnhealthy
    throw new Error("Insufficient collateral ratio");
  } else if (error.code === 6007) { // InsufficientLiquidity
    throw new Error("Reserve has insufficient liquidity");
  } else {
    throw error;
  }
}
```

### Governance Integration
```typescript
// For critical operations, use governance flow
async function updateReserveConfig(config: ReserveConfig) {
  // 1. Create multisig proposal
  const proposalTx = await createMultisigProposal({
    operationType: "UpdateReserveConfig",
    data: config,
  });
  
  // 2. Collect signatures
  await collectSignatures(proposalTx);
  
  // 3. Execute after timelock delay
  await executeAfterDelay(proposalTx);
}
```

## Rate Limiting and Quotas

### Transaction Limits
- Maximum 100 operations per batch
- Maximum 16 assets per obligation
- Maximum 128 reserves per market

### Oracle Requirements
- Price updates must be within 2 minutes (240 slots)
- Confidence interval must be < 1%
- Minimum 3 oracle sources for critical assets

### Governance Constraints
- MultiSig: 2-20 signatories, configurable threshold
- Timelock: 6 hours to 7 days depending on operation
- Roles: Maximum 200 concurrent roles per registry

## Support and Resources

- **Documentation**: [https://docs.aura-lend.com](https://docs.aura-lend.com)
- **SDK**: [https://github.com/aura-lend/sdk](https://github.com/aura-lend/sdk)
- **Discord**: [https://discord.gg/aura-lend](https://discord.gg/aura-lend)
- **GitHub**: [https://github.com/aura-lend/protocol](https://github.com/aura-lend/protocol)

---

*This guide covers the current version of the Aura Lend Protocol. For the latest updates and changes, please refer to the official documentation and changelog.*