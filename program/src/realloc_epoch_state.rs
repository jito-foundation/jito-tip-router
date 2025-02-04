use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer, config::Config, epoch_state::EpochState, utils::get_new_size,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Reallocates the epoch state account to its full size.
/// This is needed due to Solana's account size limits during initialization.
pub fn process_realloc_epoch_state(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, config, ncn, account_payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Config::load(program_id, config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    let (epoch_state_pda, epoch_state_bump, _) =
        EpochState::find_program_address(program_id, ncn.key, epoch);

    if epoch_state_pda != *epoch_state.key {
        msg!("Ballot box account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    if epoch_state.data_len() < EpochState::SIZE {
        let new_size = get_new_size(epoch_state.data_len(), EpochState::SIZE)?;
        msg!(
            "Reallocating epoch state from {} bytes to {} bytes",
            epoch_state.data_len(),
            new_size
        );
        AccountPayer::pay_and_realloc(program_id, ncn.key, account_payer, epoch_state, new_size)?;
    }

    let should_initialize = epoch_state.data_len() >= EpochState::SIZE
        && epoch_state.try_borrow_data()?[0] != EpochState::DISCRIMINATOR;

    if should_initialize {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        epoch_state_data[0] = EpochState::DISCRIMINATOR;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.initialize(ncn.key, epoch, epoch_state_bump, Clock::get()?.slot);

        epoch_state_account.update_realloc_epoch_state();
    }

    Ok(())
}
