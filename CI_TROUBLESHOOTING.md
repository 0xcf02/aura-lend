# CI/CD Pipeline Troubleshooting Guide

## Overview
This guide helps diagnose and fix common CI/CD pipeline issues for the Aura Lend project.

## Common Issues and Solutions

### 1. Build Failures

#### Anchor Build Issues
**Symptoms**: `Error: String is the wrong size` or `build-sbf` command not found

**Solutions**:
- Verify Program IDs in `Anchor.toml` are valid base58 strings (44 characters)
- Ensure Solana CLI is properly installed in the environment
- Check Anchor version compatibility between `Anchor.toml`, `package.json`, and workflows

#### Rust Compilation Errors
**Symptoms**: Multiple compilation errors in `.rs` files

**Solutions**:
- Run `cargo check` locally to identify issues
- Ensure all imports are available and modules are properly declared
- Check for lifetime and borrowing issues in the code

### 2. Dependencies Issues

#### Version Mismatches
**Symptoms**: Warnings about version mismatches between CLI and dependencies

**Solutions**:
- Update `Anchor.toml` with correct `anchor_version`
- Update `package.json` with matching `@coral-xyz/anchor` version
- Update workflows with correct `ANCHOR_VERSION` environment variable

#### NPM Audit Failures
**Symptoms**: Security vulnerabilities or outdated dependencies

**Solutions**:
- Run `npm audit fix` to automatically fix issues
- Update dependencies manually if automatic fix fails
- Use `npm audit --audit-level=moderate` to ignore low-severity issues

### 3. Test Failures

#### TypeScript Type Errors
**Symptoms**: Cannot find module '../target/types/aura_lend'

**Solutions**:
- Ensure Rust program builds successfully first (generates IDL)
- Check `tsconfig.json` includes all necessary paths
- Verify `rootDir` and `include` settings in `tsconfig.json`

#### ESLint Errors
**Symptoms**: Configuration errors or lint rule violations

**Solutions**:
- Update `.eslintrc.js` configuration
- Run `npm run lint:fix` to automatically fix issues
- Use overrides for test files if needed

### 4. Environment Setup

#### Missing Secrets
**Symptoms**: Deployment failures due to missing keys

**Required Secrets**:
- `DEVNET_PRIVATE_KEY`: Solana keypair for devnet deployment
- `MAINNET_PRIVATE_KEY`: Solana keypair for mainnet deployment
- `SLACK_WEBHOOK_URL`: For notifications (optional)

#### Tool Installation Issues
**Symptoms**: Command not found errors

**Solutions**:
- Verify correct versions in workflow environment variables
- Add fallback installation commands
- Use `continue-on-error: true` for non-critical steps

## Manual Testing Commands

```bash
# Check Rust compilation
cargo check

# Run TypeScript type checking
npm run type-check

# Run linting
npm run lint

# Test build process
anchor build

# Run tests individually
npm run test:security
npm run test:governance
npm run test:performance
```

## Monitoring

### Health Check Workflow
The project includes a weekly health check workflow that:
- Verifies dependencies
- Checks project structure
- Generates health reports

### Manual Health Check
Run the health check workflow manually via GitHub Actions interface when troubleshooting.

## Getting Help

1. Check workflow logs in GitHub Actions tab
2. Review this troubleshooting guide
3. Run commands locally to reproduce issues
4. Check individual component status (Rust, TypeScript, npm, etc.)

## Recent Fixes Applied

- ✅ Fixed missing validation functions (`validate_signer`, `validate_authority`)
- ✅ Created RBAC module for access control
- ✅ Resolved Rust lifetime and ownership issues
- ✅ Updated ESLint configuration
- ✅ Synchronized package.json scripts with workflows
- ✅ Updated Anchor versions across all configurations
- ✅ Added fallback handling for build failures
- ✅ Improved cross-platform compatibility