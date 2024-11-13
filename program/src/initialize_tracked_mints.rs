use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_system_account, load_system_program},
};
use jito_tip_router_core::{ncn_config::NcnConfig, tracked_mints::TrackedMints};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_tracked_mints(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [ncn_config, tracked_mints, ncn_account, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify accounts
    load_system_account(tracked_mints, true)?;
    load_system_program(system_program)?;

    NcnConfig::load(program_id, ncn_account.key, ncn_config, false)?;

    let (tracked_mints_pda, tracked_mints_bump, mut tracked_mints_seeds) =
        TrackedMints::find_program_address(program_id, ncn_account.key);
    tracked_mints_seeds.push(vec![tracked_mints_bump]);

    if tracked_mints_pda != *tracked_mints.key {
        return Err(ProgramError::InvalidSeeds);
    }

    create_account(
        payer,
        tracked_mints,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(std::mem::size_of::<TrackedMints>() as u64)
            .unwrap(),
        &tracked_mints_seeds,
    )?;

    let mut tracked_mints_data = tracked_mints.try_borrow_mut_data()?;
    tracked_mints_data[0] = TrackedMints::DISCRIMINATOR;
    let tracked_mints_account =
        TrackedMints::try_from_slice_unchecked_mut(&mut tracked_mints_data)?;
    *tracked_mints_account = TrackedMints::new(*ncn_account.key, tracked_mints_bump);

    Ok(())
}
