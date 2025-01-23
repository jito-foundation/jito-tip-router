use jito_bytemuck::{types::PodU64, AccountDeserialize};
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    config::Config,
    constants::{
        MAX_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE, MAX_EPOCHS_BEFORE_STALL,
        MAX_VALID_SLOTS_AFTER_CONSENSUS, MIN_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE,
        MIN_EPOCHS_BEFORE_STALL, MIN_VALID_SLOTS_AFTER_CONSENSUS,
    },
    error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process_admin_set_parameters(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    starting_valid_epoch: Option<u64>,
    epochs_before_stall: Option<u64>,
    epochs_after_consensus_before_close: Option<u64>,
    valid_slots_after_consensus: Option<u64>,
) -> ProgramResult {
    let [config, ncn_account, ncn_admin] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_signer(ncn_admin, true)?;

    // Load and verify accounts
    Config::load(program_id, ncn_account.key, config, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn_account, false)?;

    {
        let ncn_data = ncn_account.data.borrow();
        let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
        if ncn.admin != *ncn_admin.key {
            return Err(TipRouterError::IncorrectNcnAdmin.into());
        }
    }

    let mut config_data = config.try_borrow_mut_data()?;
    let config = Config::try_from_slice_unchecked_mut(&mut config_data)?;

    if config.ncn != *ncn_account.key {
        return Err(TipRouterError::IncorrectNcn.into());
    }

    if let Some(epoch) = starting_valid_epoch {
        msg!("Updated valid_starting_epoch to {}", epoch);
        config.starting_valid_epoch = PodU64::from(epoch);
    }

    if let Some(epochs) = epochs_before_stall {
        if !(MIN_EPOCHS_BEFORE_STALL..=MAX_EPOCHS_BEFORE_STALL).contains(&epochs) {
            return Err(TipRouterError::InvalidEpochsBeforeStall.into());
        }
        msg!("Updated epochs_before_stall to {}", epochs);
        config.epochs_before_stall = PodU64::from(epochs);
    }

    if let Some(epochs) = epochs_after_consensus_before_close {
        if !(MIN_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE..=MAX_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE)
            .contains(&epochs)
        {
            return Err(TipRouterError::InvalidEpochsBeforeClose.into());
        }
        msg!("Updated epochs_after_consensus_before_close to {}", epochs);
        config.epochs_after_consensus_before_close = PodU64::from(epochs);
    }

    if let Some(slots) = valid_slots_after_consensus {
        if !(MIN_VALID_SLOTS_AFTER_CONSENSUS..=MAX_VALID_SLOTS_AFTER_CONSENSUS).contains(&slots) {
            return Err(TipRouterError::InvalidSlotsAfterConsensus.into());
        }
        msg!("Updated valid_slots_after_consensus to {}", slots);
        config.valid_slots_after_consensus = PodU64::from(slots);
    }

    Ok(())
}
