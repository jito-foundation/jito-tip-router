use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    config::Config,
    constants::{
        MAX_EPOCHS_BEFORE_STALL, MAX_FEE_BPS, MAX_SLOTS_AFTER_CONSENSUS, MIN_EPOCHS_BEFORE_STALL,
        MIN_SLOTS_AFTER_CONSENSUS,
    },
    error::TipRouterError,
    fees::FeeConfig,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

// TODO rename to admin_initialize_config
pub fn process_initialize_ncn_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    block_engine_fee_bps: u16,
    dao_fee_bps: u16,
    default_ncn_fee_bps: u16,
    epochs_before_stall: u64,
    valid_slots_after_consensus: u64,
) -> ProgramResult {
    let [config, ncn_account, dao_fee_wallet, ncn_admin, tie_breaker_admin, restaking_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_account(config, true)?;
    load_system_program(system_program)?;
    load_signer(ncn_admin, false)?;

    Ncn::load(restaking_program.key, ncn_account, false)?;

    let epoch = Clock::get()?.epoch;

    let ncn_data = ncn_account.data.borrow();
    let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
    if ncn.admin != *ncn_admin.key {
        return Err(TipRouterError::IncorrectNcnAdmin.into());
    }

    let (config_pda, config_bump, mut config_seeds) =
        Config::find_program_address(program_id, ncn_account.key);
    config_seeds.push(vec![config_bump]);

    if config_pda != *config.key {
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

    if !(MIN_EPOCHS_BEFORE_STALL..=MAX_EPOCHS_BEFORE_STALL).contains(&epochs_before_stall) {
        return Err(TipRouterError::InvalidEpochsBeforeStall.into());
    }

    if !(MIN_SLOTS_AFTER_CONSENSUS..=MAX_SLOTS_AFTER_CONSENSUS)
        .contains(&valid_slots_after_consensus)
    {
        return Err(TipRouterError::InvalidSlotsAfterConsensus.into());
    }

    create_account(
        ncn_admin,
        config,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(std::mem::size_of::<Config>() as u64)
            .unwrap(),
        &config_seeds,
    )?;

    let mut config_data = config.try_borrow_mut_data()?;
    config_data[0] = Config::DISCRIMINATOR;
    let config = Config::try_from_slice_unchecked_mut(&mut config_data)?;

    let fee_config = FeeConfig::new(
        dao_fee_wallet.key,
        block_engine_fee_bps,
        dao_fee_bps,
        default_ncn_fee_bps,
        epoch,
    )?;

    *config = Config::new(
        ncn_account.key,
        tie_breaker_admin.key,
        ncn_admin.key,
        &fee_config,
        valid_slots_after_consensus,
        epochs_before_stall,
        config_bump,
    );

    config.fee_config.check_fees_okay(epoch)?;

    Ok(())
}
