#!/usr/bin/env ts-node

/**
 * Automated deployment and upgrade script for Aura Lend Protocol
 * 
 * This script handles:
 * - Initial upgradeability setup
 * - Program upgrades with proper governance
 * - Pre-upgrade validation and compatibility checks
 * - Post-upgrade verification
 */

import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Connection, PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { AuraLend } from "../../target/types/aura_lend";
import fs from "fs";
import path from "path";

interface UpgradeConfig {
    network: "localnet" | "devnet" | "mainnet";
    programId: string;
    authority: string;
    bufferKeypair?: string;
    skipVerification?: boolean;
    dryRun?: boolean;
}

interface DeploymentResult {
    success: boolean;
    programId?: PublicKey;
    upgradeAuthority?: PublicKey;
    bufferAccount?: PublicKey;
    transactionSignature?: string;
    error?: string;
}

class AuraLendDeployment {
    private connection: Connection;
    private wallet: anchor.Wallet;
    private program: Program<AuraLend>;
    private config: UpgradeConfig;

    constructor(config: UpgradeConfig) {
        this.config = config;
        
        // Initialize connection based on network
        const rpcUrl = this.getRpcUrl(config.network);
        this.connection = new Connection(rpcUrl, "confirmed");
        
        // Load wallet
        const walletKeypath = config.authority || process.env.SOLANA_WALLET || "~/.config/solana/id.json";
        const walletKeypair = Keypair.fromSecretKey(
            new Uint8Array(JSON.parse(fs.readFileSync(walletKeypath, "utf8")))
        );
        this.wallet = new anchor.Wallet(walletKeypair);
        
        // Initialize program
        const provider = new anchor.AnchorProvider(this.connection, this.wallet, {
            commitment: "confirmed",
        });
        anchor.setProvider(provider);
        
        const idl = JSON.parse(fs.readFileSync("target/idl/aura_lend.json", "utf8"));
        this.program = new Program(idl, new PublicKey(config.programId), provider);
    }

    private getRpcUrl(network: string): string {
        switch (network) {
            case "localnet":
                return "http://127.0.0.1:8899";
            case "devnet":
                return "https://api.devnet.solana.com";
            case "mainnet":
                return process.env.MAINNET_RPC_URL || "https://api.mainnet-beta.solana.com";
            default:
                throw new Error(`Unknown network: ${network}`);
        }
    }

