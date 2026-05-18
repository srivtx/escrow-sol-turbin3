use anchor_lang::prelude::*;

#[account]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive_amount: u64,
    pub escrow_bump: u8,
    pub vault_bump: u8,
}
