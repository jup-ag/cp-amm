use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    state::{ ModifyLiquidityResult, Pool, Position}, token::{calculate_transfer_fee_included_amount, transfer_from_user}, u128x128_math::Rounding, PoolError
};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddLiquidityParameters {
  /// delta liquidity
  pub liquidity_delta: u128,
  /// maximum token a amount
  pub token_a_amount_threshold: u64,
  /// maximum token b amount
  pub token_b_amount_threshold: u64,
}

#[event_cpi]
#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut, has_one = token_a_vault, has_one = token_b_vault, has_one = token_a_mint, has_one = token_b_mint)]
    pub pool: AccountLoader<'info, Pool>,

    #[account(
      mut, 
      has_one = pool,
      has_one = owner,
    )]
    pub position: AccountLoader<'info, Position>,

    pub owner: Signer<'info>,

    /// The user token a account 
    #[account(mut)]
    pub token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The user token b account
    #[account(mut)]
    pub token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The vault token account for input token
    #[account(mut, token::token_program = token_a_program, token::mint = token_a_mint)]
    pub token_a_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The vault token account for output token
    #[account(mut, token::token_program = token_b_program, token::mint = token_b_mint)]
    pub token_b_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token a program
    pub token_a_program: Interface<'info, TokenInterface>,

    /// Token b program
    pub token_b_program: Interface<'info, TokenInterface>,

    /// The mint of token a
    pub token_a_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The mint of token b
    pub token_b_mint: Box<InterfaceAccount<'info, Mint>>,
}

pub fn handle_add_liquidity(ctx: Context<AddLiquidity>, params: AddLiquidityParameters) -> Result<()> {
    let AddLiquidityParameters { liquidity_delta, token_a_amount_threshold, token_b_amount_threshold } = params;
    require!(params.liquidity_delta > 0, PoolError::InvalidParameters);

    let mut pool = ctx.accounts.pool.load_mut()?;
    let mut position = ctx.accounts.position.load_mut()?;
    let ModifyLiquidityResult{amount_a, amount_b} = pool.get_amounts_for_modify_liquidity(liquidity_delta, Rounding::Up)?;

    require!(amount_a > 0 || amount_b > 0, PoolError::AmountIsZero);

    pool.apply_add_liquidity(&mut position,  liquidity_delta)?;

    let total_amount_a = calculate_transfer_fee_included_amount(&ctx.accounts.token_a_mint, amount_a)?.amount;
    let total_amount_b = calculate_transfer_fee_included_amount(&ctx.accounts.token_b_mint, amount_b)?.amount;

    require!(total_amount_a <= token_a_amount_threshold, PoolError::ExceededSlippage);
    require!(total_amount_b <= token_b_amount_threshold, PoolError::ExceededSlippage);

    transfer_from_user(
        &ctx.accounts.owner,
        &ctx.accounts.token_a_mint,
        &ctx.accounts.token_a_account,
        &ctx.accounts.token_a_vault,
        &ctx.accounts.token_a_program,
        total_amount_a,
    )?;

    transfer_from_user(
        &ctx.accounts.owner,
        &ctx.accounts.token_b_mint,
        &ctx.accounts.token_b_account,
        &ctx.accounts.token_b_vault,
        &ctx.accounts.token_b_program,
        total_amount_b,
    )?;

    // TODO emit event

    Ok(())
}
