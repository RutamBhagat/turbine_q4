use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked, MintTo, mint_to},
};

use crate::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    maker: Signer<'info>,
    mint_x: Box<InterfaceAccount<'info, Mint>>,
    mint_y: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init,
        payer = maker,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"amm", mint_x.key().as_ref(), mint_y.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
    )]
    config: Box<Account<'info, Config>>,
    #[account(
        init_if_needed,
        payer = maker,
        mint::authority = config,
        mint::decimals = 6,
        mint::token_program = token_program,
        seeds = [b"amm", config.key().as_ref()],
        bump,
    )]
    mint_lp: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_x,
        associated_token::authority = config

    )]
    vault_x: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_y,
        associated_token::authority = config
        
    )]
    vault_y: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = maker

    )]
    maker_ata_x: Box<InterfaceAccount<'info, TokenAccount>>,             
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = maker
        
    )]
    maker_ata_y: Box<InterfaceAccount<'info, TokenAccount>>,
        #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_lp,
        associated_token::authority = maker
        
    )]
    maker_ata_lp: Box<InterfaceAccount<'info, TokenAccount>>,
    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn save_config(&mut self, seed: u64, fee: u16, bump: u8, lp_bump: u8) -> Result<()> {
        self.config.set_inner(Config {
            seed,
            fee,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            lp_bump,
            bump,
        });
        Ok(())
    }

    pub fn deposit(&mut self, amount: u64, is_x: bool) -> Result<()> {
        let (from, to, mint, decimals) = match is_x {
            true => (self.maker_ata_x.to_account_info(), self.vault_x.to_account_info(), self.mint_x.to_account_info(), self.mint_x.decimals),
            false => (self.maker_ata_y.to_account_info(), self.vault_y.to_account_info(), self.mint_y.to_account_info(), self.mint_y.decimals),
        };
        let cpi_accounts = TransferChecked{ from, mint, to, authority: self.maker.to_account_info() };

        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, amount, decimals)
    }

    pub fn mint_lp_tokens(&mut self, amount_x: u64, amount_y: u64) -> Result<()> {
        let amount = amount_x.checked_mul(amount_y).ok_or(ProgramError::ArithmeticOverflow)?;
        
        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.maker_ata_lp.to_account_info(),
            authority: self.config.to_account_info(),
        };
        
        let cpi_program = self.token_program.to_account_info();
        
        // Create a longer-lived value for the seed bytes
        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"amm",
            self.mint_x.to_account_info().key.as_ref(),
            self.mint_y.to_account_info().key.as_ref(),
            seed_bytes.as_ref(),
            &[self.config.bump]
        ]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        mint_to(cpi_ctx, amount)
    }
}