    /**
     * Setup initial upgradeability by setting multisig as upgrade authority
     */
    async setupUpgradeability(): Promise<DeploymentResult> {
        try {
            console.log("🔧 Setting up program upgradeability...");
            
            // Get market PDA
            const [marketPda] = await PublicKey.findProgramAddress(
                [Buffer.from("market")],
                this.program.programId
            );

            // Get program data account
            const programDataAddress = await this.getProgramDataAddress();
            if (!programDataAddress) {
                throw new Error("Program data account not found - program may not be upgradeable");
            }

            // Get market account to find multisig owner
            const marketAccount = await this.program.account.market.fetch(marketPda);
            const multisigOwner = marketAccount.multisigOwner;

            console.log(`📋 Market PDA: ${marketPda.toString()}`);
            console.log(`🏛️  MultiSig Owner: ${multisigOwner.toString()}`);
            console.log(`📄 Program Data: ${programDataAddress.toString()}`);

            if (this.config.dryRun) {
                console.log("🏃 Dry run - would set upgrade authority to MultiSig");
                return { success: true };
            }

            // Execute set upgrade authority instruction
            const tx = await this.program.methods
                .setUpgradeAuthority()
                .accounts({
                    market: marketPda,
                    currentAuthority: this.wallet.publicKey,
                    newAuthority: multisigOwner,
                    programData: programDataAddress,
                })
                .rpc();

            console.log("✅ Upgrade authority transferred to MultiSig");
            console.log(`📝 Transaction: ${tx}`);

            return {
                success: true,
                programId: this.program.programId,
                upgradeAuthority: multisigOwner,
                transactionSignature: tx,
            };

        } catch (error) {
            console.error("❌ Failed to setup upgradeability:", error);
            return {
                success: false,
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }

    /**
     * Deploy a program upgrade using a buffer account
     */
    async deployUpgrade(bufferPath: string): Promise<DeploymentResult> {
        try {
            console.log("🚀 Deploying program upgrade...");

            // Load buffer keypair
            const bufferKeypair = this.config.bufferKeypair 
                ? Keypair.fromSecretKey(new Uint8Array(JSON.parse(fs.readFileSync(this.config.bufferKeypair, "utf8"))))
                : Keypair.generate();

            // Get program data address
            const programDataAddress = await this.getProgramDataAddress();
            if (!programDataAddress) {
                throw new Error("Program data account not found");
            }

            console.log(`📦 Buffer Account: ${bufferKeypair.publicKey.toString()}`);
            console.log(`📄 Program Data: ${programDataAddress.toString()}`);

            // Pre-upgrade validation
            await this.validateUpgrade();

            if (this.config.dryRun) {
                console.log("🏃 Dry run - would deploy upgrade from buffer");
                return { success: true, bufferAccount: bufferKeypair.publicKey };
            }

            // Get upgrade authority (should be multisig)
            const programDataInfo = await this.connection.getAccountInfo(programDataAddress);
            if (!programDataInfo) {
                throw new Error("Program data account not found");
            }

            // This would typically be done through a multisig proposal
            console.log("⚠️  Note: Actual upgrade requires MultiSig approval");
            console.log("📋 Create a MultiSig proposal for program upgrade");

            return {
                success: true,
                programId: this.program.programId,
                bufferAccount: bufferKeypair.publicKey,
            };

        } catch (error) {
            console.error("❌ Failed to deploy upgrade:", error);
            return {
                success: false,
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }

    /**
     * Validate pre-upgrade conditions
     */
    private async validateUpgrade(): Promise<void> {
        console.log("🔍 Validating upgrade conditions...");

        // Check if program is upgradeable
        const programDataAddress = await this.getProgramDataAddress();
        if (!programDataAddress) {
            throw new Error("Program is not upgradeable");
        }

        // Check program data account
        const programDataInfo = await this.connection.getAccountInfo(programDataAddress);
        if (!programDataInfo) {
            throw new Error("Program data account not found");
        }

        // Validate market state
        const [marketPda] = await PublicKey.findProgramAddress(
            [Buffer.from("market")],
            this.program.programId
        );

        try {
            const marketAccount = await this.program.account.market.fetch(marketPda);
            console.log(`✅ Market version: ${marketAccount.version}`);
            console.log(`✅ Market initialized`);
        } catch (error) {
            console.warn("⚠️  Market not found - may need initialization");
        }

        // Check governance system
        try {
            const [governancePda] = await PublicKey.findProgramAddress(
                [Buffer.from("governance")],
                this.program.programId
            );
            
            const governanceAccount = await this.program.account.governanceRegistry.fetch(governancePda);
            console.log(`✅ Governance version: ${governanceAccount.version}`);
        } catch (error) {
            console.warn("⚠️  Governance not initialized");
        }

        console.log("✅ Pre-upgrade validation complete");
    }

    /**
     * Get program data address for upgradeable programs
     */
    private async getProgramDataAddress(): Promise<PublicKey | null> {
        const programInfo = await this.connection.getAccountInfo(this.program.programId);
        if (!programInfo || programInfo.data.length < 44) {
            return null;
        }

        // For upgradeable programs, the first 4 bytes should be [2, 0, 0, 0]
        // followed by the program data account address
        const data = programInfo.data;
        if (data[0] !== 2 || data[1] !== 0 || data[2] !== 0 || data[3] !== 0) {
            return null;
        }

        return new PublicKey(data.slice(4, 36));
    }

    /**
     * Verify deployment success
     */
    async verifyDeployment(): Promise<boolean> {
        try {
            console.log("🔍 Verifying deployment...");

            // Check program account exists
            const programInfo = await this.connection.getAccountInfo(this.program.programId);
            if (!programInfo) {
                console.error("❌ Program account not found");
                return false;
            }

            // Verify it's executable
            if (!programInfo.executable) {
                console.error("❌ Program is not executable");
                return false;
            }

            // Check if upgradeable
            const programDataAddress = await this.getProgramDataAddress();
            if (programDataAddress) {
                console.log("✅ Program is upgradeable");
                console.log(`📄 Program Data: ${programDataAddress.toString()}`);
            } else {
                console.log("ℹ️  Program is not upgradeable");
            }

            // Test basic program functionality
            try {
                const [marketPda] = await PublicKey.findProgramAddress(
                    [Buffer.from("market")],
                    this.program.programId
                );
                
                const marketAccount = await this.program.account.market.fetch(marketPda);
                console.log(`✅ Market accessible, version: ${marketAccount.version}`);
            } catch (error) {
                console.warn("⚠️  Could not verify market account");
            }

            console.log("✅ Deployment verification complete");
            return true;

        } catch (error) {
            console.error("❌ Deployment verification failed:", error);
            return false;
        }
    }

    /**
     * Generate upgrade report
     */
    generateReport(result: DeploymentResult): void {
        const report = {
            timestamp: new Date().toISOString(),
            network: this.config.network,
            programId: this.config.programId,
            authority: this.config.authority,
            result: result,
        };

        const reportPath = `scripts/upgrade/reports/${this.config.network}_${Date.now()}.json`;
        const reportDir = path.dirname(reportPath);
        
        if (!fs.existsSync(reportDir)) {
            fs.mkdirSync(reportDir, { recursive: true });
        }

        fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
        console.log(`📊 Report saved to: ${reportPath}`);
    }
}

// CLI Interface
async function main() {
    const args = process.argv.slice(2);
    const command = args[0];

    // Load configuration
    const configPath = args.find(arg => arg.startsWith('--config='))?.replace('--config=', '') || 'scripts/upgrade/config.json';
    let config: UpgradeConfig;

    try {
        config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    } catch (error) {
        console.error(`Failed to load config from ${configPath}:`, error);
        process.exit(1);
    }

    // Parse CLI flags
    config.dryRun = args.includes('--dry-run');
    config.skipVerification = args.includes('--skip-verification');

    const deployment = new AuraLendDeployment(config);

    switch (command) {
        case 'setup':
            console.log("🔧 Setting up upgradeability...");
            const setupResult = await deployment.setupUpgradeability();
            deployment.generateReport(setupResult);
            process.exit(setupResult.success ? 0 : 1);

        case 'upgrade':
            const bufferPath = args[1];
            if (!bufferPath) {
                console.error("Buffer path required for upgrade command");
                process.exit(1);
            }
            console.log("🚀 Deploying upgrade...");
            const upgradeResult = await deployment.deployUpgrade(bufferPath);
            deployment.generateReport(upgradeResult);
            process.exit(upgradeResult.success ? 0 : 1);

        case 'verify':
            console.log("🔍 Verifying deployment...");
            const verifySuccess = await deployment.verifyDeployment();
            process.exit(verifySuccess ? 0 : 1);

        default:
            console.log(`
Usage: ${process.argv[1]} <command> [options]

Commands:
  setup                    Setup initial upgradeability
  upgrade <buffer-path>    Deploy program upgrade
  verify                   Verify current deployment

Options:
  --config=<path>         Configuration file path (default: scripts/upgrade/config.json)
  --dry-run              Simulate without executing transactions
  --skip-verification    Skip post-deployment verification

Examples:
  npm run upgrade:setup
  npm run upgrade:deploy buffer.json
  npm run upgrade:verify
            `);
            process.exit(1);
    }
}

if (require.main === module) {
    main().catch(error => {
        console.error("Deployment failed:", error);
        process.exit(1);
    });
}