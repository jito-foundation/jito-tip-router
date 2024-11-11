use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::config::Config;
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError};

use crate::error::TipRouterError;

pub fn load_ncn_epoch(
    restaking_config: &AccountInfo,
    current_slot: u64,
    first_slot_of_ncn_epoch: Option<u64>,
) -> Result<(u64, u64), ProgramError> {
    let ncn_epoch_length = {
        let config_data = restaking_config.data.borrow();
        let config = Config::try_from_slice_unchecked(&config_data)?;
        config.epoch_length()
    };

    let current_ncn_epoch = current_slot
        .checked_div(ncn_epoch_length)
        .ok_or(TipRouterError::DenominatorIsZero)?;

    let ncn_epoch_slot = first_slot_of_ncn_epoch.unwrap_or(current_slot);
    let ncn_epoch = ncn_epoch_slot
        .checked_div(ncn_epoch_length)
        .ok_or(TipRouterError::DenominatorIsZero)?;

    if ncn_epoch > current_ncn_epoch {
        msg!("Epoch snapshots can only be initialized for current or past epochs");
        return Err(TipRouterError::CannotCreateFutureWeightTables.into());
    }

    Ok((ncn_epoch, ncn_epoch_length))
}
