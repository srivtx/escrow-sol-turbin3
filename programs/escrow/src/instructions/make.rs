use anchor_lang::prelude::*;

use crate::constants::ESCROW_SEED;
use crate::state::Escrow;

#[derive(Accounts)]
#[instruction(seed: u64, deposit_amount: u64, receive_amount: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    /// CHECK: Just storing pubkey for identification
    pub mint_a: UncheckedAccount<'info>,
    /// CHECK: Just storing pubkey for identification
    pub mint_b: UncheckedAccount<'info>,

    #[account(
        init,
        payer = maker,
        space = 8 + 8 + 32 + 32 + 32 + 8 + 1 + 1,
        seeds = [ESCROW_SEED.as_bytes(), seed.to_le_bytes().as_ref(), maker.key().as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        init,
        payer = maker,
        space = 8,
        seeds = [ESCROW_SEED.as_bytes(), b"vault", seed.to_le_bytes().as_ref(), maker.key().as_ref()],
        bump
    )]
    /// CHECK: Vault PDA that holds lamports
    pub vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Make>,
    seed: u64,
    deposit_amount: u64,
    receive_amount: u64,
) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;
    escrow.seed = seed;
    escrow.maker = ctx.accounts.maker.key();
    escrow.mint_a = ctx.accounts.mint_a.key();
    escrow.mint_b = ctx.accounts.mint_b.key();
    escrow.receive_amount = receive_amount;
    escrow.escrow_bump = ctx.bumps.escrow;
    escrow.vault_bump = ctx.bumps.vault;

    let cpi_accounts = anchor_lang::system_program::Transfer {
        from: ctx.accounts.maker.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.key(),
        cpi_accounts,
    );
    anchor_lang::system_program::transfer(cpi_ctx, deposit_amount)?;

    msg!("Escrow created: seed={}, deposit={}, receive={}", seed, deposit_amount, receive_amount);
    Ok(())
}
