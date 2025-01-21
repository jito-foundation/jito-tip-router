use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer,
    ballot_box::BallotBox,
    base_fee_group::BaseFeeGroup,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as NcnConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Reallocates the ballot box account to its full size.
/// This is needed due to Solana's account size limits during initialization.
#[allow(clippy::cognitive_complexity)]
pub fn process_close_epoch_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let (required_accounts, optional_accounts) = accounts.split_at(7);
    let [epoch_state, config, ncn, account_to_close, account_payer, dao_wallet, system_program] =
        required_accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    NcnConfig::load(program_id, ncn.key, config, false)?;
    AccountPayer::load(program_id, ncn.key, account_payer, false)?;

    // Empty Account Check
    if account_to_close.data_is_empty() {
        msg!("Account already closed");
        return Err(TipRouterError::CannotCloseAccount.into());
    }

    {
        let config_data = config.try_borrow_data()?;
        let config_account = NcnConfig::try_from_slice_unchecked(&config_data)?;

        // Check correct DAO wallet
        if config_account
            .fee_config
            .base_fee_wallet(BaseFeeGroup::dao())?
            .ne(dao_wallet.key)
        {
            return Err(TipRouterError::InvalidDaoWallet.into());
        }

        let epochs_before_claim = config_account.epochs_before_claim();

        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;

        // Epoch Check - epochs after consensus is reached
        {
            let current_epoch = Clock::get()?.epoch;
            let epoch_consensus_reached = epoch_state_account.epoch_consensus_reached()?;

            let epoch_delta = current_epoch.saturating_sub(epoch_consensus_reached);
            if epoch_delta < epochs_before_claim {
                msg!("Not enough epochs have passed since epoch state creation");
                return Err(TipRouterError::CannotCloseAccount.into());
            }
        }

        // Progress Check
        {
            if !epoch_state_account.voting_progress().is_complete() {
                msg!("Cannot close account until voting is complete");
                return Err(TipRouterError::CannotCloseAccount.into());
            }
        }

        // Account Check
        {
            let discriminator = {
                if account_to_close.key.eq(epoch_state.key) {
                    // Cannot borrow the data again
                    EpochState::DISCRIMINATOR
                } else {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    account_to_close_data[0]
                }
            };

            match discriminator {
                EpochState::DISCRIMINATOR => {
                    epoch_state_account.check_can_close()?;
                    epoch_state_account.close_epoch_state();
                }
                WeightTable::DISCRIMINATOR => {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let weight_table =
                        WeightTable::try_from_slice_unchecked(&account_to_close_data)?;
                    weight_table.check_can_close(epoch_state_account)?;

                    epoch_state_account.close_weight_table();
                }
                EpochSnapshot::DISCRIMINATOR => {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let epoch_snapshot =
                        EpochSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                    epoch_snapshot.check_can_close(epoch_state_account)?;

                    epoch_state_account.close_epoch_snapshot();
                }
                OperatorSnapshot::DISCRIMINATOR => {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let operator_snapshot =
                        OperatorSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                    operator_snapshot.check_can_close(epoch_state_account)?;

                    let ncn_operator_index = operator_snapshot.ncn_operator_index() as usize;
                    epoch_state_account.close_operator_snapshot(ncn_operator_index);
                }
                BallotBox::DISCRIMINATOR => {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let ballot_box = BallotBox::try_from_slice_unchecked(&account_to_close_data)?;
                    ballot_box.check_can_close(epoch_state_account)?;

                    epoch_state_account.close_ballot_box();
                }
                BaseRewardRouter::DISCRIMINATOR => {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let base_reward_router =
                        BaseRewardRouter::try_from_slice_unchecked(&account_to_close_data)?;
                    base_reward_router.check_can_close(epoch_state_account)?;

                    let [base_reward_receiver] = optional_accounts else {
                        msg!("Base reward receiver account is missing");
                        return Err(ProgramError::NotEnoughAccountKeys);
                    };

                    BaseRewardReceiver::close(
                        program_id,
                        ncn.key,
                        epoch,
                        base_reward_receiver,
                        dao_wallet,
                        account_payer,
                    )?;

                    epoch_state_account.close_base_reward_router();
                }
                NcnRewardRouter::DISCRIMINATOR => {
                    let account_to_close_data = account_to_close.try_borrow_data()?;
                    let ncn_reward_router =
                        NcnRewardRouter::try_from_slice_unchecked(&account_to_close_data)?;
                    ncn_reward_router.check_can_close(epoch_state_account)?;

                    let ncn_operator_index = ncn_reward_router.ncn_operator_index() as usize;
                    let operator = ncn_reward_router.operator();
                    let ncn_fee_group = ncn_reward_router.ncn_fee_group();

                    let [ncn_reward_receiver] = optional_accounts else {
                        msg!("NCN reward receiver account is missing");
                        return Err(ProgramError::NotEnoughAccountKeys);
                    };

                    NcnRewardReceiver::close(
                        program_id,
                        ncn_fee_group,
                        operator,
                        ncn.key,
                        epoch,
                        ncn_reward_receiver,
                        dao_wallet,
                        account_payer,
                    )?;

                    epoch_state_account.close_ncn_reward_router(ncn_operator_index, ncn_fee_group);
                }
                _ => {
                    return Err(TipRouterError::InvalidAccountToCloseDiscriminator.into());
                }
            }
        }
    }

    AccountPayer::close_account(program_id, account_payer, account_to_close)
}
