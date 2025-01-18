use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    base_fee_group::BaseFeeGroup, config::Config, error::TipRouterError, ncn_fee_group::NcnFeeGroup,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

#[allow(clippy::too_many_arguments)]
pub fn process_admin_set_config_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_block_engine_fee_bps: Option<u16>,
    base_fee_group: Option<u8>,
    new_base_fee_wallet: Option<Pubkey>,
    new_base_fee_bps: Option<u16>,
    ncn_fee_group: Option<u8>,
    new_ncn_fee_bps: Option<u16>,
) -> ProgramResult {
    let [config, ncn_account, fee_admin] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_signer(fee_admin, true)?;

    Config::load(program_id, ncn_account.key, config, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn_account, false)?;

    let epoch = Clock::get()?.epoch;

    let mut config_data = config.try_borrow_mut_data()?;
    let config = Config::try_from_slice_unchecked_mut(&mut config_data)?;

    // Verify NCN and Admin
    if config.ncn != *ncn_account.key {
        return Err(TipRouterError::IncorrectNcn.into());
    }

    if config.fee_admin != *fee_admin.key {
        return Err(TipRouterError::IncorrectFeeAdmin.into());
    }

    let base_fee_group = base_fee_group.map(BaseFeeGroup::try_from).transpose()?;
    let ncn_fee_group = ncn_fee_group.map(NcnFeeGroup::try_from).transpose()?;

    config.fee_config.update_fee_config(
        new_block_engine_fee_bps,
        base_fee_group,
        new_base_fee_wallet,
        new_base_fee_bps,
        ncn_fee_group,
        new_ncn_fee_bps,
        epoch,
    )?;

    Ok(())
}
