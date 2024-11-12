use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{
    config::Config, ncn::Ncn, ncn_vault_ticket::NcnVaultTicket, operator::Operator,
};
use jito_tip_router_core::{
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot, VaultOperatorDelegationSnapshot},
    loaders::load_ncn_epoch,
    ncn_config::NcnConfig,
    weight_table::WeightTable,
};
use jito_vault_core::{
    vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_vault_operator_delegation_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [ncn_config, restaking_config, ncn, operator, vault, vault_ncn_ticket, ncn_vault_ticket, vault_operator_delegation, weight_table, epoch_snapshot, operator_snapshot, vault_operator_delegation_snapshot, payer, restaking_program_id, system_program] =
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
    Operator::load(restaking_program_id.key, operator, false)?;
    Vault::load(restaking_program_id.key, vault, false)?;

    VaultNcnTicket::load(
        restaking_program_id.key,
        vault_ncn_ticket,
        vault,
        ncn,
        false,
    )?;
    NcnVaultTicket::load(
        restaking_program_id.key,
        ncn_vault_ticket,
        ncn,
        vault,
        false,
    )?;

    //TODO check that st mint is supported?
    //TODO may not exist
    if !vault_operator_delegation.data_is_empty() {
        VaultOperatorDelegation::load(
            restaking_program_id.key,
            vault_operator_delegation,
            vault,
            operator,
            false,
        )?;
    }

    load_system_account(vault_operator_delegation_snapshot, true)?;
    load_system_program(system_program)?;
    //TODO check that it is not writable
    load_signer(payer, false)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, ncn_epoch_length) =
        load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    WeightTable::load(program_id, weight_table, ncn, ncn_epoch, false)?;
    EpochSnapshot::load(program_id, ncn.key, ncn_epoch, epoch_snapshot, true)?;
    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        ncn_epoch,
        epoch_snapshot,
        true,
    )?;

    let (
        vault_operator_delegation_snapshot_pubkey,
        vault_operator_delegation_snapshot_bump,
        mut vault_operator_delegation_snapshot_seeds,
    ) = VaultOperatorDelegationSnapshot::find_program_address(
        program_id,
        vault.key,
        operator.key,
        ncn.key,
        ncn_epoch,
    );
    vault_operator_delegation_snapshot_seeds.push(vec![vault_operator_delegation_snapshot_bump]);

    if vault_operator_delegation_snapshot_pubkey.ne(operator_snapshot.key) {
        msg!("Incorrect vault operator delegation snapshot PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing vault operator delegation snapshot {} for NCN: {} at epoch: {}",
        epoch_snapshot.key,
        ncn.key,
        ncn_epoch
    );
    create_account(
        payer,
        operator_snapshot,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(std::mem::size_of::<OperatorSnapshot>() as u64)
            .unwrap(),
        &vault_operator_delegation_snapshot_seeds,
    )?;

    let st_mint = {
        let vault_data = vault.data.borrow();
        let vault_account = Vault::try_from_slice_unchecked(&vault_data)?;
        vault_account.supported_mint
    };

    let is_active: bool = {
        let vault_ncn_ticket_data = vault_ncn_ticket.data.borrow();
        let vault_ncn_ticket_account =
            VaultNcnTicket::try_from_slice_unchecked(&vault_ncn_ticket_data)?;

        let ncn_vault_ticket_data = ncn_vault_ticket.data.borrow();
        let ncn_vault_ticket_account =
            NcnVaultTicket::try_from_slice_unchecked(&ncn_vault_ticket_data)?;

        let vault_ncn_okay = vault_ncn_ticket_account
            .state
            .is_active(current_slot, ncn_epoch_length);

        let ncn_vault_okay = ncn_vault_ticket_account
            .state
            .is_active(current_slot, ncn_epoch_length);

        let delegation_dne = vault_operator_delegation.data_is_empty();

        vault_ncn_okay && ncn_vault_okay && delegation_dne
    };

    let mut vault_operator_delegation_snapshot_data: std::cell::RefMut<'_, &mut [u8]> =
        operator_snapshot.try_borrow_mut_data()?;
    vault_operator_delegation_snapshot_data[0] = VaultOperatorDelegationSnapshot::DISCRIMINATOR;
    let vault_operator_delegation_snapshot_account =
        VaultOperatorDelegationSnapshot::try_from_slice_unchecked_mut(
            &mut vault_operator_delegation_snapshot_data,
        )?;

    *vault_operator_delegation_snapshot_account = if is_active {
        let vault_operator_delegation_data = vault_operator_delegation.data.borrow();
        let vault_operator_delegation_account =
            VaultOperatorDelegation::try_from_slice_unchecked(&vault_operator_delegation_data)?;

        let weight_table_data = weight_table.data.borrow();
        let weight_table_account = WeightTable::try_from_slice_unchecked(&weight_table_data)?;

        //TODO Ending here for the day
        VaultOperatorDelegationSnapshot::create_snapshot(
            *vault.key,
            *operator.key,
            *ncn.key,
            ncn_epoch,
            vault_operator_delegation_snapshot_bump,
            current_slot,
            st_mint,
            vault_operator_delegation_account,
            weight_table_account,
        )?
    } else {
        VaultOperatorDelegationSnapshot::new_inactive(
            *vault.key,
            *operator.key,
            *ncn.key,
            ncn_epoch,
            vault_operator_delegation_snapshot_bump,
            current_slot,
            st_mint,
        )
    };

    // Increment vault operator delegation
    let mut operator_snapshot_data = operator_snapshot.try_borrow_mut_data()?;
    let operator_snapshot_account =
        OperatorSnapshot::try_from_slice_unchecked_mut(&mut operator_snapshot_data)?;

    operator_snapshot_account.increment_vault_operator_delegation_registration(
        current_slot,
        vault_operator_delegation_snapshot_account.total_security(),
        vault_operator_delegation_snapshot_account.total_votes(),
    )?;

    // If operator is finalized, increment operator registration
    if operator_snapshot_account.finalized() {
        let mut epoch_snapshot_data = epoch_snapshot.try_borrow_mut_data()?;
        let epoch_snapshot_account =
            EpochSnapshot::try_from_slice_unchecked_mut(&mut epoch_snapshot_data)?;

        epoch_snapshot_account.increment_operator_registration(
            current_slot,
            operator_snapshot_account.valid_operator_vault_delegations(),
            operator_snapshot_account.total_votes(),
        )?;
    }

    Ok(())
}
