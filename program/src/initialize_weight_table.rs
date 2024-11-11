use std::mem::size_of;

use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    loaders::load_ncn_epoch, ncn_config::NcnConfig, weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Initializes a Weight Table
/// Can be backfilled for previous epochs
pub fn process_initialize_weight_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, weight_table, payer, restaking_program_id, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program_id.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    Config::load(restaking_program_id.key, restaking_config, false)?;
    Ncn::load(restaking_program_id.key, ncn, false)?;

    load_system_account(weight_table, true)?;
    load_system_program(system_program)?;
    load_signer(payer, true)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    let _todo_pubkeys = {
        let ncn_config_data = ncn_config.data.borrow();
        let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        ncn_config.bump
    };

    let (weight_table_pubkey, weight_table_bump, mut weight_table_seeds) =
        WeightTable::find_program_address(program_id, ncn.key, ncn_epoch);
    weight_table_seeds.push(vec![weight_table_bump]);

    if weight_table_pubkey.ne(weight_table.key) {
        msg!("Incorrect weight table PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Weight Table {} for NCN: {} at epoch: {}",
        weight_table.key,
        ncn.key,
        ncn_epoch
    );
    create_account(
        payer,
        weight_table,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64.checked_add(size_of::<WeightTable>() as u64).unwrap(),
        &weight_table_seeds,
    )?;

    let mut weight_table_data = weight_table.try_borrow_mut_data()?;
    weight_table_data[0] = WeightTable::DISCRIMINATOR;
    let weight_table_account = WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

    *weight_table_account = WeightTable::new(*ncn.key, ncn_epoch, current_slot, weight_table_bump);

    //TODO pass in st_mint list from config
    weight_table_account.initalize_weight_table(&[])?;

    Ok(())
}
