//! Event module includes information about events of the program
use anchor_lang::prelude::*;

use crate::{
    params::fee_parameters::PoolFeeParameters, state::SwapResult, AddLiquidityParameters,
    RemoveLiquidityParameters, SwapParameters,
};

/// Close config
#[derive(Debug)]
#[event]
pub struct EvtCloseConfig {
    /// Config pubkey
    pub config: Pubkey,
    /// admin pk
    pub admin: Pubkey,
}

/// Create config
#[derive(Debug)]
#[event]
pub struct EvtCreateConfig {
    pub pool_fees: PoolFeeParameters,
    pub vault_config_key: Pubkey,
    pub pool_creator_authority: Pubkey,
    pub activation_type: u8,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub collect_fee_mode: u8,
    pub index: u64,
    pub config: Pubkey,
}

/// Create token badge
#[derive(Debug)]
#[event]
pub struct EvtCreateTokenBadge {
    pub token_mint: Pubkey,
}

/// Create claim fee operator
#[derive(Debug)]
#[event]
pub struct EvtCreateClaimFeeOperator {
    pub operator: Pubkey,
}

/// Close claim fee operator
#[derive(Debug)]
#[event]
pub struct EvtCloseClaimFeeOperator {
    pub claim_fee_operator: Pubkey,
    pub operator: Pubkey,
}

#[derive(Debug)]
#[event]
pub struct EvtInitializePool {
    pub pool: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub creator: Pubkey,
    pub payer: Pubkey,
    pub alpha_vault: Pubkey,
    pub pool_fees: PoolFeeParameters,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub activation_type: u8,
    pub collect_fee_mode: u8,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub token_a_flag: u8,
    pub token_b_flag: u8,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub total_amount_a: u64,
    pub total_amount_b: u64,
    pub pool_type: u8,
}

#[derive(Debug)]
#[event]
pub struct EvtAddLiquidity {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub params: AddLiquidityParameters,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub total_amount_a: u64,
    pub total_amount_b: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtClaimPositionFee {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub fee_a_claimed: u64,
    pub fee_b_claimed: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtCreatePosition {
    pub pool: Pubkey,
    pub owner: Pubkey,
    pub position: Pubkey,
    pub position_nft_mint: Pubkey,
}

#[derive(Debug)]
#[event]
pub struct EvtRemoveLiquidity {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub params: RemoveLiquidityParameters,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtSwap {
    pub pool: Pubkey,
    pub trade_direction: u8,
    pub has_referral: bool,
    pub params: SwapParameters,
    pub swap_result: SwapResult,
    pub actual_amount_in: u64,
    pub current_timestamp: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtLockPosition {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub owner: Pubkey,
    pub vesting: Pubkey,
    pub cliff_point: u64,
    pub period_frequency: u64,
    pub cliff_unlock_liquidity: u128,
    pub liquidity_per_period: u128,
    pub number_of_period: u16,
}
#[derive(Debug)]
#[event]
pub struct EvtPermanentLockPosition {
    pub pool: Pubkey,
    pub position: Pubkey,
    pub lock_liquidity_amount: u128,
    pub total_permanent_locked_liquidity: u128,
}

#[derive(Debug)]
#[event]
pub struct EvtClaimProtocolFee {
    pub pool: Pubkey,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtClaimPartnerFee {
    pub pool: Pubkey,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtSetPoolStatus {
    pub pool: Pubkey,
    pub status: u8,
}

// Initialize reward
#[derive(Debug)]
#[event]
pub struct EvtInitializeReward {
    // Liquidity pool
    pub pool: Pubkey,
    // Mint address of the farm reward
    pub reward_mint: Pubkey,
    // Address of the funder
    pub funder: Pubkey,
    // Index of the farm reward being initialized
    pub reward_index: u8,
    // Duration of the farm reward in seconds
    pub reward_duration: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtFundReward {
    // Liquidity pool
    pub pool: Pubkey,
    // Address of the funder
    pub funder: Pubkey,
    // Mint reward
    pub mint_reward: Pubkey,
    // Index of the farm reward being funded
    pub reward_index: u8,
    // Amount of farm reward funded
    pub amount: u64,
    // Amount excluded transfer fee
    pub transfer_fee_excluded_amount_in: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtClaimReward {
    // Liquidity pool
    pub pool: Pubkey,
    // Position address
    pub position: Pubkey,
    // Owner of the position
    pub owner: Pubkey,
    // Mint reward
    pub mint_reward: Pubkey,
    // Index of the farm reward the owner is claiming
    pub reward_index: u8,
    // Total amount of reward claimed
    pub total_reward: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtUpdateRewardDuration {
    // Liquidity pool
    pub pool: Pubkey,
    // Index of the farm reward being updated
    pub reward_index: u8,
    // Old farm reward duration
    pub old_reward_duration: u64,
    // New farm reward duration
    pub new_reward_duration: u64,
}

#[derive(Debug)]
#[event]
pub struct EvtUpdateRewardFunder {
    // Liquidity pool
    pub pool: Pubkey,
    // Index of the farm reward being updated
    pub reward_index: u8,
    // Address of the old farm reward funder
    pub old_funder: Pubkey,
    // Address of the new farm reward funder
    pub new_funder: Pubkey,
}

#[derive(Debug)]
#[event]
pub struct EvtWithdrawIneligibleReward {
    // Liquidity pool
    pub pool: Pubkey,
    // Reward mint
    pub reward_mint: Pubkey,
    // Amount of ineligible reward withdrawn
    pub amount: u64,
}
