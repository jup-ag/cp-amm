use anchor_lang::prelude::*;

use crate::{
    constants::{
        fee::{FEE_DENOMINATOR, MAX_FEE_NUMERATOR},
        BASIS_POINT_MAX,
    },
    params::pool_fees::{DynamicFee, PoolFees},
    safe_math::SafeMath,
    utils_math::safe_mul_div_cast_u64,
};

/// Encodes all results of swapping
#[derive(Debug, PartialEq)]
pub struct FeeOnAmountResult {
    pub amount: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub partner_fee: u64,
    pub referral_fee: u64,
}

#[zero_copy]
/// Information regarding fee charges
/// trading_fee = amount * trade_fee_numerator / denominator
/// protocol_fee = trading_fee * protocol_fee_percentage / 100
/// referral_fee = protocol_fee * referral_percentage / 100
/// partner_fee = (protocol_fee - referral_fee) * partner_fee_percentage / denominator
#[derive(Debug, InitSpace, Default)]
pub struct PoolFeesStruct {
    /// Trade fees are extra token amounts that are held inside the token
    /// accounts during a trade, making the value of liquidity tokens rise.
    /// Trade fee numerator
    pub trade_fee_numerator: u64,

    /// Protocol trading fees are extra token amounts that are held inside the token
    /// accounts during a trade, with the equivalent in pool tokens minted to
    /// the protocol of the program.
    /// Protocol trade fee numerator
    pub protocol_fee_percent: u8,
    /// partner fee
    pub partner_fee_percent: u8,
    /// referral fee
    pub referral_fee_percent: u8,
    /// padding
    pub padding_0: [u8; 5],

    /// dynamic fee
    pub dynamic_fee: DynamicFeeStruct,

    /// padding
    pub padding_1: [u64; 2],
}

impl PoolFeesStruct {
    pub fn from_pool_fees(pool_fees: &PoolFees) -> Self {
        let &PoolFees {
            trade_fee_numerator,
            protocol_fee_percent,
            partner_fee_percent,
            referral_fee_percent,
            dynamic_fee,
        } = pool_fees;
        if let Some(DynamicFee {
            bin_step,
            bin_step_u128,
            filter_period,
            decay_period,
            reduction_factor,
            max_volatility_accumulator,
            variable_fee_control,
        }) = dynamic_fee
        {
            Self {
                trade_fee_numerator,
                protocol_fee_percent,
                partner_fee_percent,
                referral_fee_percent,
                dynamic_fee: DynamicFeeStruct {
                    initialized: 1,
                    bin_step,
                    filter_period,
                    decay_period,
                    reduction_factor,
                    bin_step_u128,
                    max_volatility_accumulator,
                    variable_fee_control,
                    ..Default::default()
                },
                ..Default::default()
            }
        } else {
            Self {
                trade_fee_numerator,
                protocol_fee_percent,
                partner_fee_percent,
                referral_fee_percent,
                ..Default::default()
            }
        }
    }

    // in numerator
    pub fn get_total_trading_fee(&self) -> Result<u128> {
        let total_fee_numerator = self
            .dynamic_fee
            .get_variable_fee()?
            .safe_add(self.trade_fee_numerator.into())?;
        Ok(total_fee_numerator)
    }

    pub fn get_fee_on_amount(&self, amount: u64, is_referral: bool) -> Result<FeeOnAmountResult> {
        let trade_fee_numerator = self.get_total_trading_fee()?;
        let trade_fee_numerator = if trade_fee_numerator > MAX_FEE_NUMERATOR.into() {
            MAX_FEE_NUMERATOR
        } else {
            trade_fee_numerator.try_into().unwrap()
        };
        let lp_fee: u64 = safe_mul_div_cast_u64(amount, trade_fee_numerator, FEE_DENOMINATOR)?;
        // update amount
        let amount = amount.safe_sub(lp_fee)?;

        let protocol_fee = safe_mul_div_cast_u64(lp_fee, self.protocol_fee_percent.into(), 100)?;
        // update lp fee
        let lp_fee = lp_fee.safe_sub(protocol_fee)?;

        let referral_fee = if is_referral {
            safe_mul_div_cast_u64(protocol_fee, self.referral_fee_percent.into(), 100)?
        } else {
            0
        };

        let protocol_fee_after_referral_fee = protocol_fee.safe_sub(referral_fee)?;
        let partner_fee = safe_mul_div_cast_u64(
            protocol_fee_after_referral_fee,
            self.partner_fee_percent.into(),
            100,
        )?;

        let protocol_fee = protocol_fee_after_referral_fee.safe_sub(partner_fee)?;

        Ok(FeeOnAmountResult {
            amount,
            lp_fee,
            protocol_fee,
            partner_fee,
            referral_fee,
        })
    }
}

