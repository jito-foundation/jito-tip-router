use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer, constants::MAX_REALLOC_BYTES, epoch_marker::EpochMarker,
    epoch_state::EpochState, vault_registry::VaultRegistry, weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Initializes a Weight Table
/// Can be backfilled for previous epochs
pub fn process_initialize_weight_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_marker, epoch_state, vault_registry, ncn, weight_table, account_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    VaultRegistry::load(program_id, ncn.key, vault_registry, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    AccountPayer::load(program_id, ncn.key, account_payer, true)?;
    EpochMarker::check_dne(program_id, ncn.key, epoch, epoch_marker)?;

    load_system_account(weight_table, true)?;
    load_system_program(system_program)?;

    let vault_count = {
        let ncn_data = ncn.data.borrow();
        let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
        ncn.vault_count()
    };

    let vault_registry_count = {
        let vault_registry_data = vault_registry.data.borrow();
        let vault_registry = VaultRegistry::try_from_slice_unchecked(&vault_registry_data)?;
        vault_registry.vault_count()
    };

    if vault_count != vault_registry_count {
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
    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        weight_table,
        system_program,
        program_id,
        MAX_REALLOC_BYTES as usize,
        &weight_table_seeds,
    )?;

    Ok(())
}
