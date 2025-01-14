use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    loader::{load_signer, load_system_program},
    realloc,
};
use jito_tip_router_core::{
    ballot_box::BallotBox, config::Config as NcnConfig, epoch_state::EpochState,
    utils::get_new_size,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Reallocates the ballot box account to its full size.
/// This is needed due to Solana's account size limits during initialization.
pub fn process_realloc_ballot_box(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ballot_box, ncn, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    load_signer(payer, false)?;
    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;

    let (ballot_box_pda, ballot_box_bump, _) =
        BallotBox::find_program_address(program_id, ncn.key, epoch);

    if ballot_box_pda != *ballot_box.key {
        msg!("Ballot box account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    if ballot_box.data_len() < BallotBox::SIZE {
        let new_size = get_new_size(ballot_box.data_len(), BallotBox::SIZE)?;
        msg!(
            "Reallocating ballot box from {} bytes to {} bytes",
            ballot_box.data_len(),
            new_size
        );
        realloc(ballot_box, new_size, payer, &Rent::get()?)?;
    }

    let should_initialize = ballot_box.data_len() >= BallotBox::SIZE
        && ballot_box.try_borrow_data()?[0] != BallotBox::DISCRIMINATOR;

    if should_initialize {
        let mut ballot_box_data = ballot_box.try_borrow_mut_data()?;
        ballot_box_data[0] = BallotBox::DISCRIMINATOR;
        let ballot_box_account = BallotBox::try_from_slice_unchecked_mut(&mut ballot_box_data)?;
        ballot_box_account.initialize(ncn.key, epoch, ballot_box_bump, Clock::get()?.slot);

        // Update Epoch State
        {
            let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
            let epoch_state_account =
                EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
            epoch_state_account.update_realloc_ballot_box();
        }
    }

    Ok(())
}
