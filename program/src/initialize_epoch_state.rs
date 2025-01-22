use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer, config::Config, constants::MAX_REALLOC_BYTES,
    epoch_marker::EpochMarker, epoch_state::EpochState,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

pub fn process_initialize_epoch_state(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_marker, epoch_state, config, ncn, account_payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Check epoch cannot be in the future
    if epoch > Clock::get()?.epoch {
        return Err(ProgramError::InvalidArgument);
    }

    // Verify accounts
    load_system_account(epoch_state, true)?;
    load_system_program(system_program)?;

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Config::load(program_id, ncn.key, config, false)?;
    AccountPayer::load(program_id, ncn.key, account_payer, true)?;
    EpochMarker::check_dne(program_id, ncn.key, epoch, epoch_marker)?;

    let config_data = config.try_borrow_data()?;
    let config_account = Config::try_from_slice_unchecked(&config_data)?;
    if config_account.starting_valid_epoch() > epoch {
        msg!("This epoch is before the starting_valid_epoch");
        return Err(ProgramError::InvalidArgument);
    }

    let (epoch_state_pda, epoch_state_bump, mut epoch_state_seeds) =
        EpochState::find_program_address(program_id, ncn.key, epoch);
    epoch_state_seeds.push(vec![epoch_state_bump]);

    if epoch_state_pda != *epoch_state.key {
        return Err(ProgramError::InvalidSeeds);
    }

    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        epoch_state,
        system_program,
        program_id,
        MAX_REALLOC_BYTES as usize,
        &epoch_state_seeds,
    )?;

    Ok(())
}
