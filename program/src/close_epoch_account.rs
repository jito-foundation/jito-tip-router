use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer,
    ballot_box::BallotBox,
    config::Config as NcnConfig,
    epoch_marker::EpochMarker,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    epoch_schedule::EpochSchedule, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};

/// Crank Closes all accounts associated with an epoch
#[allow(clippy::cognitive_complexity)]
pub fn process_close_epoch_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_marker, epoch_state, config, ncn, account_to_close, account_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    EpochState::load(program_id, epoch_state, ncn.key, epoch, false)?;
    NcnConfig::load(program_id, config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, false)?;
    EpochMarker::check_dne(program_id, epoch_marker, ncn.key, epoch)?;

    let closing_epoch_state = account_to_close.key.eq(epoch_state.key);

    // Empty Account Check
    if account_to_close.data_is_empty() {
        msg!("Account already closed");
        return Err(TipRouterError::CannotCloseAccountAlreadyClosed.into());
    }

    {
        let config_data = config.try_borrow_data()?;
        let config_account = NcnConfig::try_from_slice_unchecked(&config_data)?;

        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;

        // Epoch Check - epochs after consensus is reached
        {
            let epochs_after_consensus_before_close =
                config_account.epochs_after_consensus_before_close();

            let current_slot = Clock::get()?.slot;
            let epoch_schedule = EpochSchedule::get()?;

            let can_close_epoch_accounts = epoch_state_account.can_close_epoch_accounts(
                &epoch_schedule,
                epochs_after_consensus_before_close,
                current_slot,
            )?;

            if !can_close_epoch_accounts {
                msg!("Not enough epochs have passed since consensus reached");
                return Err(TipRouterError::CannotCloseAccountNotEnoughEpochs.into());
            }

            epoch_state_account.set_is_closing();
        }

        // Account Check
        {
            let discriminator = {
                if closing_epoch_state {
                    // Cannot borrow the data again
                    EpochState::DISCRIMINATOR
                } else {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    account_to_close_data[0]
                }
            };

            match discriminator {
                EpochState::DISCRIMINATOR => {
                    EpochState::load_to_close(epoch_state_account, ncn.key, epoch)?;
                    epoch_state_account.close_epoch_state();
                }
                WeightTable::DISCRIMINATOR => {
                    WeightTable::load_to_close(program_id, account_to_close, ncn.key, epoch)?;
                    epoch_state_account.close_weight_table();
                }
                EpochSnapshot::DISCRIMINATOR => {
                    EpochSnapshot::load_to_close(program_id, account_to_close, ncn.key, epoch)?;
                    epoch_state_account.close_epoch_snapshot();
                }
                OperatorSnapshot::DISCRIMINATOR => {
                    OperatorSnapshot::load_to_close(program_id, account_to_close, ncn.key, epoch)?;
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let account_to_close_struct =
                        OperatorSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                    let ncn_operator_index = account_to_close_struct.ncn_operator_index() as usize;
                    epoch_state_account.close_operator_snapshot(ncn_operator_index);
                }
                BallotBox::DISCRIMINATOR => {
                    BallotBox::load_to_close(program_id, account_to_close, ncn.key, epoch)?;
                    epoch_state_account.close_ballot_box();
                }
                _ => {
                    return Err(TipRouterError::InvalidAccountToCloseDiscriminator.into());
                }
            }
        }
    }

    if closing_epoch_state {
        let (epoch_marker_pda, epoch_marker_bump, mut epoch_marker_seeds) =
            EpochMarker::find_program_address(program_id, ncn.key, epoch);
        epoch_marker_seeds.push(vec![epoch_marker_bump]);

        if epoch_marker_pda != *epoch_marker.key {
            return Err(ProgramError::InvalidSeeds);
        }

        AccountPayer::pay_and_create_account(
            program_id,
            ncn.key,
            account_payer,
            epoch_marker,
            system_program,
            program_id,
            EpochMarker::SIZE,
            &epoch_marker_seeds,
        )?;

        let mut epoch_marker_data = epoch_marker.try_borrow_mut_data()?;
        epoch_marker_data[0] = EpochMarker::DISCRIMINATOR;
        let epoch_marker = EpochMarker::try_from_slice_unchecked_mut(&mut epoch_marker_data)?;

        let slot_closed = Clock::get()?.slot;
        *epoch_marker = EpochMarker::new(ncn.key, epoch, slot_closed);
    }

    AccountPayer::close_account(program_id, account_payer, account_to_close)
}
