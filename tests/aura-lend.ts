import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AuraLend } from "../target/types/aura_lend";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo } from "@solana/spl-token";
import { assert, expect } from "chai";

describe("aura-lend", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AuraLend as Program<AuraLend>;
  
  // Test accounts
  let marketPubkey: PublicKey;
  let usdcMint: PublicKey;
  let solReserve: PublicKey;
  let usdcReserve: PublicKey;
  let userKeypair: Keypair;
  let obligationPubkey: PublicKey;
  
  // Test constants
  const USDC_DECIMALS = 6;
  const SOL_DECIMALS = 9;
  const INITIAL_USDC_AMOUNT = 1000 * 10 ** USDC_DECIMALS; // 1000 USDC
  const DEPOSIT_AMOUNT = 100 * 10 ** USDC_DECIMALS; // 100 USDC
  const BORROW_AMOUNT = 1 * 10 ** SOL_DECIMALS; // 1 SOL

  before(async () => {
    // Create test user
    userKeypair = Keypair.generate();
    
    // Airdrop SOL to test accounts
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        provider.wallet.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      )
    );
    
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        userKeypair.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      )
    );

    // Create test USDC mint
    usdcMint = await createMint(
      provider.connection,
      provider.wallet.payer,
      provider.wallet.publicKey,
      null,
      USDC_DECIMALS
    );

    // Derive PDA addresses
    [marketPubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("market")],
      program.programId
    );

    [solReserve] = PublicKey.findProgramAddressSync(
      [Buffer.from("reserve"), PublicKey.default.toBuffer()], // Using default pubkey as SOL mint
      program.programId
    );

    [usdcReserve] = PublicKey.findProgramAddressSync(
      [Buffer.from("reserve"), usdcMint.toBuffer()],
      program.programId
    );

    [obligationPubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("obligation"), userKeypair.publicKey.toBuffer()],
      program.programId
    );
  });

  it("Initializes the market", async () => {
    const [auraTokenMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("aura_mint")],
      program.programId
    );

    const [auraMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("aura_mint_authority")],
      program.programId
    );

    const params = {
      owner: provider.wallet.publicKey,
      emergencyAuthority: provider.wallet.publicKey,
      quoteCurrency: usdcMint,
      auraTokenMint: auraTokenMint,
    };

    await program.methods
      .initializeMarket(params)
      .accounts({
        market: marketPubkey,
        quoteCurrencyMint: usdcMint,
        auraTokenMint: auraTokenMint,
        auraMintAuthority: auraMintAuthority,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Verify market was created
    const marketAccount = await program.account.market.fetch(marketPubkey);
    assert.equal(marketAccount.owner.toString(), provider.wallet.publicKey.toString());
    assert.equal(marketAccount.quoteCurrency.toString(), usdcMint.toString());
    assert.equal(marketAccount.reservesCount.toString(), "0");
  });

  it("Initializes a USDC reserve", async () => {
    const [collateralMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("collateral"), usdcMint.toBuffer()],
      program.programId
    );

    const [collateralMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("collateral"), usdcMint.toBuffer(), Buffer.from("authority")],
      program.programId
    );

    const [liquiditySupply] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity"), usdcMint.toBuffer()],
      program.programId
    );

    const [liquiditySupplyAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity"), usdcMint.toBuffer(), Buffer.from("authority")],
      program.programId
    );

    // Create fee receiver token account
    const feeReceiver = await createAccount(
      provider.connection,
      provider.wallet.payer,
      usdcMint,
      provider.wallet.publicKey
    );

    const reserveConfig = {
      loanToValueRatioBps: 7500, // 75%
      liquidationThresholdBps: 8000, // 80%
      liquidationPenaltyBps: 500, // 5%
      baseBorrowRateBps: 0,
      borrowRateMultiplierBps: 500,
      jumpRateMultiplierBps: 10000,
      optimalUtilizationRateBps: 8000, // 80%
      protocolFeeBps: 1000, // 10%
      maxBorrowRateBps: 50000, // 500%
      decimals: USDC_DECIMALS,
      flags: {
        depositsDisabled: false,
        withdrawalsDisabled: false,
        borrowingDisabled: false,
        repaymentsDisabled: false,
        liquidationsDisabled: false,
        collateralEnabled: true,
      },
    };

    const params = {
      liquidityMint: usdcMint,
      priceOracle: PublicKey.default, // Mock oracle for testing
      config: reserveConfig,
    };

    await program.methods
      .initializeReserve(params)
      .accounts({
        market: marketPubkey,
        reserve: usdcReserve,
        liquidityMint: usdcMint,
        collateralMint: collateralMint,
        collateralMintAuthority: collateralMintAuthority,
        liquiditySupply: liquiditySupply,
        liquiditySupplyAuthority: liquiditySupplyAuthority,
        feeReceiver: feeReceiver,
        owner: provider.wallet.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    // Verify reserve was created
    const reserveAccount = await program.account.reserve.fetch(usdcReserve);
    assert.equal(reserveAccount.liquidityMint.toString(), usdcMint.toString());
    assert.equal(reserveAccount.config.decimals, USDC_DECIMALS);
    assert.equal(reserveAccount.config.loanToValueRatioBps, 7500);
  });

  it("Deposits USDC liquidity", async () => {
    // Create user USDC token account and mint tokens
    const userUsdcAccount = await createAccount(
      provider.connection,
      userKeypair,
      usdcMint,
      userKeypair.publicKey
    );

    await mintTo(
      provider.connection,
      provider.wallet.payer,
      usdcMint,
      userUsdcAccount,
      provider.wallet.payer,
      INITIAL_USDC_AMOUNT
    );

    // Create user collateral token account (aUSDC)
    const [collateralMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("collateral"), usdcMint.toBuffer()],
      program.programId
    );

    const userCollateralAccount = await createAccount(
      provider.connection,
      userKeypair,
      collateralMint,
      userKeypair.publicKey
    );

    // Get reserve liquidity supply account
    const [liquiditySupply] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity"), usdcMint.toBuffer()],
      program.programId
    );

    const [liquiditySupplyAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity"), usdcMint.toBuffer(), Buffer.from("authority")],
      program.programId
    );

    const [collateralMintAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("collateral"), usdcMint.toBuffer(), Buffer.from("authority")],
      program.programId
    );

    await program.methods
      .depositReserveLiquidity(new anchor.BN(DEPOSIT_AMOUNT))
      .accounts({
        market: marketPubkey,
        reserve: usdcReserve,
        destinationLiquidity: liquiditySupply,
        liquiditySupplyAuthority: liquiditySupplyAuthority,
        collateralMint: collateralMint,
        collateralMintAuthority: collateralMintAuthority,
        sourceLiquidity: userUsdcAccount,
        destinationCollateral: userCollateralAccount,
        userTransferAuthority: userKeypair.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([userKeypair])
      .rpc();

    // Verify deposit was successful
    const reserveAccount = await program.account.reserve.fetch(usdcReserve);
    assert.equal(reserveAccount.state.availableLiquidity.toString(), DEPOSIT_AMOUNT.toString());

    // Verify user received collateral tokens
    const userCollateralBalance = await provider.connection.getTokenAccountBalance(userCollateralAccount);
    assert.equal(userCollateralBalance.value.amount, DEPOSIT_AMOUNT.toString());
  });

  it("Initializes user obligation", async () => {
    await program.methods
      .initObligation()
      .accounts({
        market: marketPubkey,
        obligation: obligationPubkey,
        obligationOwner: userKeypair.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([userKeypair])
      .rpc();

    // Verify obligation was created
    const obligationAccount = await program.account.obligation.fetch(obligationPubkey);
    assert.equal(obligationAccount.owner.toString(), userKeypair.publicKey.toString());
    assert.equal(obligationAccount.deposits.length, 0);
    assert.equal(obligationAccount.borrows.length, 0);
  });

  it("Refreshes reserve interest", async () => {
    await program.methods
      .refreshReserve()
      .accounts({
        market: marketPubkey,
        reserve: usdcReserve,
        priceOracle: PublicKey.default, // Mock oracle
      })
      .rpc();

    // Verify reserve was refreshed
    const reserveAccount = await program.account.reserve.fetch(usdcReserve);
    assert.isTrue(reserveAccount.lastUpdateTimestamp > 0);
  });

  it("Handles error cases correctly", async () => {
    // Test depositing zero amount
    try {
      await program.methods
        .depositReserveLiquidity(new anchor.BN(0))
        .accounts({
          market: marketPubkey,
          reserve: usdcReserve,
          destinationLiquidity: PublicKey.default,
          liquiditySupplyAuthority: PublicKey.default,
          collateralMint: PublicKey.default,
          collateralMintAuthority: PublicKey.default,
          sourceLiquidity: PublicKey.default,
          destinationCollateral: PublicKey.default,
          userTransferAuthority: userKeypair.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([userKeypair])
        .rpc();
      
      assert.fail("Should have thrown error for zero deposit");
    } catch (error) {
      // Expected error for zero amount
      assert.include(error.toString(), "AmountTooSmall");
    }
  });

  it("Calculates interest rates correctly", async () => {
    const reserveAccount = await program.account.reserve.fetch(usdcReserve);
    
    // With 100 USDC deposited and 0 borrowed, utilization should be 0%
    const utilization = reserveAccount.state.currentUtilizationRate.value;
    assert.equal(utilization.toString(), "0");
    
    // Supply rate should be 0% when no one is borrowing
    const supplyRate = reserveAccount.state.currentSupplyRate.value;
    assert.equal(supplyRate.toString(), "0");
    
    // Borrow rate should be base rate (0%) when utilization is 0%
    const borrowRate = reserveAccount.state.currentBorrowRate.value;
    assert.equal(borrowRate.toString(), "0");
  });
});

// Helper functions for testing
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function calculateHealthFactor(
  collateralValueUSD: number,
  borrowedValueUSD: number,
  liquidationThreshold: number
): number {
  if (borrowedValueUSD === 0) return Number.MAX_SAFE_INTEGER;
  return (collateralValueUSD * liquidationThreshold) / borrowedValueUSD;
}

function calculateInterestRate(
  baseRateBps: number,
  multiplierBps: number,
  jumpMultiplierBps: number,
  optimalUtilizationBps: number,
  currentUtilizationBps: number
): number {
  const BASE_BPS = 10000;
  
  if (currentUtilizationBps <= optimalUtilizationBps) {
    return baseRateBps + (currentUtilizationBps * multiplierBps) / optimalUtilizationBps;
  } else {
    const excessUtilization = currentUtilizationBps - optimalUtilizationBps;
    const maxExcess = BASE_BPS - optimalUtilizationBps;
    
    return baseRateBps + multiplierBps + (excessUtilization * jumpMultiplierBps) / maxExcess;
  }
}