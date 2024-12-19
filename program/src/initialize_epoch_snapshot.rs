use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    epoch_snapshot::EpochSnapshot, error::TipRouterError, fees, ncn_config::NcnConfig,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Initializes an Epoch Snapshot
pub fn process_initialize_epoch_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [ncn_config, restaking_config, ncn, weight_table, epoch_snapshot, payer, restaking_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;

    load_system_account(epoch_snapshot, true)?;
    load_system_program(system_program)?;
    //TODO check that it is not writable
    load_signer(payer, false)?;

    let current_slot = Clock::get()?.slot;
    let ncn_epoch = epoch;

    WeightTable::load(program_id, weight_table, ncn, ncn_epoch, false)?;

    // Weight table needs to be finalized before the snapshot can be taken
    let vault_count = {
        let weight_table_data = weight_table.data.borrow();
        let weight_table_account = WeightTable::try_from_slice_unchecked(&weight_table_data)?;

        if !weight_table_account.finalized() {
            msg!("Weight table must be finalized before initializing epoch snapshot");
            return Err(TipRouterError::WeightTableNotFinalized.into());
        }

        weight_table_account.vault_count()
    };

    let (epoch_snapshot_pubkey, epoch_snapshot_bump, mut epoch_snapshot_seeds) =
        EpochSnapshot::find_program_address(program_id, ncn.key, ncn_epoch);
    epoch_snapshot_seeds.push(vec![epoch_snapshot_bump]);

    if epoch_snapshot_pubkey.ne(epoch_snapshot.key) {
        msg!("Incorrect epoch snapshot PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Epoch snapshot {} for NCN: {} at epoch: {}",
        epoch_snapshot.key,
        ncn.key,
        ncn_epoch
    );
    create_account(
        payer,
        epoch_snapshot,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(std::mem::size_of::<EpochSnapshot>() as u64)
            .unwrap(),
        &epoch_snapshot_seeds,
    )?;

    let ncn_fees: fees::Fees = {
        let ncn_config_data = ncn_config.data.borrow();
        let ncn_config_account = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        *ncn_config_account.fee_config.current_fees(ncn_epoch)
    };

    let operator_count: u64 = {
        let ncn_data = ncn.data.borrow();
        let ncn_account = Ncn::try_from_slice_unchecked(&ncn_data)?;
        ncn_account.operator_count()
    };

    if operator_count == 0 {
        msg!("No operators to snapshot");
        return Err(TipRouterError::NoOperators.into());
    }

    let mut epoch_snapshot_data: std::cell::RefMut<'_, &mut [u8]> =
        epoch_snapshot.try_borrow_mut_data()?;
    epoch_snapshot_data[0] = EpochSnapshot::DISCRIMINATOR;
    let epoch_snapshot_account =
        EpochSnapshot::try_from_slice_unchecked_mut(&mut epoch_snapshot_data)?;

    *epoch_snapshot_account = EpochSnapshot::new(
        *ncn.key,
        ncn_epoch,
        epoch_snapshot_bump,
        current_slot,
        ncn_fees,
        operator_count,
        vault_count,
    );

    Ok(())
}
