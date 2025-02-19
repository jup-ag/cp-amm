use crate::activation_handler::ActivationType;
use crate::alpha_vault::alpha_vault;
use crate::constants::seeds::{CUSTOMIZABLE_POOL_PREFIX, POSITION_PREFIX};
use crate::constants::{MAX_SQRT_PRICE, MIN_SQRT_PRICE};
use crate::curve::get_initialize_amounts;
use crate::params::pool_fees::PoolFees;
use crate::state::fee::PoolFeesStruct;
use crate::state::CollectFeeMode;
use crate::token::{
    calculate_transfer_fee_included_amount, get_token_program_flags, is_supported_mint,
    is_token_badge_initialized, transfer_from_user,
};
use crate::PoolError;
use crate::{
    constants::seeds::{POOL_AUTHORITY_PREFIX, TOKEN_VAULT_PREFIX},
    state::{Pool, Position},
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use super::initialize_pool_utils::{get_first_key, get_second_key};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeCustomizablePoolParameters {
    /// pool fees
    pub pool_fees: PoolFees,
    /// sqrt min price
    pub sqrt_min_price: u128,
    /// sqrt max price
    pub sqrt_max_price: u128,
    /// activation type
    pub activation_type: ActivationType,
    /// collect fee mode
    pub collect_fee_mode: CollectFeeMode,
    /// has alpha vault
    pub has_alpha_vault: bool,
    /// initialize liquidity
    pub liquidity: u128,
    /// The init price of the pool as a sqrt(token_b/token_a) Q64.64 value
    pub sqrt_price: u128,
    /// activation point
    pub activation_point: Option<u64>,
}

impl InitializeCustomizablePoolParameters {
    pub fn validate(&self) -> Result<()> {
        require!(
            self.sqrt_min_price == MIN_SQRT_PRICE && self.sqrt_max_price == MAX_SQRT_PRICE,
            PoolError::InvalidPriceRange
        );
        require!(
            self.sqrt_price >= self.sqrt_min_price && self.sqrt_price <= self.sqrt_max_price,
            PoolError::InvalidPriceRange
        );

        Ok(())
    }
}

#[event_cpi]
#[derive(Accounts)]
pub struct InitializeCustomizablePoolCtx<'info> {
    /// CHECK: Pool creator
    pub creator: UncheckedAccount<'info>,

    /// Address paying to create the pool. Can be anyone
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: pool authority
    #[account(
        seeds = [
            POOL_AUTHORITY_PREFIX.as_ref(),
        ],
        bump,
    )]
    pub pool_authority: UncheckedAccount<'info>,

    /// Initialize an account to store the pool state
    #[account(
        init,
        seeds = [
            CUSTOMIZABLE_POOL_PREFIX.as_ref(),
            get_first_key(token_a_mint.key(), token_b_mint.key()).as_ref(),
            get_second_key(token_a_mint.key(), token_b_mint.key()).as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + Pool::INIT_SPACE
    )]
    pub pool: AccountLoader<'info, Pool>,

    #[account(
        init,
        seeds = [
            POSITION_PREFIX.as_ref(),
            pool.key().as_ref(),
            creator.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + Position::INIT_SPACE
    )]
    pub position: AccountLoader<'info, Position>,

    /// Token a mint
    #[account(
        constraint = token_a_mint.key() != token_b_mint.key(),
        mint::token_program = token_a_program,
    )]
    pub token_a_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token b mint
    #[account(
        mint::token_program = token_b_program,
    )]
    pub token_b_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token a vault for the pool
    #[account(
        init,
        seeds = [
            TOKEN_VAULT_PREFIX.as_ref(),
            token_a_mint.key().as_ref(),
            pool.key().as_ref(),
        ],
        token::mint = token_a_mint,
        token::authority = pool_authority,
        token::token_program = token_a_program,
        payer = payer,
        bump,
    )]
    pub token_a_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token b vault for the pool
    #[account(
        init,
        seeds = [
            TOKEN_VAULT_PREFIX.as_ref(),
            token_b_mint.key().as_ref(),
            pool.key().as_ref(),
        ],
        token::mint = token_b_mint,
        token::authority = pool_authority,
        token::token_program = token_b_program,
        payer = payer,
        bump,
    )]
    pub token_b_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// payer token a account
    #[account(mut)]
    pub payer_token_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// creator token b account
    #[account(mut)]
    pub payer_token_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Program to create mint account and mint tokens
    pub token_a_program: Interface<'info, TokenInterface>,
    /// Program to create mint account and mint tokens
    pub token_b_program: Interface<'info, TokenInterface>,
    // Sysvar for program account
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_customizable_pool<'c: 'info, 'info>(
    ctx: Context<'_, '_, 'c, 'info, InitializeCustomizablePoolCtx<'info>>,
    params: InitializeCustomizablePoolParameters,
) -> Result<()> {
    params.validate()?;
    if !is_supported_mint(&ctx.accounts.token_a_mint)? {
        require!(
            is_token_badge_initialized(
                ctx.accounts.token_a_mint.key(),
                ctx.remaining_accounts
                    .get(0)
                    .ok_or(PoolError::InvalidTokenBadge)?,
            )?,
            PoolError::InvalidTokenBadge
        )
    }

    if !is_supported_mint(&ctx.accounts.token_b_mint)? {
        require!(
            is_token_badge_initialized(
                ctx.accounts.token_b_mint.key(),
                ctx.remaining_accounts
                    .get(1)
                    .ok_or(PoolError::InvalidTokenBadge)?,
            )?,
            PoolError::InvalidTokenBadge
        )
    }

    // TODO validate params
    let InitializeCustomizablePoolParameters {
        pool_fees,
        liquidity,
        sqrt_price,
        activation_point,
        sqrt_min_price,
        sqrt_max_price,
        activation_type,
        collect_fee_mode,
        has_alpha_vault,
    } = params;

    let (token_a_amount, token_b_amount) =
        get_initialize_amounts(sqrt_min_price, sqrt_max_price, sqrt_price, liquidity)?;
    let mut pool = ctx.accounts.pool.load_init()?;

    pool.initialize(
        PoolFeesStruct::from_pool_fees(&pool_fees),
        ctx.accounts.token_a_mint.key(),
        ctx.accounts.token_b_mint.key(),
        ctx.accounts.token_a_vault.key(),
        ctx.accounts.token_b_mint.key(),
        get_whitelisted_alpha_vault(
            ctx.accounts.payer.key(),
            ctx.accounts.pool.key(),
            has_alpha_vault,
        ),
        ctx.accounts.creator.key(),
        sqrt_min_price,
        sqrt_max_price,
        sqrt_price,
        activation_point.unwrap_or_default(),
        activation_type.into(),
        get_token_program_flags(&ctx.accounts.token_a_mint).into(),
        get_token_program_flags(&ctx.accounts.token_b_mint).into(),
        token_a_amount,
        token_b_amount,
        liquidity,
        collect_fee_mode.into(),
    );

    // init position
    let mut position = ctx.accounts.position.load_init()?;
    position.initialize(
        ctx.accounts.pool.key(),
        ctx.accounts.creator.key(),
        Pubkey::default(), // TODO may add more params
        Pubkey::default(), // TODO may add more params
        liquidity,
        0, // TODO check this
        0,
    );

    // transfer token
    let total_amount_a =
        calculate_transfer_fee_included_amount(&ctx.accounts.token_a_mint, token_a_amount)?.amount;
    let total_amount_b =
        calculate_transfer_fee_included_amount(&ctx.accounts.token_b_mint, token_b_amount)?.amount;
    transfer_from_user(
        &ctx.accounts.payer,
        &ctx.accounts.token_a_mint,
        &ctx.accounts.payer_token_a,
        &ctx.accounts.token_a_vault,
        &ctx.accounts.token_a_program,
        total_amount_a,
    )?;
    transfer_from_user(
        &ctx.accounts.payer,
        &ctx.accounts.token_b_mint,
        &ctx.accounts.payer_token_b,
        &ctx.accounts.token_b_vault,
        &ctx.accounts.token_b_program,
        total_amount_b,
    )?;

    // TODO emit events

    Ok(())
}

pub fn get_whitelisted_alpha_vault(payer: Pubkey, pool: Pubkey, has_alpha_vault: bool) -> Pubkey {
    if has_alpha_vault {
        alpha_vault::derive_vault_pubkey(payer, pool)
    } else {
        Pubkey::default()
    }
}
