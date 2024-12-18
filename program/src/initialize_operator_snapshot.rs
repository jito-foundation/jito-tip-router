use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{
    config::Config, ncn::Ncn, ncn_operator_state::NcnOperatorState, operator::Operator,
};
use jito_tip_router_core::{
    constants::MAX_REALLOC_BYTES,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    ncn_config::NcnConfig,
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
    let [ncn_config, restaking_config, ncn, operator, ncn_operator_state, epoch_snapshot, operator_snapshot, payer, restaking_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    Operator::load(restaking_program.key, operator, false)?;
    NcnOperatorState::load(
        restaking_program.key,
        ncn_operator_state,
        ncn,
        operator,
        false,
    )?;
    EpochSnapshot::load(program_id, ncn.key, epoch, epoch_snapshot, true)?;

    load_system_account(operator_snapshot, true)?;
    load_system_program(system_program)?;
    //TODO check that it is not writable
    load_signer(payer, false)?;

    let (operator_snapshot_pubkey, operator_snapshot_bump, mut operator_snapshot_seeds) =
        OperatorSnapshot::find_program_address(program_id, operator.key, ncn.key, epoch);
    operator_snapshot_seeds.push(vec![operator_snapshot_bump]);

    if operator_snapshot_pubkey.ne(operator_snapshot.key) {
        msg!("Operator snapshot account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
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
