use crate::activation_handler::ActivationHandler;
use crate::error::PoolError;
use crate::safe_math::SafeMath;
use crate::state::{Pool, Position, Vesting};
use crate::EvtLockPosition;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct VestingParameters {
    // Set to None to start vesting immediately
    pub cliff_point: Option<u64>,
    pub period_frequency: u64,
    pub cliff_unlock_liquidity: u128,
    pub liquidity_per_period: u128,
    pub number_of_period: u16,
    pub index: u16,
}

impl VestingParameters {
    pub fn get_cliff_point(&self, current_point: u64) -> Result<u64> {
        Ok(self.cliff_point.unwrap_or(current_point))
    }

    pub fn get_total_lock_amount(&self) -> Result<u128> {
        let total_amount = self.cliff_unlock_liquidity.safe_add(
            self.liquidity_per_period
                .safe_mul(self.number_of_period.into())?,
        )?;

        Ok(total_amount)
    }

    pub fn validate(&self, current_point: u64, max_vesting_duration: u64) -> Result<()> {
        let cliff_point = self.get_cliff_point(current_point)?;

        require!(cliff_point >= current_point, PoolError::InvalidVestingInfo);
        if self.number_of_period > 0 {
            require!(
                self.period_frequency > 0 && self.liquidity_per_period > 0,
                PoolError::InvalidVestingInfo
            );
        }

        let vesting_duration = cliff_point.safe_sub(current_point)?.safe_add(
            self.period_frequency
                .safe_mul(self.number_of_period.into())?,
        )?;

        require!(
            vesting_duration <= max_vesting_duration,
            PoolError::InvalidVestingInfo
        );

        require!(
            self.get_total_lock_amount()? > 0,
            PoolError::InvalidVestingInfo
        );

        Ok(())
    }
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(params: VestingParameters)]
pub struct LockPositionCtx<'info> {
    pub pool: AccountLoader<'info, Pool>,

    #[account(
        init,
        payer = payer,
        space = 8 + Vesting::INIT_SPACE
    )]
    pub vesting: AccountLoader<'info, Vesting>,

    #[account(mut, has_one = pool, has_one = owner)]
    pub position: AccountLoader<'info, Position>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_lock_position(
    ctx: Context<LockPositionCtx>,
    params: VestingParameters,
) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    let (current_point, max_vesting_duration) =
        ActivationHandler::get_current_point_and_max_vesting_duration(pool.activation_type)?;

    params.validate(current_point, max_vesting_duration)?;

    let total_lock_liquidity = params.get_total_lock_amount()?;
    let cliff_point = params.get_cliff_point(current_point)?;

    let VestingParameters {
        period_frequency,
        cliff_unlock_liquidity,
        liquidity_per_period,
        number_of_period,
        ..
    } = params;

    let mut vesting = ctx.accounts.vesting.load_init()?;
    vesting.initialize(
        ctx.accounts.position.key(),
        cliff_point,
        period_frequency,
        cliff_unlock_liquidity,
        liquidity_per_period,
        number_of_period,
    );

    let mut position = ctx.accounts.position.load_mut()?;
    position.lock(total_lock_liquidity)?;

    emit_cpi!(EvtLockPosition {
        position: ctx.accounts.position.key(),
        pool: ctx.accounts.pool.key(),
        owner: ctx.accounts.owner.key(),
        vesting: ctx.accounts.vesting.key(),
        cliff_point,
        period_frequency,
        cliff_unlock_liquidity,
        liquidity_per_period,
        number_of_period,
    });

    Ok(())
}
