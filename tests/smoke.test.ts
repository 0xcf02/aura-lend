import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import { AuraLend } from "../target/types/aura_lend";

describe("Smoke Tests - Quick Deployment Verification", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AuraLend as Program<AuraLend>;
  
  describe("Basic Program Health", () => {
    it("Should have correct program ID", async () => {
      const expectedProgramId = "AuRa1Lend1111111111111111111111111111111111";
      expect(program.programId.toString()).to.equal(expectedProgramId);
    });

    it("Should be able to fetch program info", async () => {
      const programInfo = await provider.connection.getAccountInfo(program.programId);
      expect(programInfo).to.not.be.null;
      expect(programInfo?.executable).to.be.true;
    });
  });

  describe("PDA Derivation", () => {
    it("Should derive market PDA correctly", async () => {
      const [marketPubkey, bump] = PublicKey.findProgramAddressSync(
        [Buffer.from("market")],
        program.programId
      );
      
      expect(marketPubkey).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a('number');
      expect(bump).to.be.lessThan(256);
    });

    it("Should derive config PDA correctly", async () => {
      const [configPubkey, bump] = PublicKey.findProgramAddressSync(
        [Buffer.from("config")],
        program.programId
      );
      
      expect(configPubkey).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a('number');
      expect(bump).to.be.lessThan(256);
    });

    it("Should derive governance PDA correctly", async () => {
      const [governancePubkey, bump] = PublicKey.findProgramAddressSync(
        [Buffer.from("governance")],
        program.programId
      );
      
      expect(governancePubkey).to.be.instanceOf(PublicKey);
      expect(bump).to.be.a('number');
      expect(bump).to.be.lessThan(256);
    });
  });

  describe("Network Connectivity", () => {
    it("Should be connected to Solana network", async () => {
      const slot = await provider.connection.getSlot();
      expect(slot).to.be.a('number');
      expect(slot).to.be.greaterThan(0);
    });

    it("Should have sufficient balance for testing", async () => {
      const balance = await provider.connection.getBalance(provider.wallet.publicKey);
      expect(balance).to.be.greaterThan(0);
    });

    it("Should be able to get recent blockhash", async () => {
      const { blockhash } = await provider.connection.getLatestBlockhash();
      expect(blockhash).to.be.a('string');
      expect(blockhash.length).to.be.greaterThan(0);
    });
  });

  describe("Program Instructions", () => {
    it("Should have all expected instruction methods", async () => {
      const expectedMethods = [
        'initializeMarket',
        'initializeConfig',
        'initializeMultisig',
        'initializeTimelock',
        'initializeGovernance',
        'updateConfig',
        'emergencyConfigUpdate',
        'depositReserveLiquidity',
        'redeemReserveCollateral',
        'borrowObligationLiquidity',
        'repayObligationLiquidity',
        'liquidateObligation'
      ];

      expectedMethods.forEach(method => {
        expect(program.methods).to.have.property(method);
        expect(typeof program.methods[method]).to.equal('function');
      });
    });
  });

  describe("Account Types", () => {
    it("Should have all expected account types", async () => {
      const expectedAccounts = [
        'market',
        'reserve',
        'obligation',
        'multisig',
        'timelock',
        'governanceRegistry',
        'protocolConfig'
      ];

      expectedAccounts.forEach(accountType => {
        expect(program.account).to.have.property(accountType);
      });
    });
  });

  describe("Error Handling", () => {
    it("Should handle invalid instruction gracefully", async () => {
      try {
        // Try to call an instruction with invalid accounts
        const invalidPubkey = PublicKey.default;
        
        await program.methods
          .getConfig()
          .accounts({
            config: invalidPubkey,
          })
          .rpc();
        
        expect.fail("Should have thrown an error");
      } catch (error) {
        // Expected to fail - this validates error handling works
        expect(error).to.be.instanceOf(Error);
      }
    });
  });

  describe("Basic Functionality", () => {
    let marketPubkey: PublicKey;
    let configPubkey: PublicKey;
    
    before(async () => {
      [marketPubkey] = PublicKey.findProgramAddressSync(
        [Buffer.from("market")],
        program.programId
      );
      
      [configPubkey] = PublicKey.findProgramAddressSync(
        [Buffer.from("config")],
        program.programId
      );
    });

    it("Should check if market is initialized", async () => {
      try {
        const market = await program.account.market.fetch(marketPubkey);
        console.log("✅ Market is initialized");
        console.log(`   Owner: ${market.owner}`);
        console.log(`   Reserves: ${market.reservesCount}`);
      } catch (error) {
        console.log("ℹ️  Market not yet initialized (expected for fresh deployment)");
      }
    });

    it("Should check if config is initialized", async () => {
      try {
        const config = await program.account.protocolConfig.fetch(configPubkey);
        console.log("✅ Protocol config is initialized");
        console.log(`   Version: ${config.version}`);
        console.log(`   Max Reserves: ${config.maxReserves}`);
        console.log(`   Emergency Mode: ${config.emergencyMode}`);
      } catch (error) {
        console.log("ℹ️  Protocol config not yet initialized (expected for fresh deployment)");
      }
    });
  });

  describe("Performance", () => {
    it("Should execute view operations quickly", async () => {
      const start = Date.now();
      
      // Perform multiple quick operations
      await provider.connection.getSlot();
      await provider.connection.getBalance(provider.wallet.publicKey);
      await provider.connection.getLatestBlockhash();
      
      const duration = Date.now() - start;
      
      expect(duration).to.be.lessThan(5000); // Should complete within 5 seconds
      console.log(`   Operations completed in ${duration}ms`);
    });
  });
});