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
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Reallocates the ballot box account to its full size.
/// This is needed due to Solana's account size limits during initialization.
pub fn process_close_epoch_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ncn, account_to_close, claim_status_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    ClaimStatusPayer::load(program_id, claim_status_payer, false)?;

    {
        let epochs_before_claim = {
            let ncn_config_data = ncn_config.try_borrow_data()?;
            let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
            ncn_config.epochs_before_claim()
        };

        let epoch_state_data = epoch_state.try_borrow_data()?;
        let epoch_state = EpochState::try_from_slice_unchecked(&epoch_state_data)?;

        let account_to_close_data = account_to_close.try_borrow_data()?;
        let discriminator = account_to_close_data[0];

        match discriminator {
            EpochState::DISCRIMINATOR => {
                epoch_state.check_can_close(epochs_before_claim)?;
            }
            WeightTable::DISCRIMINATOR => {
                let weight_table = WeightTable::try_from_slice_unchecked(&account_to_close_data)?;
                weight_table.check_can_close(&epoch_state, epochs_before_claim)?;
            }
            EpochSnapshot::DISCRIMINATOR => {
                let epoch_snapshot =
                    EpochSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                epoch_snapshot.check_can_close(&epoch_state, epochs_before_claim)?;
            }
            OperatorSnapshot::DISCRIMINATOR => {
                let operator_snapshot =
                    OperatorSnapshot::try_from_slice_unchecked(&account_to_close_data)?;
                operator_snapshot.check_can_close(&epoch_state, epochs_before_claim)?;
            }
            BallotBox::DISCRIMINATOR => {
                let ballot_box = BallotBox::try_from_slice_unchecked(&account_to_close_data)?;
                ballot_box.check_can_close(&epoch_state, epochs_before_claim)?;
            }
            BaseRewardRouter::DISCRIMINATOR => {
                let base_reward_router =
                    BaseRewardRouter::try_from_slice_unchecked(&account_to_close_data)?;
                base_reward_router.check_can_close(&epoch_state, epochs_before_claim)?;
            }
            NcnRewardRouter::DISCRIMINATOR => {
                let ncn_reward_router =
                    NcnRewardRouter::try_from_slice_unchecked(&account_to_close_data)?;
                ncn_reward_router.check_can_close(&epoch_state, epochs_before_claim)?;
            }
            _ => {
                return Err(TipRouterError::InvalidAccountToCloseDiscriminator.into());
            }
        }
    }

    ClaimStatusPayer::close_account(program_id, claim_status_payer, account_to_close)?;

    Ok(())
}
