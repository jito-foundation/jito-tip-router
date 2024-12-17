use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, ncn_vault_ticket::NcnVaultTicket};
use jito_tip_router_core::vault_registry::VaultRegistry;
use jito_vault_core::{vault::Vault, vault_ncn_ticket::VaultNcnTicket};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

pub fn process_register_vault(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [restaking_config, tracked_mints, ncn, vault, vault_ncn_ticket, ncn_vault_ticket, restaking_program_id, vault_program_id] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    VaultRegistry::load(program_id, ncn.key, tracked_mints, true)?;
    Ncn::load(restaking_program_id.key, ncn, false)?;
    Vault::load(vault_program_id.key, vault, false)?;
    VaultNcnTicket::load(vault_program_id.key, vault_ncn_ticket, vault, ncn, false)?;
    NcnVaultTicket::load(
        restaking_program_id.key,
        ncn_vault_ticket,
        ncn,
        vault,
        false,
    )?;

    let epoch_length = {
        let restaking_config_data = restaking_config.data.borrow();
        Config::try_from_slice_unchecked(&restaking_config_data)?.epoch_length()
    };

    let clock = Clock::get()?;
    let slot = clock.slot;

    // Verify tickets are active
    let vault_ncn_ticket_data = vault_ncn_ticket.data.borrow();
    let vault_ncn_ticket = VaultNcnTicket::try_from_slice_unchecked(&vault_ncn_ticket_data)?;
    if !vault_ncn_ticket
        .state
        .is_active_or_cooldown(slot, epoch_length)
    {
        msg!("Vault NCN ticket is not enabled");
        return Err(ProgramError::InvalidAccountData);
    }

    let ncn_vault_ticket_data = ncn_vault_ticket.data.borrow();
    let ncn_vault_ticket = NcnVaultTicket::try_from_slice_unchecked(&ncn_vault_ticket_data)?;
    if !ncn_vault_ticket
        .state
        .is_active_or_cooldown(slot, epoch_length)
    {
        msg!("NCN vault ticket is not enabled");
        return Err(ProgramError::InvalidAccountData);
    }

    let mut tracked_mints_data = tracked_mints.try_borrow_mut_data()?;
    let tracked_mints = VaultRegistry::try_from_slice_unchecked_mut(&mut tracked_mints_data)?;

    let vault_data = vault.data.borrow();
    let vault = Vault::try_from_slice_unchecked(&vault_data)?;

    if !tracked_mints.has_st_mint(&vault.supported_mint) {
        msg!("Supported mint not registered");
        return Err(ProgramError::InvalidAccountData);
    }

    tracked_mints.register_vault(vault.supported_mint, vault.vault_index())?;

    Ok(())
}