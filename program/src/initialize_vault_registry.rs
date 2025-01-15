use jito_jsm_core::{
    create_account,
    loader::{load_system_account, load_system_program},
};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    config::Config as NcnConfig, constants::MAX_REALLOC_BYTES, vault_registry::VaultRegistry,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_vault_registry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [ncn_config, vault_registry, ncn_account, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify accounts
    load_system_account(vault_registry, true)?;
    load_system_program(system_program)?;

    Ncn::load(&jito_restaking_program::id(), ncn_account, false)?;
    NcnConfig::load(program_id, ncn_account.key, ncn_config, false)?;

    let (vault_registry_pda, vault_registry_bump, mut vault_registry_seeds) =
        VaultRegistry::find_program_address(program_id, ncn_account.key);
    vault_registry_seeds.push(vec![vault_registry_bump]);

    if vault_registry_pda != *vault_registry.key {
        return Err(ProgramError::InvalidSeeds);
    }

    create_account(
        payer,
        vault_registry,
        system_program,
        program_id,
        &Rent::get()?,
        MAX_REALLOC_BYTES,
        &vault_registry_seeds,
    )?;

    Ok(())
}
