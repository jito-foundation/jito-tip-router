use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_associated_token_account;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    constants::JITO_SOL_MINT,
    ncn_config::NcnConfig,
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
    let [restaking_config, ncn_config, ncn, operator, operator_ata, ncn_reward_router, ncn_reward_receiver, restaking_program, stake_pool_program, stake_pool, stake_pool_withdraw_authority, reserve_stake, manager_fee_account, referrer_pool_tokens_account, pool_mint, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    Operator::load(restaking_program.key, operator, true)?;

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
    load_associated_token_account(operator_ata, operator.key, &JITO_SOL_MINT)?;

    if stake_pool_program.key.ne(&spl_stake_pool::id()) {
        msg!("Incorrect stake pool program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    // Get rewards and update state
    let rewards = {
        let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
        let ncn_reward_router_account =
            NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

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

    Ok(())
}
