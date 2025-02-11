use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_associated_token_account;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    base_fee_group::BaseFeeGroup,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as NcnConfig,
    constants::JITOSOL_MINT,
    epoch_state::EpochState,
    error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};
use spl_stake_pool::instruction::deposit_sol;

pub fn process_distribute_base_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    base_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ncn, base_reward_router, base_reward_receiver, base_fee_wallet, base_fee_wallet_ata, stake_pool_program, stake_pool, stake_pool_withdraw_authority, reserve_stake, manager_fee_account, referrer_pool_tokens_account, pool_mint, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    NcnConfig::load(program_id, ncn_config, ncn.key, false)?;
    BaseRewardRouter::load(program_id, base_reward_router, ncn.key, epoch, true)?;
    BaseRewardReceiver::load(program_id, base_reward_receiver, ncn.key, epoch, true)?;
    load_associated_token_account(base_fee_wallet_ata, base_fee_wallet.key, &JITOSOL_MINT)?;

    if stake_pool_program.key.ne(&spl_stake_pool::id()) {
        msg!("Incorrect stake pool program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    let group = BaseFeeGroup::try_from(base_fee_group)?;

    {
        let ncn_config_data = ncn_config.try_borrow_data()?;
        let ncn_config_account = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        let fee_wallet = ncn_config_account.fee_config.base_fee_wallet(group)?;

        if fee_wallet.ne(base_fee_wallet.key) {
            msg!("Incorrect base fee wallet");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Get rewards and update state
    let rewards = {
        let mut base_reward_router_data = base_reward_router.try_borrow_mut_data()?;
        let base_reward_router_account =
            BaseRewardRouter::try_from_slice_unchecked_mut(&mut base_reward_router_data)?;

        if base_reward_router_account.still_routing() {
            msg!("Rewards still routing");
            return Err(TipRouterError::RouterStillRouting.into());
        }

        base_reward_router_account.distribute_base_fee_group_rewards(group)?
    };

    if rewards > 0 {
        let (_, base_reward_receiver_bump, mut base_reward_receiver_seeds) =
            BaseRewardReceiver::find_program_address(program_id, ncn.key, epoch);
        base_reward_receiver_seeds.push(vec![base_reward_receiver_bump]);

        let deposit_ix = deposit_sol(
            stake_pool_program.key,
            stake_pool.key,
            stake_pool_withdraw_authority.key,
            reserve_stake.key,
            base_reward_receiver.key,
            base_fee_wallet_ata.key,
            manager_fee_account.key,
            referrer_pool_tokens_account.key,
            pool_mint.key,
            token_program.key,
            rewards,
        );

        // Invoke the deposit instruction with base_reward_router as signer
        invoke_signed(
            &deposit_ix,
            &[
                stake_pool.clone(),
                stake_pool_withdraw_authority.clone(),
                reserve_stake.clone(),
                base_reward_receiver.clone(),
                base_fee_wallet_ata.clone(),
                manager_fee_account.clone(),
                referrer_pool_tokens_account.clone(),
                pool_mint.clone(),
                system_program.clone(),
                token_program.clone(),
            ],
            &[base_reward_receiver_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }

    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_distribute_base_rewards(rewards)?;
    }

    Ok(())
}
