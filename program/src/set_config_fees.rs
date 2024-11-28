use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    error::TipRouterError, ncn_config::NcnConfig, ncn_fee_group::NcnFeeGroup,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

pub fn process_set_config_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_fee_wallet: Option<Pubkey>,
    new_block_engine_fee_bps: Option<u64>,
    new_dao_fee_bps: Option<u64>,
    new_ncn_fee_bps: Option<u64>,
    new_ncn_fee_group: Option<u8>,
) -> ProgramResult {
    let [restaking_config, config, ncn_account, fee_admin, restaking_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_signer(fee_admin, true)?;

    NcnConfig::load(program_id, ncn_account.key, config, true)?;
    Ncn::load(restaking_program.key, ncn_account, false)?;
    Config::load(restaking_program.key, restaking_config, false)?;

    let ncn_epoch_length = {
        let config_data = restaking_config.data.borrow();
        let config = Config::try_from_slice_unchecked(&config_data)?;
        config.epoch_length()
    };

    let epoch = {
        let current_slot = Clock::get()?.slot;
        current_slot
            .checked_div(ncn_epoch_length)
            .ok_or(TipRouterError::DenominatorIsZero)?
    };

    let mut config_data = config.try_borrow_mut_data()?;
    if config_data[0] != NcnConfig::DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }
    let config = NcnConfig::try_from_slice_unchecked_mut(&mut config_data)?;

    // Verify NCN and Admin
    if config.ncn != *ncn_account.key {
        return Err(TipRouterError::IncorrectNcn.into());
    }

    if config.fee_admin != *fee_admin.key {
        return Err(TipRouterError::IncorrectFeeAdmin.into());
    }

    let new_ncn_fee_group = if let Some(new_ncn_fee_group) = new_ncn_fee_group {
        Some(NcnFeeGroup::try_from(new_ncn_fee_group)?)
    } else {
        None
    };

    config.fee_config.update_fee_config(
        new_fee_wallet,
        new_block_engine_fee_bps,
        new_dao_fee_bps,
        new_ncn_fee_bps,
        new_ncn_fee_group,
        epoch,
    )?;

    Ok(())
}
