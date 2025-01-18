use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{config::Config, constants::MAX_REALLOC_BYTES, epoch_state::EpochState};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_epoch_state(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, config, ncn_account, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check epoch cannot be in the future
    if epoch > Clock::get()?.epoch {
        return Err(ProgramError::InvalidArgument);
    }

    // Verify accounts
    load_system_account(epoch_state, true)?;
    load_system_program(system_program)?;

    load_signer(payer, false)?;

    Ncn::load(&jito_restaking_program::id(), ncn_account, false)?;
    Config::load(program_id, ncn_account.key, config, false)?;

    let (epoch_state_pda, epoch_state_bump, mut epoch_state_seeds) =
        EpochState::find_program_address(program_id, ncn_account.key, epoch);
    epoch_state_seeds.push(vec![epoch_state_bump]);

    if epoch_state_pda != *epoch_state.key {
        return Err(ProgramError::InvalidSeeds);
    }

    create_account(
        payer,
        epoch_state,
        system_program,
        program_id,
        &Rent::get()?,
        MAX_REALLOC_BYTES,
        &epoch_state_seeds,
    )?;

    Ok(())
}
