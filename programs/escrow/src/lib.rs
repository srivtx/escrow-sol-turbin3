pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("7U1UpLXwYPm9HSWF4hmEJ1rhYrrgSV896CB9dYXrDQBi");

#[program]
pub mod escrow {
    use super::*;

    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit_amount: u64,
        receive_amount: u64,
    ) -> Result<()> {
        make::handler(ctx, seed, deposit_amount, receive_amount)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        refund::handler(ctx)
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        take::handler(ctx)
    }
}
