use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_tip_router_core::{ballot_box::BallotBox, ncn_config::NcnConfig};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_ballot_box(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [ncn_config, ballot_box, ncn_account, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify accounts
    load_system_account(ballot_box, true)?;
    load_system_program(system_program)?;

    load_signer(payer, false)?;

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
        8_u64
            .checked_add(std::mem::size_of::<BallotBox>() as u64)
            .unwrap(),
        &ballot_box_seeds,
    )?;

    let mut ballot_box_data = ballot_box.try_borrow_mut_data()?;
    ballot_box_data[0] = BallotBox::DISCRIMINATOR;
    let ballot_box_account = BallotBox::try_from_slice_unchecked_mut(&mut ballot_box_data)?;

    ballot_box_account.initialize(*ncn_account.key, epoch, ballot_box_bump, Clock::get()?.slot);

    Ok(())
}