#[zero_copy]
#[derive(Debug, InitSpace, Default)]
pub struct DynamicFeeStruct {
    pub initialized: u8, // 0, ignore for dynamic fee
    pub padding: [u8; 7],
    pub max_volatility_accumulator: u32,
    pub variable_fee_control: u32,
    pub bin_step: u16,
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub last_update_timestamp: u64,
    pub bin_step_u128: u128,
    pub sqrt_price_reference: u128, // reference sqrt price
    pub volatility_accumulator: u128,
    pub volatility_reference: u128, // decayed volatility accumulator
}

impl DynamicFeeStruct {
    // we approximate (1+bin_step)^bin_id = 1 + bin_step * bin_id
    pub fn get_detal_bin_id(
        bin_step_u128: u128,
        sqrt_price_a: u128,
        sqrt_price_b: u128,
    ) -> Result<u128> {
        let delta_id = if sqrt_price_a > sqrt_price_b {
            sqrt_price_a
                .safe_sub(sqrt_price_b)?
                .safe_div(bin_step_u128)?
        } else {
            sqrt_price_b
                .safe_sub(sqrt_price_b)?
                .safe_div(bin_step_u128)?
        };
        Ok(delta_id.safe_mul(2)?) // mul 2 because we are using sqrt price
    }
    pub fn update_volatility_accumulator(&mut self, sqrt_price: u128) -> Result<()> {
        let delta_price =
            Self::get_detal_bin_id(self.bin_step_u128, sqrt_price, self.sqrt_price_reference)?;

        let volatility_accumulator = self
            .volatility_reference
            .safe_add(delta_price.safe_mul(BASIS_POINT_MAX.into())?)?;

        self.volatility_accumulator = std::cmp::min(
            volatility_accumulator,
            self.max_volatility_accumulator.into(),
        );
        Ok(())
    }

    pub fn update_references(
        &mut self,
        sqrt_price_current: u128,
        current_timestamp: u64,
    ) -> Result<()> {
        let elapsed = current_timestamp.safe_sub(self.last_update_timestamp)?;
        // Not high frequency trade
        if elapsed >= self.filter_period as u64 {
            // Update sqrt of last transaction
            self.sqrt_price_reference = sqrt_price_current;
            // filter period < t < decay_period. Decay time window.
            if elapsed < self.decay_period as u64 {
                let volatility_reference = self
                    .volatility_accumulator
                    .safe_mul(self.reduction_factor.into())?
                    .safe_div(BASIS_POINT_MAX.into())?;

                self.volatility_reference = volatility_reference;
            }
            // Out of decay time window
            else {
                self.volatility_reference = 0;
            }
        }
        Ok(())
    }

    pub fn is_dynamic_fee_enable(&self) -> bool {
        self.initialized != 0
    }

    pub fn get_variable_fee(&self) -> Result<u128> {
        if self.is_dynamic_fee_enable() {
            let square_vfa_bin = self
                .volatility_accumulator
                .safe_mul(self.bin_step.into())?
                .checked_pow(2)
                .unwrap();
            // Variable fee control, volatility accumulator, bin step are in basis point unit (10_000)
            // This is 1e20. Which > 1e9. Scale down it to 1e9 unit and ceiling the remaining.
            let v_fee = square_vfa_bin.safe_mul(self.variable_fee_control.into())?;

            let scaled_v_fee = v_fee.safe_add(99_999_999_999)?.safe_div(100_000_000_000)?;

            Ok(scaled_v_fee)
        } else {
            Ok(0)
        }
    }
}
