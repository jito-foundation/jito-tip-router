use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    error::TipRouterError, loaders::load_ncn_epoch, ncn_config::NcnConfig,
    ncn_fee_group::NcnFeeGroup, ncn_reward_router::NcnRewardRouter,
};
use jito_vault_core::vault::Vault;
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_distribute_ncn_vault_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, operator, vault, ncn_reward_router, restaking_program, vault_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    if vault_program.key.ne(&jito_vault_program::id()) {
        msg!("Incorrect vault program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    Operator::load(restaking_program.key, operator, false)?;
    Vault::load(vault_program.key, vault, true)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;
    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    NcnRewardRouter::load(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        ncn_epoch,
        ncn_reward_router,
        true,
    )?;

    //TODO do we want an Operator Fee Wallet?

    // Get rewards and update state
    let rewards = {
        let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
        let ncn_reward_router_account =
            NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

        ncn_reward_router_account.distribute_vault_reward_route(vault.key)?
    };

    if rewards == 0 {
        msg!("No rewards to distribute");
        return Err(TipRouterError::NoRewards.into());
    }

    {
        msg!("rewards {}", rewards);
        msg!("vault {}", vault.key);
        msg!("vault {}", vault.lamports.borrow_mut());
        msg!("router {}", ncn_reward_router.key);
        msg!("router {}", ncn_reward_router.lamports.borrow_mut());
        msg!(
            "rent {}",
            Rent::get()?.minimum_balance(ncn_reward_router.data_len())
        );
    }

    // Send rewards
    {
        **vault.lamports.borrow_mut() = vault
            .lamports()
            .checked_add(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        **ncn_reward_router.lamports.borrow_mut() = ncn_reward_router
            .lamports()
            .checked_sub(rewards)
            .ok_or(TipRouterError::ArithmeticUnderflowError)?;

        // let ix = transfer(vault.key, ncn_reward_router.key, rewards);

        // solana_program::program::invoke(&ix, &[vault, ncn_reward_router])?;

        // let (_, ncn_reward_router_bump, mut ncn_reward_router_seeds) =
        //     NcnRewardRouter::find_program_address(
        //         program_id,
        //         ncn_fee_group,
        //         operator.key,
        //         ncn.key,
        //         ncn_epoch,
        //     );
        // ncn_reward_router_seeds.push(vec![ncn_reward_router_bump]);

        // // Convert Vec<Vec<u8>> to slice of slices
        // let seeds: &[&[u8]] = &ncn_reward_router_seeds
        //     .iter()
        //     .map(|v| v.as_slice())
        //     .collect::<Vec<&[u8]>>();

        // invoke_signed(
        //     &system_instruction::transfer(ncn_reward_router.key, vault.key, rewards),
        //     &[ncn_reward_router.clone(), vault.clone()],
        //     &[seeds],
        // )?;
    }

    Ok(())
}
