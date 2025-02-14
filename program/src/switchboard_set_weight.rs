use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    constants::{SWITCHBOARD_MAX_STALE_SLOTS, WEIGHT_PRECISION},
    epoch_state::EpochState,
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
pub fn process_switchboard_set_weight(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    st_mint: &Pubkey,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn, weight_table, switchboard_feed] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    WeightTable::load(program_id, weight_table, ncn.key, epoch, true)?;

    let (registered_switchboard_feed, no_feed_weight) = {
        let weight_table_data = weight_table.data.borrow();
        let weight_table_account = WeightTable::try_from_slice_unchecked(&weight_table_data)?;

        let weight_entry = weight_table_account.get_weight_entry(st_mint)?;

        (
            *weight_entry.st_mint_entry().switchboard_feed(),
            weight_entry.st_mint_entry().no_feed_weight(),
        )
    };

    let weight: u128 = if registered_switchboard_feed.eq(&Pubkey::default()) {
        if no_feed_weight == 0 {
            msg!("No feed weight is not set");
            return Err(TipRouterError::NoFeedWeightNotSet.into());
        }

        msg!("No Feed Weight: {}", no_feed_weight);
        no_feed_weight
    } else {
        if registered_switchboard_feed.ne(switchboard_feed.key) {
            msg!("Switchboard feed is not registered");
            return Err(TipRouterError::SwitchboardNotRegistered.into());
        }

        let feed = PullFeedAccountData::parse(switchboard_feed.data.borrow())
            .map_err(|_| TipRouterError::BadSwitchboardFeed)?;

        let clock = Clock::get()?;
        let price: Decimal = feed
            .value(&clock)
            .map_err(|_| TipRouterError::BadSwitchboardValue)?;

        let current_slot = clock.slot;
        let stale_slot = {
            feed.result
                .slot
                .checked_add(SWITCHBOARD_MAX_STALE_SLOTS)
                .ok_or(TipRouterError::ArithmeticOverflow)?
        };

        if current_slot > stale_slot {
            msg!("Stale feed");
            return Err(TipRouterError::StaleSwitchboardFeed.into());
        }

        msg!("Oracle Price: {}", price);
        let weight = price
            .checked_mul(WEIGHT_PRECISION.into())
            .ok_or(TipRouterError::ArithmeticOverflow)?
            .round();

        msg!("Oracle Weight: {}", weight);
        weight.to_u128().ok_or(TipRouterError::CastToU128Error)?
    };

    let mut weight_table_data = weight_table.try_borrow_mut_data()?;
    let weight_table_account = WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

    weight_table_account.check_table_initialized()?;

    if weight_table_account.finalized() {
        msg!("Weight table is finalized");
        return Err(ProgramError::InvalidAccountData);
    }

    weight_table_account.set_weight(st_mint, weight, Clock::get()?.slot)?;

    // Update Epoch State
    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_set_weight(
            weight_table_account.weight_count() as u64,
            weight_table_account.st_mint_count() as u64,
        );
    }

    Ok(())
}
