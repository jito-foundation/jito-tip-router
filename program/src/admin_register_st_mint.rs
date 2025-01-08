use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::{load_signer, load_token_mint};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    config::Config, ncn_fee_group::NcnFeeGroup, vault_registry::VaultRegistry,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process_admin_register_st_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    reward_multiplier_bps: u64,
    switchboard_feed: Option<Pubkey>,
    no_feed_weight: Option<u128>,
) -> ProgramResult {
    let [config, ncn, st_mint, vault_registry, admin, restaking_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    //TODO take out
    Config::load(program_id, ncn.key, config, false)?;
    VaultRegistry::load(program_id, ncn.key, vault_registry, true)?;
    Ncn::load(restaking_program.key, ncn, false)?;

    load_token_mint(st_mint)?;

    load_signer(admin, false)?;

    {
        let ncn_data = ncn.data.borrow();
        let ncn_account = Ncn::try_from_slice_unchecked(&ncn_data)?;

        if ncn_account.ncn_program_admin.ne(admin.key) {
            msg!("Admin is not the NCN program admin");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let mut vault_registry_data = vault_registry.data.borrow_mut();
    let vault_registry_account =
        VaultRegistry::try_from_slice_unchecked_mut(&mut vault_registry_data)?;

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    let switchboard_feed = switchboard_feed.unwrap_or_default();
    let no_feed_weight = no_feed_weight.unwrap_or_default();

    if switchboard_feed.eq(&Pubkey::default()) && no_feed_weight == 0 {
        msg!("Either switchboard feed or no feed weight must be set");
        return Err(ProgramError::InvalidArgument);
    }

    vault_registry_account.register_st_mint(
        st_mint.key,
        ncn_fee_group,
        reward_multiplier_bps,
        &switchboard_feed,
        no_feed_weight,
    )?;

    Ok(())
}
