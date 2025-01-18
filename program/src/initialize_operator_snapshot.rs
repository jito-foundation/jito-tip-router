use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{ncn::Ncn, ncn_operator_state::NcnOperatorState, operator::Operator};
use jito_tip_router_core::{
    config::Config,
    constants::MAX_REALLOC_BYTES,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Initializes an Operator Snapshot
pub fn process_initialize_operator_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, config, ncn, operator, ncn_operator_state, epoch_snapshot, operator_snapshot, payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    Config::load(program_id, ncn.key, config, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    NcnOperatorState::load(
        &jito_restaking_program::id(),
        ncn_operator_state,
        ncn,
        operator,
        false,
    )?;
    EpochSnapshot::load(program_id, ncn.key, epoch, epoch_snapshot, false)?;

    load_system_account(operator_snapshot, true)?;
    load_system_program(system_program)?;
    load_signer(payer, true)?;

    let (operator_snapshot_pubkey, operator_snapshot_bump, mut operator_snapshot_seeds) =
        OperatorSnapshot::find_program_address(program_id, operator.key, ncn.key, epoch);
    operator_snapshot_seeds.push(vec![operator_snapshot_bump]);

    if operator_snapshot_pubkey.ne(operator_snapshot.key) {
        msg!("Operator snapshot account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    // Cannot create Operator snapshot if the operator index is greater than the operator count
    {
        let epoch_snapshot_data = epoch_snapshot.data.borrow();
        let epoch_snapshot = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

        let ncn_operator_state_data = ncn_operator_state.data.borrow();
        let ncn_operator_state =
            NcnOperatorState::try_from_slice_unchecked(&ncn_operator_state_data)?;

        let operator_count = epoch_snapshot.operator_count();
        let operator_index = ncn_operator_state.index();

        if operator_index >= operator_count {
            msg!("Operator index is out of bounds");
            return Err(TipRouterError::OperatorIsNotInSnapshot.into());
        }
    }

    msg!(
        "Initializing Operator snapshot {} for NCN: {} at epoch: {}",
        epoch_snapshot.key,
        ncn.key,
        epoch
    );

    create_account(
        payer,
        operator_snapshot,
        system_program,
        program_id,
        &Rent::get()?,
        MAX_REALLOC_BYTES,
        &operator_snapshot_seeds,
    )?;

    Ok(())
}
