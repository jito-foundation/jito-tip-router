use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    ballot_box::BallotBox, config::Config as NcnConfig, constants::MAX_REALLOC_BYTES,
    epoch_state::EpochState,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_ballot_box(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ballot_box, ncn_account, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify accounts
    load_system_account(ballot_box, true)?;
    load_system_program(system_program)?;

    load_signer(payer, false)?;

    Ncn::load(&jito_restaking_program::id(), ncn_account, false)?;
    EpochState::load(program_id, ncn_account.key, epoch, epoch_state, false)?;
    NcnConfig::load(program_id, ncn_account.key, ncn_config, false)?;

    let (ballot_box_pda, ballot_box_bump, mut ballot_box_seeds) =
        BallotBox::find_program_address(program_id, ncn_account.key, epoch);
    ballot_box_seeds.push(vec![ballot_box_bump]);

    if ballot_box_pda != *ballot_box.key {
        return Err(ProgramError::InvalidSeeds);
    }

    create_account(
        payer,
        ballot_box,
        system_program,
        program_id,
        &Rent::get()?,
        MAX_REALLOC_BYTES,
        &ballot_box_seeds,
    )?;

    Ok(())
}
