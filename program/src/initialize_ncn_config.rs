use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    constants::MAX_FEE_BPS, error::TipRouterError, fees::FeeConfig, ncn_config::NcnConfig,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_initialize_ncn_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    block_engine_fee_bps: u16,
    dao_fee_bps: u16,
    default_ncn_fee_bps: u16,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn_account, dao_fee_wallet, ncn_admin, tie_breaker_admin, restaking_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_account(ncn_config, true)?;
    load_system_program(system_program)?;
    load_signer(ncn_admin, false)?;

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

    let ncn_data = ncn_account.data.borrow();
    let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
    if ncn.admin != *ncn_admin.key {
        return Err(TipRouterError::IncorrectNcnAdmin.into());
    }

    let (config_pda, config_bump, mut config_seeds) =
        NcnConfig::find_program_address(program_id, ncn_account.key);
    config_seeds.push(vec![config_bump]);

    if config_pda != *ncn_config.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if block_engine_fee_bps as u64 >= MAX_FEE_BPS {
        return Err(TipRouterError::FeeCapExceeded.into());
    }
    if dao_fee_bps as u64 > MAX_FEE_BPS {
        return Err(TipRouterError::FeeCapExceeded.into());
    }
    if default_ncn_fee_bps as u64 > MAX_FEE_BPS {
        return Err(TipRouterError::FeeCapExceeded.into());
    }

    create_account(
        ncn_admin,
        ncn_config,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(std::mem::size_of::<NcnConfig>() as u64)
            .unwrap(),
        &config_seeds,
    )?;

    let mut config_data = ncn_config.try_borrow_mut_data()?;
    config_data[0] = NcnConfig::DISCRIMINATOR;
    let config = NcnConfig::try_from_slice_unchecked_mut(&mut config_data)?;

    let fee_config = FeeConfig::new(
        *dao_fee_wallet.key,
        block_engine_fee_bps,
        dao_fee_bps,
        default_ncn_fee_bps,
        epoch,
    )?;

    *config = NcnConfig::new(
        *ncn_account.key,
        *tie_breaker_admin.key,
        *ncn_admin.key,
        &fee_config,
    );
    config.bump = config_bump;

    config.fee_config.check_fees_okay(epoch)?;

    Ok(())
}
