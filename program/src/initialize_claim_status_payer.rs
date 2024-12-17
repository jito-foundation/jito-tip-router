use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_tip_router_core::claim_status_payer::ClaimStatusPayer;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_claim_status_payer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [claim_status_payer, payer, tip_distribution_program, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_account(claim_status_payer, true)?;
    load_system_program(system_program)?;
    load_signer(payer, false)?;

    let (payer_pda, payer_bump, mut payer_seeds) =
        ClaimStatusPayer::find_program_address(program_id, tip_distribution_program.key);
    payer_seeds.push(vec![payer_bump]);

    if payer_pda != *claim_status_payer.key {
        return Err(ProgramError::InvalidSeeds);
    }

    create_account(
        payer,
        claim_status_payer,
        system_program,
        program_id,
        &Rent::get()?,
        0,
        &payer_seeds,
    )?;

    // let mut payer_data = claim_status_payer.try_borrow_mut_data()?;
    // payer_data[0] = ClaimStatusPayer::DISCRIMINATOR;
    // let payer_account = ClaimStatusPayer::try_from_slice_unchecked_mut(&mut payer_data)?;
    // *payer_account = ClaimStatusPayer::new(payer_bump, *tip_distribution_program.key);

    Ok(())
}
