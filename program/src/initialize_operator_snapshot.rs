use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{
    config::Config, ncn::Ncn, ncn_operator_state::NcnOperatorState, operator::Operator,
};
use jito_tip_router_core::{
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    error::TipRouterError,
    fees,
    loaders::load_ncn_epoch,
    ncn_config::NcnConfig,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Initializes an Operator Snapshot
pub fn process_initialize_operator_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [ncn_config, restaking_config, ncn, operator, ncn_operator_state, epoch_snapshot, operator_snapshot, payer, restaking_program_id, system_program] =
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
    NcnOperatorState::load(
        restaking_program_id.key,
        ncn_operator_state,
        ncn,
        operator,
        false,
    )?;

    load_system_account(epoch_snapshot, true)?;
    load_system_program(system_program)?;
    //TODO check that it is not writable
    load_signer(payer, false)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, ncn_epoch_length) =
        load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    EpochSnapshot::load(program_id, ncn.key, ncn_epoch, epoch_snapshot, true)?;

    let is_active: bool = {
        let ncn_operator_state_data = ncn_operator_state.data.borrow();
        let ncn_operator_state_account =
            NcnOperatorState::try_from_slice_unchecked(&ncn_operator_state_data)?;

        ncn_operator_state_account
            .ncn_opt_in_state
            .is_active(current_slot, ncn_epoch_length)
            && ncn_operator_state_account
                .operator_opt_in_state
                .is_active(current_slot, ncn_epoch_length)
    };

    let (operator_snapshot_pubkey, operator_snapshot_bump, mut operator_snapshot_seeds) =
        EpochSnapshot::find_program_address(program_id, ncn.key, ncn_epoch);
    operator_snapshot_seeds.push(vec![operator_snapshot_bump]);

    if operator_snapshot_pubkey.ne(operator_snapshot.key) {
        msg!("Incorrect epoch snapshot PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Operator snapshot {} for NCN: {} at epoch: {}",
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
        &operator_snapshot_seeds,
    )?;

    let ncn_fees: fees::Fees = {
        let ncn_config_data = ncn_config.data.borrow();
        let ncn_config_account = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        ncn_config_account.fees
    };

    let operator_count: u64 = {
        let ncn_data = ncn.data.borrow();
        let ncn_account = Ncn::try_from_slice_unchecked(&ncn_data)?;
        ncn_account.operator_count()
    };

    let mut operator_snapshot_data: std::cell::RefMut<'_, &mut [u8]> =
        operator_snapshot.try_borrow_mut_data()?;
    operator_snapshot_data[0] = EpochSnapshot::DISCRIMINATOR;
    let operator_snapshot_account =
        OperatorSnapshot::try_from_slice_unchecked_mut(&mut operator_snapshot_data)?;

    *operator_snapshot_account =
        OperatorSnapshot::new(*ncn.key, ncn_epoch, current_slot, ncn_fees, operator_count);

    Ok(())
}
