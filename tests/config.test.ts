import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { PublicKey, Keypair } from "@solana/web3.js";
import { AuraLend } from "../target/types/aura_lend";

describe("Configuration System Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AuraLend as Program<AuraLend>;
  
  let authority: Keypair;
  let configPubkey: PublicKey;
  let governancePubkey: PublicKey;
  
  before(async () => {
    authority = Keypair.generate();
    
    // Airdrop SOL to authority
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(authority.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL),
      "confirmed"
    );
    
    // Derive PDAs
    [configPubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("config")],
      program.programId
    );
    
    [governancePubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("governance")],
      program.programId
    );
  });

  describe("Configuration Initialization", () => {
    it("Should initialize protocol configuration with default values", async () => {
      const params = {
        maxReserves: new anchor.BN(128),
        defaultProtocolFeeBps: new anchor.BN(100),
        emergencyMode: false,
        pauseDeposits: false,
        pauseWithdrawals: false,
        pauseBorrows: false,
        pauseLiquidations: false,
        maxOracleConfidenceThreshold: new anchor.BN(100),
        computeUnitLimit: 400000,
      };
      
      await program.methods
        .initializeConfig(params)
        .accounts({
          config: configPubkey,
          authority: authority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([authority])
        .rpc();
      
      const config = await program.account.protocolConfig.fetch(configPubkey);
      
      expect(config.authority.toString()).to.equal(authority.publicKey.toString());
      expect(config.maxReserves.toNumber()).to.equal(128);
      expect(config.defaultProtocolFeeBps.toNumber()).to.equal(100);
      expect(config.emergencyMode).to.be.false;
    });

    it("Should validate configuration parameters during initialization", async () => {
      const invalidParams = {
        maxReserves: new anchor.BN(0), // Invalid: must be > 0
        defaultProtocolFeeBps: new anchor.BN(20000), // Invalid: exceeds basis points precision
        emergencyMode: false,
        pauseDeposits: false,
        pauseWithdrawals: false,
        pauseBorrows: false,
        pauseLiquidations: false,
        maxOracleConfidenceThreshold: new anchor.BN(100),
        computeUnitLimit: 400000,
      };
      
      const invalidConfig = Keypair.generate();
      
      try {
        await program.methods
          .initializeConfig(invalidParams)
          .accounts({
            config: invalidConfig.publicKey,
            authority: authority.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([authority, invalidConfig])
          .rpc();
        
        expect.fail("Should have failed with invalid configuration");
      } catch (error) {
        expect(error.toString()).to.include("InvalidConfiguration");
      }
    });
  });

  describe("Configuration Updates", () => {
    let configManager: Keypair;
    
    before(async () => {
      configManager = Keypair.generate();
      
      // Airdrop SOL
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(configManager.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
      );
      
      // Initialize governance (simplified for testing)
      try {
        await program.methods
          .initializeGovernance({
            maxRoles: new anchor.BN(200),
            defaultRoleExpiration: new anchor.BN(365 * 24 * 3600),
          })
          .accounts({
            governance: governancePubkey,
            authority: authority.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([authority])
          .rpc();
      } catch (error) {
        // Governance might already be initialized
      }
      
      // Grant CONFIG_MANAGER role to configManager
      try {
        await program.methods
          .grantRole({
            holder: configManager.publicKey,
            roleType: "ConfigManager",
            permissions: ["CONFIG_MANAGER"],
            expiresAt: new anchor.BN(Math.floor(Date.now() / 1000) + (365 * 24 * 60 * 60)),
          })
          .accounts({
            governance: governancePubkey,
            granter: authority.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([authority])
          .rpc();
      } catch (error) {
        // Role might already exist
      }
    });

    it("Should update configuration with proper permissions", async () => {
      const updateParams = {
        maxReserves: new anchor.BN(256),
        defaultProtocolFeeBps: new anchor.BN(150),
        maxLtvRatio: new anchor.BN(8000),
        minHealthFactor: new anchor.BN(1200000000000000000), // 1.2 in 18 decimal precision
        emergencyMode: null,
        pauseDeposits: null,
        pauseWithdrawals: null,
        pauseBorrows: null,
        pauseLiquidations: null,
      };
      
      const config = await program.account.protocolConfig.fetch(configPubkey);
      const [configHistoryPubkey] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("config_history"),
          configPubkey.toBuffer(),
          config.lastUpdatedSlot.toBuffer('le', 8)
        ],
        program.programId
      );
      
      await program.methods
        .updateConfig(updateParams, { medium: {} })
        .accounts({
          config: configPubkey,
          governance: governancePubkey,
          authority: configManager.publicKey,
          configHistory: configHistoryPubkey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([configManager])
        .rpc();
      
      const updatedConfig = await program.account.protocolConfig.fetch(configPubkey);
      
      expect(updatedConfig.maxReserves.toNumber()).to.equal(256);
      expect(updatedConfig.defaultProtocolFeeBps.toNumber()).to.equal(150);
      expect(updatedConfig.maxLtvRatio.toNumber()).to.equal(8000);
    });

    it("Should reject updates without proper permissions", async () => {
      const unauthorizedUser = Keypair.generate();
      
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(unauthorizedUser.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
      );
      
      const updateParams = {
        maxReserves: new anchor.BN(512),
        defaultProtocolFeeBps: null,
        maxLtvRatio: null,
        minHealthFactor: null,
        emergencyMode: null,
        pauseDeposits: null,
        pauseWithdrawals: null,
        pauseBorrows: null,
        pauseLiquidations: null,
      };
      
      const config = await program.account.protocolConfig.fetch(configPubkey);
      const [configHistoryPubkey] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("config_history"),
          configPubkey.toBuffer(),
          config.lastUpdatedSlot.toBuffer('le', 8)
        ],
        program.programId
      );
      
      try {
        await program.methods
          .updateConfig(updateParams, { medium: {} })
          .accounts({
            config: configPubkey,
            governance: governancePubkey,
            authority: unauthorizedUser.publicKey,
            configHistory: configHistoryPubkey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([unauthorizedUser])
          .rpc();
        
        expect.fail("Should have failed with insufficient permissions");
      } catch (error) {
        expect(error.toString()).to.include("InsufficientPermissions");
      }
    });

    it("Should validate updated parameters", async () => {
      const invalidParams = {
        maxReserves: null,
        defaultProtocolFeeBps: new anchor.BN(15000), // Invalid: exceeds max
        maxLtvRatio: null,
        minHealthFactor: null,
        emergencyMode: null,
        pauseDeposits: null,
        pauseWithdrawals: null,
        pauseBorrows: null,
        pauseLiquidations: null,
      };
      
      const config = await program.account.protocolConfig.fetch(configPubkey);
      const [configHistoryPubkey] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("config_history"),
          configPubkey.toBuffer(),
          config.lastUpdatedSlot.toBuffer('le', 8)
        ],
        program.programId
      );
      
      try {
        await program.methods
          .updateConfig(invalidParams, { medium: {} })
          .accounts({
            config: configPubkey,
            governance: governancePubkey,
            authority: configManager.publicKey,
            configHistory: configHistoryPubkey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([configManager])
          .rpc();
        
        expect.fail("Should have failed with invalid configuration");
      } catch (error) {
        expect(error.toString()).to.include("InvalidConfiguration");
      }
    });
  });

  describe("Emergency Configuration", () => {
    let emergencyResponder: Keypair;
    
    before(async () => {
      emergencyResponder = Keypair.generate();
      
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(emergencyResponder.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
      );
      
      // Grant EMERGENCY_RESPONDER role
      try {
        await program.methods
          .grantRole({
            holder: emergencyResponder.publicKey,
            roleType: "EmergencyResponder",
            permissions: ["EMERGENCY_RESPONDER"],
            expiresAt: new anchor.BN(Math.floor(Date.now() / 1000) + (24 * 60 * 60)), // 24 hours
          })
          .accounts({
            governance: governancePubkey,
            granter: authority.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([authority])
          .rpc();
      } catch (error) {
        // Role might already exist
      }
    });

    it("Should handle emergency configuration updates", async () => {
      const emergencyParams = {
        emergencyMode: true,
        pauseDeposits: true,
        pauseWithdrawals: false,
        pauseBorrows: true,
        pauseLiquidations: false,
      };
      
      await program.methods
        .emergencyConfigUpdate(emergencyParams)
        .accounts({
          config: configPubkey,
          governance: governancePubkey,
          emergencyAuthority: emergencyResponder.publicKey,
        })
        .signers([emergencyResponder])
        .rpc();
      
      const config = await program.account.protocolConfig.fetch(configPubkey);
      
      expect(config.emergencyMode).to.be.true;
      expect(config.pauseDeposits).to.be.true;
      expect(config.pauseWithdrawals).to.be.false;
      expect(config.pauseBorrows).to.be.true;
      expect(config.pauseLiquidations).to.be.false;
    });

    it("Should reject emergency updates from unauthorized users", async () => {
      const unauthorizedUser = Keypair.generate();
      
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(unauthorizedUser.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
      );
      
      const emergencyParams = {
        emergencyMode: false,
        pauseDeposits: false,
        pauseWithdrawals: false,
        pauseBorrows: false,
        pauseLiquidations: false,
      };
      
      try {
        await program.methods
          .emergencyConfigUpdate(emergencyParams)
          .accounts({
            config: configPubkey,
            governance: governancePubkey,
            emergencyAuthority: unauthorizedUser.publicKey,
          })
          .signers([unauthorizedUser])
          .rpc();
        
        expect.fail("Should have failed with insufficient permissions");
      } catch (error) {
        expect(error.toString()).to.include("InsufficientPermissions");
      }
    });
  });

  describe("Configuration Queries", () => {
    it("Should retrieve current configuration", async () => {
      const config = await program.methods
        .getConfig()
        .accounts({
          config: configPubkey,
        })
        .view();
      
      expect(config).to.not.be.undefined;
      expect(config.authority.toString()).to.equal(authority.publicKey.toString());
      expect(config.version).to.equal(1);
    });

    it("Should check pause states correctly", async () => {
      const config = await program.account.protocolConfig.fetch(configPubkey);
      
      // With emergency mode on, deposits and borrows should be paused
      expect(config.emergencyMode).to.be.true;
      expect(config.pauseDeposits).to.be.true;
      expect(config.pauseBorrows).to.be.true;
      
      // Withdrawals and liquidations should still be allowed
      expect(config.pauseWithdrawals).to.be.false;
      expect(config.pauseLiquidations).to.be.false;
    });
  });

  describe("Configuration History", () => {
    it("Should track configuration changes", async () => {
      // First, get current config to create new history entry
      const config = await program.account.protocolConfig.fetch(configPubkey);
      
      const updateParams = {
        maxReserves: null,
        defaultProtocolFeeBps: new anchor.BN(200),
        maxLtvRatio: null,
        minHealthFactor: null,
        emergencyMode: null,
        pauseDeposits: null,
        pauseWithdrawals: null,
        pauseBorrows: null,
        pauseLiquidations: null,
      };
      
      const [configHistoryPubkey] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("config_history"),
          configPubkey.toBuffer(),
          config.lastUpdatedSlot.toBuffer('le', 8)
        ],
        program.programId
      );
      
      const configManager = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(configManager.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
      );
      
      // Grant permissions
      try {
        await program.methods
          .grantRole({
            holder: configManager.publicKey,
            roleType: "ConfigManager",
            permissions: ["CONFIG_MANAGER"],
            expiresAt: new anchor.BN(Math.floor(Date.now() / 1000) + (365 * 24 * 60 * 60)),
          })
          .accounts({
            governance: governancePubkey,
            granter: authority.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([authority])
          .rpc();
      } catch (error) {
        // Role might already exist
      }
      
      await program.methods
        .updateConfig(updateParams, { medium: {} })
        .accounts({
          config: configPubkey,
          governance: governancePubkey,
          authority: configManager.publicKey,
          configHistory: configHistoryPubkey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([configManager])
        .rpc();
      
      const history = await program.account.configHistory.fetch(configHistoryPubkey);
      
      expect(history.configAddress.toString()).to.equal(configPubkey.toString());
      expect(history.updatedBy.toString()).to.equal(configManager.publicKey.toString());
      expect(history.changes.length).to.be.greaterThan(0);
    });
  });
});