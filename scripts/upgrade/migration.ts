#!/usr/bin/env ts-node

/**
 * Data migration script for Aura Lend Protocol upgrades
 * 
 * Handles migration of state accounts between different program versions
 */

import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { AuraLend } from "../../target/types/aura_lend";
import fs from "fs";
import path from "path";

interface MigrationConfig {
    network: "localnet" | "devnet" | "mainnet";
    programId: string;
    authority: string;
    fromVersion: number;
    toVersion: number;
    batchSize: number;
    dryRun?: boolean;
}

interface MigrationResult {
    success: boolean;
    totalAccounts: number;
    migratedAccounts: number;
    failedAccounts: number;
    errors: string[];
    transactionSignatures: string[];
}

interface AccountToMigrate {
    pubkey: PublicKey;
    accountType: 'market' | 'reserve' | 'obligation' | 'multisig' | 'timelock' | 'governance';
    currentVersion: number;
}

class AuraLendMigration {
    private connection: Connection;
    private wallet: anchor.Wallet;
    private program: Program<AuraLend>;
    private config: MigrationConfig;

    constructor(config: MigrationConfig) {
        this.config = config;
        
        // Initialize connection
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
     * Discover all accounts that need migration
     */
    async discoverAccountsToMigrate(): Promise<AccountToMigrate[]> {
        console.log("🔍 Discovering accounts that need migration...");
        
        const accountsToMigrate: AccountToMigrate[] = [];

        try {
            // Get all program accounts
            const programAccounts = await this.connection.getProgramAccounts(this.program.programId);
            console.log(`📊 Found ${programAccounts.length} total program accounts`);

            // Analyze each account
            for (const { pubkey, account } of programAccounts) {
                try {
                    // Try to determine account type and version
                    const accountInfo = await this.analyzeAccount(pubkey, account.data);
                    
                    if (accountInfo && accountInfo.currentVersion < this.config.toVersion) {
                        accountsToMigrate.push({
                            pubkey,
                            accountType: accountInfo.accountType,
                            currentVersion: accountInfo.currentVersion,
                        });
                        
                        console.log(`📋 Found ${accountInfo.accountType} account ${pubkey.toString()} (v${accountInfo.currentVersion})`);
                    }
                } catch (error) {
                    console.warn(`⚠️  Could not analyze account ${pubkey.toString()}:`, error);
                }
            }

            console.log(`🎯 Found ${accountsToMigrate.length} accounts needing migration`);
            return accountsToMigrate;

        } catch (error) {
            console.error("❌ Failed to discover accounts:", error);
            throw error;
        }
    }

    /**
     * Analyze an account to determine its type and version
     */
    private async analyzeAccount(pubkey: PublicKey, data: Buffer): Promise<{
        accountType: AccountToMigrate['accountType'];
        currentVersion: number;
    } | null> {
        // Skip if too small
        if (data.length < 16) return null;

        try {
            // Try each account type
            const accountTypes: Array<{
                name: AccountToMigrate['accountType'];
                decoder: any;
            }> = [
                { name: 'market', decoder: this.program.account.market },
                { name: 'reserve', decoder: this.program.account.reserve },
                { name: 'obligation', decoder: this.program.account.obligation },
                { name: 'multisig', decoder: this.program.account.multiSig },
                { name: 'timelock', decoder: this.program.account.timelockController },
                { name: 'governance', decoder: this.program.account.governanceRegistry },
            ];

            for (const { name, decoder } of accountTypes) {
                try {
                    const decoded = decoder.coder.accounts.decode(name, data);
                    return {
                        accountType: name,
                        currentVersion: decoded.version || 0,
                    };
                } catch {
                    // Continue to next type
                }
            }

            return null;
        } catch {
            return null;
        }
    }

    /**
     * Execute migration for all discovered accounts
     */
    async executeMigration(): Promise<MigrationResult> {
        console.log("🚀 Starting migration process...");
        
        const result: MigrationResult = {
            success: false,
            totalAccounts: 0,
            migratedAccounts: 0,
            failedAccounts: 0,
            errors: [],
            transactionSignatures: [],
        };

        try {
            // Discover accounts
            const accountsToMigrate = await this.discoverAccountsToMigrate();
            result.totalAccounts = accountsToMigrate.length;

            if (accountsToMigrate.length === 0) {
                console.log("✅ No accounts need migration");
                result.success = true;
                return result;
            }

            // Group accounts by type for efficient batch processing
            const accountsByType = this.groupAccountsByType(accountsToMigrate);

            // Migrate each type
            for (const [accountType, accounts] of Object.entries(accountsByType)) {
                console.log(`📦 Migrating ${accounts.length} ${accountType} accounts...`);
                
                const typeResult = await this.migrateAccountType(accountType as AccountToMigrate['accountType'], accounts);
                
                result.migratedAccounts += typeResult.migrated;
                result.failedAccounts += typeResult.failed;
                result.errors.push(...typeResult.errors);
                result.transactionSignatures.push(...typeResult.signatures);
            }

            result.success = result.failedAccounts === 0;
            
            console.log(`✅ Migration completed: ${result.migratedAccounts}/${result.totalAccounts} successful`);
            return result;

        } catch (error) {
            console.error("❌ Migration failed:", error);
            result.errors.push(error instanceof Error ? error.message : String(error));
            return result;
        }
    }

    /**
     * Group accounts by type for batch processing
     */
    private groupAccountsByType(accounts: AccountToMigrate[]): Record<string, AccountToMigrate[]> {
        return accounts.reduce((groups, account) => {
            if (!groups[account.accountType]) {
                groups[account.accountType] = [];
            }
            groups[account.accountType].push(account);
            return groups;
        }, {} as Record<string, AccountToMigrate[]>);
    }

    /**
     * Migrate accounts of a specific type
     */
    private async migrateAccountType(
        accountType: AccountToMigrate['accountType'],
        accounts: AccountToMigrate[]
    ): Promise<{
        migrated: number;
        failed: number;
        errors: string[];
        signatures: string[];
    }> {
        const result = { migrated: 0, failed: 0, errors: [], signatures: [] };
        
        // Process in batches
        const batches = this.chunkArray(accounts, this.config.batchSize);
        
        for (let i = 0; i < batches.length; i++) {
            const batch = batches[i];
            console.log(`📦 Processing batch ${i + 1}/${batches.length} (${batch.length} accounts)`);
            
            try {
                const batchResult = await this.migrateBatch(accountType, batch);
                result.migrated += batchResult.migrated;
                result.failed += batchResult.failed;
                result.errors.push(...batchResult.errors);
                result.signatures.push(...batchResult.signatures);
                
                // Small delay between batches to avoid rate limiting
                if (i < batches.length - 1) {
                    await this.sleep(1000);
                }
            } catch (error) {
                console.error(`❌ Batch ${i + 1} failed:`, error);
                result.failed += batch.length;
                result.errors.push(`Batch ${i + 1}: ${error instanceof Error ? error.message : String(error)}`);
            }
        }
        
        return result;
    }

    /**
     * Migrate a batch of accounts
     */
    private async migrateBatch(
        accountType: AccountToMigrate['accountType'],
        accounts: AccountToMigrate[]
    ): Promise<{
        migrated: number;
        failed: number;
        errors: string[];
        signatures: string[];
    }> {
        const result = { migrated: 0, failed: 0, errors: [], signatures: [] };

        if (this.config.dryRun) {
            console.log(`🏃 Dry run - would migrate ${accounts.length} ${accountType} accounts`);
            result.migrated = accounts.length;
            return result;
        }

        // Get market PDA for authority validation
        const [marketPda] = await PublicKey.findProgramAddress(
            [Buffer.from("market")],
            this.program.programId
        );

        // Migrate each account individually for now
        // TODO: Implement batch migration when available
        for (const account of accounts) {
            try {
                let signature: string;

                switch (accountType) {
                    case 'market':
                        signature = await this.program.methods
                            .migrateMarket()
                            .accounts({
                                market: account.pubkey,
                                authority: this.wallet.publicKey,
                            })
                            .rpc();
                        break;

                    case 'reserve':
                        signature = await this.program.methods
                            .migrateReserve()
                            .accounts({
                                market: marketPda,
                                reserve: account.pubkey,
                                authority: this.wallet.publicKey,
                            })
                            .rpc();
                        break;

                    case 'obligation':
                        signature = await this.program.methods
                            .migrateObligation()
                            .accounts({
                                market: marketPda,
                                obligation: account.pubkey,
                                authority: this.wallet.publicKey,
                            })
                            .rpc();
                        break;

                    case 'multisig':
                        signature = await this.program.methods
                            .migrateMultisig()
                            .accounts({
                                market: marketPda,
                                multisig: account.pubkey,
                                authority: this.wallet.publicKey,
                            })
                            .rpc();
                        break;

                    case 'timelock':
                        signature = await this.program.methods
                            .migrateTimelock()
                            .accounts({
                                market: marketPda,
                                timelock: account.pubkey,
                                authority: this.wallet.publicKey,
                            })
                            .rpc();
                        break;

                    case 'governance':
                        signature = await this.program.methods
                            .migrateGovernance()
                            .accounts({
                                market: marketPda,
                                governance: account.pubkey,
                                authority: this.wallet.publicKey,
                            })
                            .rpc();
                        break;

                    default:
                        throw new Error(`Unknown account type: ${accountType}`);
                }

                result.migrated++;
                result.signatures.push(signature);
                console.log(`✅ Migrated ${accountType} ${account.pubkey.toString()}`);

            } catch (error) {
                result.failed++;
                const errorMsg = `Failed to migrate ${accountType} ${account.pubkey.toString()}: ${error instanceof Error ? error.message : String(error)}`;
                result.errors.push(errorMsg);
                console.error(`❌ ${errorMsg}`);
            }
        }

        return result;
    }

    /**
     * Utility function to chunk array into batches
     */
    private chunkArray<T>(array: T[], size: number): T[][] {
        const chunks: T[][] = [];
        for (let i = 0; i < array.length; i += size) {
            chunks.push(array.slice(i, i + size));
        }
        return chunks;
    }

    /**
     * Utility function to sleep
     */
    private sleep(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    /**
     * Generate migration report
     */
    generateReport(result: MigrationResult): void {
        const report = {
            timestamp: new Date().toISOString(),
            network: this.config.network,
            programId: this.config.programId,
            fromVersion: this.config.fromVersion,
            toVersion: this.config.toVersion,
            config: this.config,
            result: result,
        };

        const reportPath = `scripts/upgrade/reports/migration_${this.config.network}_${Date.now()}.json`;
        const reportDir = path.dirname(reportPath);
        
        if (!fs.existsSync(reportDir)) {
            fs.mkdirSync(reportDir, { recursive: true });
        }

        fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
        console.log(`📊 Migration report saved to: ${reportPath}`);
    }
}

// CLI Interface
async function main() {
    const args = process.argv.slice(2);
    const command = args[0];

    if (!command) {
        console.log(`
Usage: ${process.argv[1]} <command> [options]

Commands:
  discover     Discover accounts that need migration
  migrate      Execute migration for all accounts
  status       Check migration status

Options:
  --config=<path>         Configuration file path
  --network=<network>     Target network (localnet/devnet/mainnet)
  --from-version=<n>      Source version number
  --to-version=<n>        Target version number
  --batch-size=<n>        Batch size for processing (default: 10)
  --dry-run              Simulate without executing transactions

Examples:
  npm run migrate:discover --network=devnet
  npm run migrate:execute --network=devnet --from-version=1 --to-version=2
        `);
        process.exit(1);
    }

    // Parse configuration
    const config: MigrationConfig = {
        network: (args.find(arg => arg.startsWith('--network='))?.replace('--network=', '') as any) || "localnet",
        programId: "AuRa1Lend1111111111111111111111111111111111",
        authority: "~/.config/solana/id.json",
        fromVersion: parseInt(args.find(arg => arg.startsWith('--from-version='))?.replace('--from-version=', '') || "0"),
        toVersion: parseInt(args.find(arg => arg.startsWith('--to-version='))?.replace('--to-version=', '') || "1"),
        batchSize: parseInt(args.find(arg => arg.startsWith('--batch-size='))?.replace('--batch-size=', '') || "10"),
        dryRun: args.includes('--dry-run'),
    };

    const migration = new AuraLendMigration(config);

    switch (command) {
        case 'discover':
            console.log("🔍 Discovering accounts to migrate...");
            const accounts = await migration.discoverAccountsToMigrate();
            console.log(`📊 Found ${accounts.length} accounts needing migration`);
            break;

        case 'migrate':
            console.log("🚀 Starting migration...");
            const result = await migration.executeMigration();
            migration.generateReport(result);
            process.exit(result.success ? 0 : 1);

        default:
            console.error(`Unknown command: ${command}`);
            process.exit(1);
    }
}

if (require.main === module) {
    main().catch(error => {
        console.error("Migration failed:", error);
        process.exit(1);
    });
}