use anchor_lang::prelude::*;

use crate::constants::ESCROW_SEED;
use crate::state::Escrow;

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [ESCROW_SEED.as_bytes(), escrow.seed.to_le_bytes().as_ref(), maker.key().as_ref()],
        bump = escrow.escrow_bump,
        close = maker,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [ESCROW_SEED.as_bytes(), b"vault", escrow.seed.to_le_bytes().as_ref(), maker.key().as_ref()],
        bump = escrow.vault_bump,
    )]
    /// CHECK: Vault PDA that holds lamports
    pub vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Take>) -> Result<()> {
    // Transfer receive_amount from taker to maker
    let cpi_accounts = anchor_lang::system_program::Transfer {
        from: ctx.accounts.taker.to_account_info(),
        to: ctx.accounts.maker.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.key(),
        cpi_accounts,
    );
    anchor_lang::system_program::transfer(cpi_ctx, ctx.accounts.escrow.receive_amount)?;

    // Transfer vault lamports to taker
    let vault_lamports = ctx.accounts.vault.to_account_info().lamports();

    **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= vault_lamports;
    **ctx.accounts.taker.to_account_info().try_borrow_mut_lamports()? += vault_lamports;

    msg!("Escrow taken");
    Ok(())
}
