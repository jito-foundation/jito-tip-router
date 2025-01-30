use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer, ballot_box::BallotBox, config::Config as NcnConfig,
    constants::MAX_REALLOC_BYTES, epoch_marker::EpochMarker, epoch_state::EpochState,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process_initialize_ballot_box(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_marker, epoch_state, ncn_config, ballot_box, ncn, account_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify accounts
    load_system_account(ballot_box, true)?;
    load_system_program(system_program)?;

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    AccountPayer::load(program_id, ncn.key, account_payer, true)?;
    EpochMarker::check_dne(program_id, ncn.key, epoch, epoch_marker)?;

    let (ballot_box_pda, ballot_box_bump, mut ballot_box_seeds) =
        BallotBox::find_program_address(program_id, ncn.key, epoch);
    ballot_box_seeds.push(vec![ballot_box_bump]);

    if ballot_box_pda != *ballot_box.key {
        return Err(ProgramError::InvalidSeeds);
    }

    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        ballot_box,
        system_program,
        program_id,
        MAX_REALLOC_BYTES as usize,
        &ballot_box_seeds,
    )?;

    Ok(())
}
