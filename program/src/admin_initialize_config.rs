use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::{load_signer, load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer,
    config::Config,
    constants::{
        MAX_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE, MAX_EPOCHS_BEFORE_STALL, MAX_FEE_BPS,
        MAX_VALID_SLOTS_AFTER_CONSENSUS, MIN_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE,
        MIN_EPOCHS_BEFORE_STALL, MIN_VALID_SLOTS_AFTER_CONSENSUS,
    },
    error::TipRouterError,
    fees::FeeConfig,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

#[allow(clippy::too_many_arguments)]
pub fn process_admin_initialize_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    block_engine_fee_bps: u16,
    dao_fee_bps: u16,
    default_ncn_fee_bps: u16,
    epochs_before_stall: u64,
    epochs_after_consensus_before_close: u64,
    valid_slots_after_consensus: u64,
) -> ProgramResult {
    let [config, ncn, dao_fee_wallet, ncn_admin, tie_breaker_admin, account_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_account(config, true)?;
    load_system_program(system_program)?;
    load_signer(ncn_admin, false)?;

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    let epoch = Clock::get()?.epoch;

    if block_engine_fee_bps as u64 > MAX_FEE_BPS {
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

    if !(MIN_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE..=MAX_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE)
        .contains(&epochs_after_consensus_before_close)
    {
        return Err(TipRouterError::InvalidEpochsBeforeClose.into());
    }

    if !(MIN_VALID_SLOTS_AFTER_CONSENSUS..=MAX_VALID_SLOTS_AFTER_CONSENSUS)
        .contains(&valid_slots_after_consensus)
    {
        return Err(TipRouterError::InvalidSlotsAfterConsensus.into());
    }

    let ncn_data = ncn.data.borrow();
    let ncn_account = Ncn::try_from_slice_unchecked(&ncn_data)?;
    if ncn_account.admin != *ncn_admin.key {
        return Err(TipRouterError::IncorrectNcnAdmin.into());
    }

    let (config_pda, config_bump, mut config_seeds) =
        Config::find_program_address(program_id, ncn.key);
    config_seeds.push(vec![config_bump]);

    if config_pda != *config.key {
        return Err(ProgramError::InvalidSeeds);
    }

    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        config,
        system_program,
        program_id,
        Config::SIZE,
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

    let starting_valid_epoch = epoch;

    *config = Config::new(
        ncn.key,
        tie_breaker_admin.key,
        ncn_admin.key,
        &fee_config,
        starting_valid_epoch,
        valid_slots_after_consensus,
        epochs_before_stall,
        epochs_after_consensus_before_close,
        config_bump,
    );

    config.fee_config.check_fees_okay(epoch)?;

    Ok(())
}
