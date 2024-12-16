use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    constants::{JTO_MINT, JTO_USD_FEED, MAX_STALE_SLOTS, WEIGHT_PRECISION},
    error::TipRouterError,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};
use switchboard_on_demand::{
    prelude::rust_decimal::{prelude::ToPrimitive, Decimal},
    PullFeedAccountData,
};

/// Updates weight table
pub fn process_set_jto_weight(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [ncn, weight_table, jto_usd_feed] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;

    WeightTable::load(program_id, weight_table, ncn, epoch, true)?;

    if jto_usd_feed.key.ne(&JTO_USD_FEED) {
        msg!("Incorrect jto usd feed");
        return Err(ProgramError::InvalidAccountData);
    }

    let weight: u128 = {
        let feed = PullFeedAccountData::parse(jto_usd_feed.data.borrow())
            .map_err(|_| TipRouterError::BadSwitchboardFeed)?;
        let price: Decimal = feed.value().ok_or(TipRouterError::BadSwitchboardValue)?;

        let slot_delta = {
            let current_slot = Clock::get()?.slot;
            current_slot
                .checked_sub(feed.result.slot)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?
        };

        if slot_delta > MAX_STALE_SLOTS {
            msg!("Stale feed");
            return Err(ProgramError::InvalidAccountData);
        }

        msg!("Oracle Price: {}", price);
        let weight = price
            .checked_mul(WEIGHT_PRECISION.into())
            .ok_or(TipRouterError::ArithmeticOverflow)?
            .round();

        msg!("Weight: {}", weight);

        weight.to_u128().ok_or(TipRouterError::CastToU128Error)?
    };

    let mut weight_table_data = weight_table.try_borrow_mut_data()?;
    let weight_table_account = WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

    weight_table_account.check_initialized()?;
    if weight_table_account.finalized() {
        msg!("Weight table is finalized");
        return Err(ProgramError::InvalidAccountData);
    }

    weight_table_account.set_weight(&JTO_MINT, weight, Clock::get()?.slot)?;

    Ok(())
}
