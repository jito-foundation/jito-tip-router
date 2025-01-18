use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_associated_token_account;
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    config::Config as NcnConfig,
    constants::JITOSOL_MINT,
    epoch_snapshot::OperatorSnapshot,
    epoch_state::EpochState,
    error::TipRouterError,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};
use spl_stake_pool::instruction::deposit_sol;

/// Can be backfilled for previous epochs
pub fn process_distribute_ncn_operator_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ncn, operator, operator_ata, operator_snapshot, ncn_reward_router, ncn_reward_receiver, stake_pool_program, stake_pool, stake_pool_withdraw_authority, reserve_stake, manager_fee_account, referrer_pool_tokens_account, pool_mint, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, true)?;
    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        epoch,
        operator_snapshot,
        true,
    )?;

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    NcnRewardRouter::load(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        ncn_reward_router,
        true,
    )?;
    NcnRewardReceiver::load(
        program_id,
        ncn_reward_receiver,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        true,
    )?;
    load_associated_token_account(operator_ata, operator.key, &JITOSOL_MINT)?;

    if stake_pool_program.key.ne(&spl_stake_pool::id()) {
        msg!("Incorrect stake pool program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    // Get rewards and update state
    let rewards = {
        let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
        let ncn_reward_router_account =
            NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

        if ncn_reward_router_account.still_routing() {
            msg!("Rewards still routing");
            return Err(TipRouterError::RouterStillRouting.into());
        }

        ncn_reward_router_account.distribute_operator_rewards()?
    };

    if rewards > 0 {
        let (_, ncn_reward_receiver_bump, mut ncn_reward_receiver_seeds) =
            NcnRewardReceiver::find_program_address(
                program_id,
                ncn_fee_group,
                operator.key,
                ncn.key,
                epoch,
            );
        ncn_reward_receiver_seeds.push(vec![ncn_reward_receiver_bump]);

        let deposit_ix = deposit_sol(
            stake_pool_program.key,
            stake_pool.key,
            stake_pool_withdraw_authority.key,
            reserve_stake.key,
            ncn_reward_receiver.key,
            operator_ata.key,
            manager_fee_account.key,
            referrer_pool_tokens_account.key,
            pool_mint.key,
            token_program.key,
            rewards,
        );

        // Invoke the deposit instruction with ncn_reward_receiver as signer
        invoke_signed(
            &deposit_ix,
            &[
                stake_pool.clone(),
                stake_pool_withdraw_authority.clone(),
                reserve_stake.clone(),
                ncn_reward_receiver.clone(),
                operator_ata.clone(),
                manager_fee_account.clone(),
                referrer_pool_tokens_account.clone(),
                pool_mint.clone(),
                system_program.clone(),
                token_program.clone(),
            ],
            &[ncn_reward_receiver_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }

    {
        let operator_snapshot_data = operator_snapshot.try_borrow_data()?;
        let operator_snapshot_account =
            OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_distribute_ncn_rewards(
            operator_snapshot_account.ncn_operator_index() as usize,
            ncn_fee_group,
            rewards,
        )?;
    }

    Ok(())
}
