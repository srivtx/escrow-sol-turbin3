use anchor_lang::prelude::*;

use crate::constants::ESCROW_SEED;
use crate::state::Escrow;

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        seeds = [ESCROW_SEED.as_bytes(), escrow.seed.to_le_bytes().as_ref(), maker.key().as_ref()],
        bump = escrow.escrow_bump,
        has_one = maker,
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

pub fn handler(ctx: Context<Refund>) -> Result<()> {
    let vault_lamports = ctx.accounts.vault.to_account_info().lamports();

    **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= vault_lamports;
    **ctx.accounts.maker.to_account_info().try_borrow_mut_lamports()? += vault_lamports;

    msg!("Escrow refunded");
    Ok(())
}
