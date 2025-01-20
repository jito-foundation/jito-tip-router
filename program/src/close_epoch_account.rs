use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_reward_router::BaseRewardRouter,
    claim_status_payer::ClaimStatusPayer,
    config::Config as NcnConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
    ncn_reward_router::NcnRewardRouter,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Reallocates the ballot box account to its full size.
/// This is needed due to Solana's account size limits during initialization.
pub fn process_close_epoch_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, config, ncn, account_to_close, claim_status_payer, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    NcnConfig::load(program_id, ncn.key, config, false)?;
    ClaimStatusPayer::load(program_id, claim_status_payer, false)?;

    {
        let epochs_before_claim = {
            let config_data = config.try_borrow_data()?;
            let config_account = NcnConfig::try_from_slice_unchecked(&config_data)?;
            config_account.epochs_before_claim()
        };

        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        let epoch_state_epoch = epoch_state.epoch();

        // Epoch Check
        {
            let current_epoch = Clock::get()?.epoch;
            let epoch_delta = current_epoch.saturating_sub(epoch_state_epoch);
            if epoch_delta < epochs_before_claim {
                msg!("Not enough epochs have passed since epoch state creation");
                return Err(TipRouterError::CannotCloseAccount.into());
            }
        }

        // Progress Check
        {
            // Check upload progress is complete
            if !epoch_state.upload_progress().is_complete() {
                msg!("Cannot close account until upload is complete");
                return Err(TipRouterError::CannotCloseAccount.into());
            }

            // Check distribution progress is complete
            if !epoch_state.total_distribution_progress().is_complete() {
                msg!("Cannot close account until distribution is complete");
                return Err(TipRouterError::CannotCloseAccount.into());
            }
        }

        // Account Check
        {
            let account_to_close_data = account_to_close.try_borrow_data()?;
            let discriminator = account_to_close_data[0];

            match discriminator {
                EpochState::DISCRIMINATOR => {
                    epoch_state.check_can_close()?;

                    epoch_state.close_epoch_state();
                }
                WeightTable::DISCRIMINATOR => {
                    let weight_table =
                        WeightTable::try_from_slice_unchecked(&account_to_close_data)?;
                    weight_table.check_can_close(&epoch_state)?;

                    epoch_state.close_weight_table();
                }
                EpochSnapshot::DISCRIMINATOR => {
                    let epoch_snapshot =
                        EpochSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                    epoch_snapshot.check_can_close(&epoch_state)?;

                    epoch_state.close_epoch_snapshot();
                }
                OperatorSnapshot::DISCRIMINATOR => {
                    let operator_snapshot =
                        OperatorSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                    operator_snapshot.check_can_close(&epoch_state)?;

                    let ncn_operator_index = operator_snapshot.ncn_operator_index() as usize;
                    epoch_state.close_operator_snapshot(ncn_operator_index);
                }
                BallotBox::DISCRIMINATOR => {
                    let ballot_box = BallotBox::try_from_slice_unchecked(&account_to_close_data)?;
                    ballot_box.check_can_close(&epoch_state)?;

                    epoch_state.close_ballot_box();
                }
                BaseRewardRouter::DISCRIMINATOR => {
                    let base_reward_router =
                        BaseRewardRouter::try_from_slice_unchecked(&account_to_close_data)?;
                    base_reward_router.check_can_close(&epoch_state)?;

                    epoch_state.close_base_reward_router();
                }
                NcnRewardRouter::DISCRIMINATOR => {
                    let ncn_reward_router =
                        NcnRewardRouter::try_from_slice_unchecked(&account_to_close_data)?;
                    ncn_reward_router.check_can_close(&epoch_state)?;

                    let ncn_operator_index = ncn_reward_router.ncn_operator_index() as usize;
                    let group = ncn_reward_router.ncn_fee_group();

                    epoch_state.close_ncn_reward_router(ncn_operator_index, group);
                }
                _ => {
                    return Err(TipRouterError::InvalidAccountToCloseDiscriminator.into());
                }
            }
        }
    }

    ClaimStatusPayer::close_account(program_id, claim_status_payer, account_to_close)?;

    Ok(())
}
