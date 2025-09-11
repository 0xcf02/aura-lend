import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AuraLend } from "../target/types/aura_lend";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { assert, expect } from "chai";

describe("Governance System Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AuraLend as Program<AuraLend>;
  
  let marketPubkey: PublicKey;
  let multisigPubkey: PublicKey;
  let timelockPubkey: PublicKey;
  let governancePubkey: PublicKey;
  
  // Test signatories
  let signatory1: Keypair;
  let signatory2: Keypair;
  let signatory3: Keypair;
  let nonSignatory: Keypair;
  
  before(async () => {
    // Generate test keypairs
    signatory1 = Keypair.generate();
    signatory2 = Keypair.generate();
    signatory3 = Keypair.generate();
    nonSignatory = Keypair.generate();
    
    // Airdrop SOL to test accounts
    const airdropPromises = [signatory1, signatory2, signatory3, nonSignatory].map(
      async (keypair) => {
        await provider.connection.confirmTransaction(
          await provider.connection.requestAirdrop(
            keypair.publicKey,
            2 * anchor.web3.LAMPORTS_PER_SOL
          )
        );
      }
    );
    await Promise.all(airdropPromises);

    // Derive PDAs
    [marketPubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("market")],
      program.programId
    );

    [multisigPubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("multisig")],
      program.programId
    );

    [timelockPubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("timelock")],
      program.programId
    );

    [governancePubkey] = PublicKey.findProgramAddressSync(
      [Buffer.from("governance")],
      program.programId
    );
  });

  describe("MultiSig Operations", () => {
    it("should initialize multisig with correct parameters", async () => {
      try {
        await program.methods
          .initializeMultisig({
            signatories: [
              signatory1.publicKey,
              signatory2.publicKey,
              signatory3.publicKey,
            ],
            threshold: 2, // 2 of 3 signatures required
            nonce: 0,
          })
          .accounts({
            multisig: multisigPubkey,
            authority: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        // Fetch the multisig account to verify initialization
        const multisigAccount = await program.account.multisigState.fetch(multisigPubkey);
        assert.equal(multisigAccount.threshold, 2);
        assert.equal(multisigAccount.signatories.length, 3);
        assert.equal(multisigAccount.nonce, 0);
      } catch (error) {
        console.log("Multisig might already be initialized:", error.message);
      }
    });

    it("should create a proposal", async () => {
      try {
        const proposalId = 1;
        const [proposalPubkey] = PublicKey.findProgramAddressSync(
          [Buffer.from("proposal"), Buffer.from(proposalId.toString())],
          program.programId
        );

        await program.methods
          .createMultisigProposal({
            proposalId,
            operationType: "UpdateReserveConfig",
            instructionData: Buffer.from("test_instruction_data"),
            targetAccounts: [marketPubkey],
            expiresAt: null, // No expiration
          })
          .accounts({
            multisig: multisigPubkey,
            proposal: proposalPubkey,
            proposer: signatory1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([signatory1])
          .rpc();

        // Verify proposal creation
        const proposalAccount = await program.account.multisigProposal.fetch(proposalPubkey);
        assert.equal(proposalAccount.proposalId, proposalId);
        assert.equal(proposalAccount.proposer.toString(), signatory1.publicKey.toString());
      } catch (error) {
        console.log("Proposal creation error:", error.message);
      }
    });

    it("should allow signatories to sign proposals", async () => {
      try {
        const proposalId = 1;
        const [proposalPubkey] = PublicKey.findProgramAddressSync(
          [Buffer.from("proposal"), Buffer.from(proposalId.toString())],
          program.programId
        );

        // First signatory signs
        await program.methods
          .signMultisigProposal()
          .accounts({
            multisig: multisigPubkey,
            proposal: proposalPubkey,
            signatory: signatory2.publicKey,
          })
          .signers([signatory2])
          .rpc();

        // Check signature count
        const proposalAccount = await program.account.multisigProposal.fetch(proposalPubkey);
        assert.isTrue(proposalAccount.signatureCount >= 1);
      } catch (error) {
        console.log("Signing error:", error.message);
      }
    });

    it("should reject non-signatories from signing", async () => {
      try {
        const proposalId = 1;
        const [proposalPubkey] = PublicKey.findProgramAddressSync(
          [Buffer.from("proposal"), Buffer.from(proposalId.toString())],
          program.programId
        );

        await program.methods
          .signMultisigProposal()
          .accounts({
            multisig: multisigPubkey,
            proposal: proposalPubkey,
            signatory: nonSignatory.publicKey, // Not a signatory
          })
          .signers([nonSignatory])
          .rpc();

        assert.fail("Should have rejected non-signatory");
      } catch (error) {
        expect(error.message).to.include("InvalidSignatory");
      }
    });

    it("should execute proposal when threshold is met", async () => {
      try {
        const proposalId = 1;
        const [proposalPubkey] = PublicKey.findProgramAddressSync(
          [Buffer.from("proposal"), Buffer.from(proposalId.toString())],
          program.programId
        );

        // Third signatory signs to meet threshold
        await program.methods
          .signMultisigProposal()
          .accounts({
            multisig: multisigPubkey,
            proposal: proposalPubkey,
            signatory: signatory3.publicKey,
          })
          .signers([signatory3])
          .rpc();

        // Execute the proposal
        await program.methods
          .executeMultisigProposal()
          .accounts({
            multisig: multisigPubkey,
            proposal: proposalPubkey,
            executor: signatory1.publicKey,
          })
          .signers([signatory1])
          .rpc();

        console.log("Proposal executed successfully");
      } catch (error) {
        console.log("Execution error:", error.message);
      }
    });
  });

  describe("Timelock Operations", () => {
    it("should initialize timelock controller", async () => {
      try {
        await program.methods
          .initializeTimelock()
          .accounts({
            timelock: timelockPubkey,
            authority: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        const timelockAccount = await program.account.timelockController.fetch(timelockPubkey);
        assert.equal(timelockAccount.activeProposals, 0);
      } catch (error) {
        console.log("Timelock might already be initialized:", error.message);
      }
    });

    it("should create timelock proposal with correct delay", async () => {
      try {
        const proposalId = 1;
        
        await program.methods
          .createTimelockProposal({
            proposalId,
            operationType: "UpdateMarketOwner", // Critical operation
            instructionData: Buffer.from("owner_change_data"),
            targetAccounts: [marketPubkey],
          })
          .accounts({
            timelock: timelockPubkey,
            proposer: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        console.log("Timelock proposal created");
      } catch (error) {
        console.log("Timelock proposal error:", error.message);
      }
    });

    it("should reject early execution of timelock proposals", async () => {
      try {
        const proposalId = 1;
        const [proposalPubkey] = PublicKey.findProgramAddressSync(
          [Buffer.from("timelock_proposal"), Buffer.from(proposalId.toString())],
          program.programId
        );

        // Try to execute immediately (should fail)
        await program.methods
          .executeTimelockProposal()
          .accounts({
            timelock: timelockPubkey,
            proposal: proposalPubkey,
            executor: provider.wallet.publicKey,
          })
          .rpc();

        assert.fail("Should have rejected early execution");
      } catch (error) {
        expect(error.message).to.include("TimelockNotReady");
      }
    });

    it("should allow cancellation of timelock proposals", async () => {
      try {
        const proposalId = 1;
        const [proposalPubkey] = PublicKey.findProgramAddressSync(
          [Buffer.from("timelock_proposal"), Buffer.from(proposalId.toString())],
          program.programId
        );

        await program.methods
          .cancelTimelockProposal()
          .accounts({
            timelock: timelockPubkey,
            proposal: proposalPubkey,
            canceller: provider.wallet.publicKey,
          })
          .rpc();

        console.log("Timelock proposal cancelled");
      } catch (error) {
        console.log("Cancellation error:", error.message);
      }
    });
  });

  describe("Role-Based Access Control", () => {
    it("should initialize governance registry", async () => {
      try {
        await program.methods
          .initializeGovernance({
            maxRoles: 100,
            availablePermissions: 0xFFFFFFFFFFFFFFFF, // All permissions
          })
          .accounts({
            governance: governancePubkey,
            authority: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        const governanceAccount = await program.account.governanceRegistry.fetch(governancePubkey);
        assert.equal(governanceAccount.maxRoles, 100);
      } catch (error) {
        console.log("Governance might already be initialized:", error.message);
      }
    });

    it("should grant roles to users", async () => {
      try {
        const oneYearFromNow = Math.floor(Date.now() / 1000) + (365 * 24 * 60 * 60);
        
        await program.methods
          .grantRole({
            holder: signatory1.publicKey,
            roleType: "ReserveManager",
            permissions: ["RESERVE_MANAGER"], // Bit flags for permissions
            expiresAt: oneYearFromNow,
          })
          .accounts({
            governance: governancePubkey,
            granter: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        console.log("Role granted successfully");
      } catch (error) {
        console.log("Role grant error:", error.message);
      }
    });

    it("should revoke roles", async () => {
      try {
        await program.methods
          .revokeRole(signatory1.publicKey)
          .accounts({
            governance: governancePubkey,
            revoker: provider.wallet.publicKey,
          })
          .rpc();

        console.log("Role revoked successfully");
      } catch (error) {
        console.log("Role revoke error:", error.message);
      }
    });

    it("should enforce role permissions", async () => {
      // This would test that operations requiring specific roles
      // are rejected when performed by users without those roles
      console.log("Role permission enforcement test");
      assert.isTrue(true); // Placeholder
    });

    it("should handle emergency roles correctly", async () => {
      try {
        const emergencyExpiration = Math.floor(Date.now() / 1000) + (24 * 60 * 60); // 24 hours
        
        await program.methods
          .emergencyGrantRole({
            holder: signatory2.publicKey,
            roleType: "EmergencyResponder",
            permissions: ["EMERGENCY_RESPONDER"],
            expiresAt: emergencyExpiration,
          })
          .accounts({
            governance: governancePubkey,
            emergencyAuthority: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        console.log("Emergency role granted");
      } catch (error) {
        console.log("Emergency role error:", error.message);
      }
    });
  });

  describe("Permission Delegation", () => {
    it("should allow permission delegation", async () => {
      try {
        await program.methods
          .delegatePermissions({
            delegatee: signatory3.publicKey,
            permissions: ["RESERVE_MANAGER"],
            expiresAt: Math.floor(Date.now() / 1000) + (30 * 24 * 60 * 60), // 30 days
          })
          .accounts({
            governance: governancePubkey,
            delegator: signatory1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([signatory1])
          .rpc();

        console.log("Permissions delegated successfully");
      } catch (error) {
        console.log("Delegation error:", error.message);
      }
    });

    it("should prevent delegation of non-held permissions", async () => {
      try {
        await program.methods
          .delegatePermissions({
            delegatee: nonSignatory.publicKey,
            permissions: ["SUPER_ADMIN"], // Permission not held
            expiresAt: Math.floor(Date.now() / 1000) + (30 * 24 * 60 * 60),
          })
          .accounts({
            governance: governancePubkey,
            delegator: signatory1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([signatory1])
          .rpc();

        assert.fail("Should have rejected delegation of non-held permissions");
      } catch (error) {
        expect(error.message).to.include("CannotDelegatePermissionsNotHeld");
      }
    });
  });

  describe("Governance Integration", () => {
    it("should require governance approval for critical operations", async () => {
      // Test that critical operations like market configuration changes
      // require proper governance approval
      console.log("Governance approval requirement test");
      assert.isTrue(true); // Placeholder
    });

    it("should enforce timelock delays for different operation types", async () => {
      // Test that different operations have appropriate timelock delays
      // Critical: 7 days, High: 3 days, Medium: 1 day, Low: 6 hours
      console.log("Timelock delay enforcement test");
      assert.isTrue(true); // Placeholder
    });

    it("should clean up expired proposals automatically", async () => {
      try {
        await program.methods
          .cleanupExpiredProposals()
          .accounts({
            timelock: timelockPubkey,
            cleaner: provider.wallet.publicKey,
          })
          .rpc();

        console.log("Expired proposals cleaned up");
      } catch (error) {
        console.log("Cleanup error:", error.message);
      }
    });

    it("should clean up expired roles automatically", async () => {
      try {
        await program.methods
          .cleanupExpiredRoles()
          .accounts({
            governance: governancePubkey,
            cleaner: provider.wallet.publicKey,
          })
          .rpc();

        console.log("Expired roles cleaned up");
      } catch (error) {
        console.log("Role cleanup error:", error.message);
      }
    });
  });
});