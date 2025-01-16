use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn::Ncn, ncn_vault_ticket::NcnVaultTicket, operator::Operator,
};
use jito_tip_router_core::{
    config::Config as NcnConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
    loaders::load_ncn_epoch,
    stake_weight::StakeWeights,
    weight_table::WeightTable,
};
use jito_vault_core::{
    vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

pub fn process_snapshot_vault_operator_delegation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, restaking_config, ncn, operator, vault, vault_ncn_ticket, ncn_vault_ticket, vault_operator_delegation, weight_table, epoch_snapshot, operator_snapshot] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, true)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    Config::load(&jito_restaking_program::id(), restaking_config, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    Vault::load(&jito_vault_program::id(), vault, false)?;

    NcnVaultTicket::load(
        &jito_restaking_program::id(),
        ncn_vault_ticket,
        ncn,
        vault,
        false,
    )?;

    if !vault_ncn_ticket.data_is_empty() {
        VaultNcnTicket::load(
            &jito_vault_program::id(),
            vault_ncn_ticket,
            vault,
            ncn,
            false,
        )?;
    }

    if !vault_operator_delegation.data_is_empty() {
        VaultOperatorDelegation::load(
            &jito_vault_program::id(),
            vault_operator_delegation,
            vault,
            operator,
            false,
        )?;
    }

    let current_slot = Clock::get()?.slot;
    let (_, ncn_epoch_length) = load_ncn_epoch(restaking_config, current_slot, None)?;

    WeightTable::load(program_id, weight_table, ncn.key, epoch, false)?;
    EpochSnapshot::load(program_id, ncn.key, epoch, epoch_snapshot, true)?;
    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        epoch,
        operator_snapshot,
        true,
    )?;

    // check vault is up to date
    let vault_needs_update = {
        let vault_data = vault.data.borrow();
        let vault_account = Vault::try_from_slice_unchecked(&vault_data)?;

        vault_account.is_update_needed(current_slot, ncn_epoch_length)?
    };
    if vault_needs_update {
        msg!("Vault is not up to date");
        return Err(TipRouterError::VaultNeedsUpdate.into());
    }

    let (vault_index, st_mint) = {
        let vault_data = vault.data.borrow();
        let vault_account = Vault::try_from_slice_unchecked(&vault_data)?;
        (vault_account.vault_index(), vault_account.supported_mint)
    };

    let is_active: bool = {
        let ncn_vault_okay = {
            let ncn_vault_ticket_data = ncn_vault_ticket.data.borrow();
            let ncn_vault_ticket_account =
                NcnVaultTicket::try_from_slice_unchecked(&ncn_vault_ticket_data)?;

            // If the NCN removes a vault, it should immediately be barred from the snapshot
            ncn_vault_ticket_account
                .state
                .is_active(current_slot, ncn_epoch_length)
        };

        let vault_ncn_okay = {
            if vault_ncn_ticket.data_is_empty() {
                false
            } else {
                let vault_ncn_ticket_data = vault_ncn_ticket.data.borrow();
                let vault_ncn_ticket_account =
                    VaultNcnTicket::try_from_slice_unchecked(&vault_ncn_ticket_data)?;

                // If a vault removes itself from the ncn, it should still be able to participate
                // until it is finished cooling down - this is so the operators with delegation
                // from this vault can still participate
                vault_ncn_ticket_account
                    .state
                    .is_active_or_cooldown(current_slot, ncn_epoch_length)
            }
        };

        let delegation_dne = vault_operator_delegation.data_is_empty();

        vault_ncn_okay && ncn_vault_okay && !delegation_dne
    };

    let (ncn_fee_group, reward_multiplier_bps, total_stake_weight) = {
        let weight_table_data = weight_table.data.borrow();
        let weight_table_account = WeightTable::try_from_slice_unchecked(&weight_table_data)?;
        let weight_entry = weight_table_account.get_weight_entry(&st_mint)?;

        weight_table_account.check_registry_for_vault(vault_index)?;

        let total_stake_weight: u128 = if is_active {
            let vault_operator_delegation_data = vault_operator_delegation.data.borrow();
            let vault_operator_delegation_account =
                VaultOperatorDelegation::try_from_slice_unchecked(&vault_operator_delegation_data)?;

            OperatorSnapshot::calculate_total_stake_weight(
                vault_operator_delegation_account,
                weight_table_account,
                &st_mint,
            )?
        } else {
            0u128
        };

        (
            weight_entry.st_mint_entry().ncn_fee_group(),
            weight_entry.st_mint_entry().reward_multiplier_bps(),
            total_stake_weight,
        )
    };

    // Increment vault operator delegation
    let mut operator_snapshot_data = operator_snapshot.try_borrow_mut_data()?;
    let operator_snapshot_account =
        OperatorSnapshot::try_from_slice_unchecked_mut(&mut operator_snapshot_data)?;

    let stake_weights =
        StakeWeights::snapshot(ncn_fee_group, total_stake_weight, reward_multiplier_bps)?;

    operator_snapshot_account.increment_vault_operator_delegation_registration(
        current_slot,
        vault.key,
        vault_index,
        ncn_fee_group,
        &stake_weights,
    )?;

    // If operator is finalized, increment operator registration
    if operator_snapshot_account.finalized() {
        let mut epoch_snapshot_data = epoch_snapshot.try_borrow_mut_data()?;
        let epoch_snapshot_account =
            EpochSnapshot::try_from_slice_unchecked_mut(&mut epoch_snapshot_data)?;

        epoch_snapshot_account.increment_operator_registration(
            current_slot,
            operator_snapshot_account.valid_operator_vault_delegations(),
            operator_snapshot_account.stake_weights(),
        )?;
    }

    // Update Epoch State
    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_snapshot_vault_operator_delegation(
            operator_snapshot_account.ncn_operator_index() as usize,
            operator_snapshot_account.finalized(),
        )?;
    }

    Ok(())
}
