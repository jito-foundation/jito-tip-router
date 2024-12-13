use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    constants::MAX_REALLOC_BYTES, tracked_mints::TrackedMints, weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Initializes a Weight Table
/// Can be backfilled for previous epochs
pub fn process_initialize_weight_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [restaking_config, tracked_mints, ncn, weight_table, payer, restaking_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    TrackedMints::load(program_id, ncn.key, tracked_mints, false)?;
    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;

    load_system_account(weight_table, true)?;
    load_system_program(system_program)?;
    load_signer(payer, true)?;

    let vault_count = {
        let ncn_data = ncn.data.borrow();
        let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
        ncn.vault_count()
    };

    let tracked_mint_count = {
        let tracked_mints_data = tracked_mints.data.borrow();
        let tracked_mints = TrackedMints::try_from_slice_unchecked(&tracked_mints_data)?;
        tracked_mints.mint_count()
    };

    if vault_count != tracked_mint_count {
        msg!("Vault count does not match supported mint count");
        return Err(ProgramError::InvalidAccountData);
    }

    let (weight_table_pubkey, weight_table_bump, mut weight_table_seeds) =
        WeightTable::find_program_address(program_id, ncn.key, epoch);
    weight_table_seeds.push(vec![weight_table_bump]);

    if weight_table_pubkey.ne(weight_table.key) {
        msg!("Incorrect weight table PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Weight Table {} for NCN: {} at epoch: {}",
        weight_table.key,
        ncn.key,
        epoch
    );
    create_account(
        payer,
        weight_table,
        system_program,
        program_id,
        &Rent::get()?,
        MAX_REALLOC_BYTES,
        &weight_table_seeds,
    )?;

    Ok(())
}
