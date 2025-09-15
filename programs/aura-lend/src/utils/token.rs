use crate::error::LendingError;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

/// Token utility functions for SPL token operations
pub struct TokenUtils;

impl TokenUtils {
    /// Transfer tokens from one account to another
    pub fn transfer_tokens<'info>(
        token_program: &Program<'info, Token>,
        from: &Account<'info, TokenAccount>,
        to: &Account<'info, TokenAccount>,
        authority: &AccountInfo<'info>,
        authority_signer_seeds: &[&[&[u8]]],
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = Transfer {
            from: from.to_account_info(),
            to: to.to_account_info(),
            authority: authority.clone(),
        };

        let cpi_context = if authority_signer_seeds.is_empty() {
            CpiContext::new(token_program.to_account_info(), cpi_accounts)
        } else {
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                cpi_accounts,
                authority_signer_seeds,
            )
        };

        token::transfer(cpi_context, amount)
    }

    /// Mint new tokens to an account
    pub fn mint_tokens<'info>(
        token_program: &Program<'info, Token>,
        mint: &Account<'info, Mint>,
        to: &Account<'info, TokenAccount>,
        mint_authority: &AccountInfo<'info>,
        authority_signer_seeds: &[&[&[u8]]],
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = MintTo {
            mint: mint.to_account_info(),
            to: to.to_account_info(),
            authority: mint_authority.clone(),
        };

        let cpi_context = if authority_signer_seeds.is_empty() {
            CpiContext::new(token_program.to_account_info(), cpi_accounts)
        } else {
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                cpi_accounts,
                authority_signer_seeds,
            )
        };

        token::mint_to(cpi_context, amount)
    }

    /// Burn tokens from an account
    pub fn burn_tokens<'info>(
        token_program: &Program<'info, Token>,
        mint: &Account<'info, Mint>,
        from: &Account<'info, TokenAccount>,
        authority: &AccountInfo<'info>,
        authority_signer_seeds: &[&[&[u8]]],
        amount: u64,
    ) -> Result<()> {
        let cpi_accounts = Burn {
            mint: mint.to_account_info(),
            from: from.to_account_info(),
            authority: authority.clone(),
        };

        let cpi_context = if authority_signer_seeds.is_empty() {
            CpiContext::new(token_program.to_account_info(), cpi_accounts)
        } else {
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                cpi_accounts,
                authority_signer_seeds,
            )
        };

        token::burn(cpi_context, amount)
    }

    /// Get the amount of tokens accounting for decimals
    pub fn get_token_amount(ui_amount: f64, decimals: u8) -> u64 {
        (ui_amount * 10_f64.powi(decimals as i32)) as u64
    }

    /// Get the UI amount from token amount accounting for decimals
    pub fn get_ui_amount(token_amount: u64, decimals: u8) -> f64 {
        token_amount as f64 / 10_f64.powi(decimals as i32)
    }

    /// Validate that token accounts have the expected mint
    pub fn validate_token_mint(
        token_account: &Account<TokenAccount>,
        expected_mint: &Pubkey,
    ) -> Result<()> {
        if token_account.mint != *expected_mint {
            return Err(LendingError::TokenMintMismatch.into());
        }
        Ok(())
    }

    /// Validate that token account has the expected owner
    pub fn validate_token_owner(
        token_account: &Account<TokenAccount>,
        expected_owner: &Pubkey,
    ) -> Result<()> {
        if token_account.owner != *expected_owner {
            return Err(LendingError::TokenAccountOwnerMismatch.into());
        }
        Ok(())
    }

    /// Check if account has sufficient token balance
    pub fn validate_sufficient_balance(
        token_account: &Account<TokenAccount>,
        required_amount: u64,
    ) -> Result<()> {
        if token_account.amount < required_amount {
            return Err(LendingError::InsufficientTokenBalance.into());
        }
        Ok(())
    }

    /// Calculate proportional amount based on shares and total supply
    pub fn calculate_proportional_amount(
        shares: u64,
        total_shares: u64,
        total_amount: u64,
    ) -> Result<u64> {
        if total_shares == 0 || shares == 0 {
            return Ok(0);
        }

        let proportional_amount = (shares as u128)
            .checked_mul(total_amount as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(total_shares as u128)
            .ok_or(LendingError::DivisionByZero)?;

        if proportional_amount > u64::MAX as u128 {
            return Err(LendingError::MathOverflow.into());
        }

        Ok(proportional_amount as u64)
    }

    /// Calculate shares to mint based on deposit amount
    pub fn calculate_shares_to_mint(
        deposit_amount: u64,
        total_shares: u64,
        total_amount: u64,
    ) -> Result<u64> {
        if total_amount == 0 || total_shares == 0 {
            // First deposit - mint 1:1 shares
            return Ok(deposit_amount);
        }

        let shares_to_mint = (deposit_amount as u128)
            .checked_mul(total_shares as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(total_amount as u128)
            .ok_or(LendingError::DivisionByZero)?;

        if shares_to_mint > u64::MAX as u128 {
            return Err(LendingError::MathOverflow.into());
        }

        Ok(shares_to_mint as u64)
    }

    /// Calculate token amount to withdraw based on shares to burn
    pub fn calculate_withdraw_amount(
        shares_to_burn: u64,
        total_shares: u64,
        total_amount: u64,
    ) -> Result<u64> {
        if total_shares == 0 {
            return Ok(0);
        }

        let withdraw_amount = (shares_to_burn as u128)
            .checked_mul(total_amount as u128)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(total_shares as u128)
            .ok_or(LendingError::DivisionByZero)?;

        if withdraw_amount > u64::MAX as u128 {
            return Err(LendingError::MathOverflow.into());
        }

        Ok(withdraw_amount as u64)
    }

    /// Validate that the token program is the expected SPL Token program
    pub fn validate_token_program(token_program: &AccountInfo) -> Result<()> {
        if token_program.key() != spl_token::ID {
            return Err(LendingError::InvalidTokenProgram.into());
        }
        Ok(())
    }

    /// Create a PDA for associated token account
    pub fn get_associated_token_address(
        wallet_address: &Pubkey,
        token_mint_address: &Pubkey,
    ) -> Pubkey {
        spl_associated_token_account::get_associated_token_address(
            wallet_address,
            token_mint_address,
        )
    }

    /// Check if an account is an associated token account
    pub fn is_associated_token_account(
        token_account: &Pubkey,
        wallet_address: &Pubkey,
        token_mint_address: &Pubkey,
    ) -> bool {
        let expected_ata = Self::get_associated_token_address(wallet_address, token_mint_address);
        token_account == &expected_ata
    }
}
