#[cfg(test)]
mod tests {

    use jito_tip_router_core::base_reward_router::{BaseRewardReceiver, BaseRewardRouter};
    use jito_tip_router_core::ncn_fee_group::NcnFeeGroup;
    use jito_tip_router_core::ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter};
    use jito_tip_router_core::weight_table::WeightTable;
    use jito_tip_router_core::{epoch_state::EpochState, error::TipRouterError};
    use solana_sdk::pubkey::Pubkey;

    use crate::fixtures::TestResult;
    use crate::fixtures::{test_builder::TestBuilder, tip_router_client::assert_tip_router_error};

    #[tokio::test]
    async fn close_all_epoch_accounts_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 3;
        const VAULT_COUNT: usize = 2;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        fixture
            .reward_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;
        fixture.close_epoch_accounts_for_test_ncn(&test_ncn).await?;

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_before_enough_epochs_after_consensus() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        fixture
            .reward_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch_to_close = fixture.clock().await.epoch;

        // Try Close Epoch State
        {
            let (epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, epoch_state, None)
                .await;

            assert_tip_router_error(result, TipRouterError::CannotCloseAccountNotEnoughEpochs);

            let result = fixture.get_account(&epoch_state).await?;
            assert!(result.is_some());
        }

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_before_consensus_is_reached() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch_to_close = fixture.clock().await.epoch;

        // Warp to way after close
        {
            let config: jito_tip_router_core::config::Config =
                fixture.tip_router_client().get_ncn_config(ncn).await?;
            let epochs_after_consensus_before_close = config.epochs_after_consensus_before_close();

            fixture
                .warp_epoch_incremental(epochs_after_consensus_before_close * 2)
                .await?;
        }

        // Try Close Epoch State
        {
            let (epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, epoch_state, None)
                .await;

            assert_tip_router_error(result, TipRouterError::ConsensusNotReached);

            let result = fixture.get_account(&epoch_state).await?;
            assert!(result.is_some());
        }

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_epoch_state_before_others() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        fixture
            .reward_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch_to_close = fixture.clock().await.epoch;

        // Warp to epoch to close
        {
            let config: jito_tip_router_core::config::Config =
                fixture.tip_router_client().get_ncn_config(ncn).await?;
            let epochs_after_consensus_before_close = config.epochs_after_consensus_before_close();

            fixture
                .warp_epoch_incremental(epochs_after_consensus_before_close + 1)
                .await?;
        }

        // Try Close Epoch State
        {
            let (epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, epoch_state, None)
                .await;

            assert_tip_router_error(result, TipRouterError::CannotCloseEpochStateAccount);

            let result = fixture.get_account(&epoch_state).await?;
            assert!(result.is_some());
        }

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_closed_account() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        fixture
            .reward_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch_to_close = fixture.clock().await.epoch;

        // Warp to epoch to close
        {
            let config: jito_tip_router_core::config::Config =
                fixture.tip_router_client().get_ncn_config(ncn).await?;
            let epochs_after_consensus_before_close = config.epochs_after_consensus_before_close();

            fixture
                .warp_epoch_incremental(epochs_after_consensus_before_close + 1)
                .await?;
        }

        // Close Weight Table
        {
            let (weight_table, _, _) = WeightTable::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, weight_table, None)
                .await?;

            let result = fixture.get_account(&weight_table).await?;
            assert!(result.is_none());
        }

        // Try Close Weight Table Again
        {
            fixture.warp_epoch_incremental(1).await?;

            let (weight_table, _, _) = WeightTable::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, weight_table, None)
                .await;

            assert_tip_router_error(result, TipRouterError::CannotCloseAccountAlreadyClosed);

            let result = fixture.get_account(&weight_table).await?;
            assert!(result.is_none());
        }

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_without_receiver() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        fixture
            .reward_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch_to_close = fixture.clock().await.epoch;

        // Warp to epoch to close
        {
            let config: jito_tip_router_core::config::Config =
                fixture.tip_router_client().get_ncn_config(ncn).await?;
            let epochs_after_consensus_before_close = config.epochs_after_consensus_before_close();

            fixture
                .warp_epoch_incremental(epochs_after_consensus_before_close + 1)
                .await?;
        }

        // Close NCN Reward Router
        {
            let group = NcnFeeGroup::default();
            let operator = test_ncn.operators[0].operator_pubkey;
            let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                group,
                &operator,
                &ncn,
                epoch_to_close,
            );

            let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
                &jito_tip_router_program::id(),
                group,
                &operator,
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, ncn_reward_router, None)
                .await;

            assert!(result.is_err());

            let result = fixture.get_account(&ncn_reward_router).await?;
            assert!(result.is_some());

            let result = fixture.get_account(&ncn_reward_receiver).await?;
            assert!(result.is_some());
        }

        // Close Base Router
        {
            let (base_reward_router, _, _) = BaseRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let (base_reward_receiver, _, _) = BaseRewardReceiver::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, base_reward_router, None)
                .await;

            assert!(result.is_err());

            let result = fixture.get_account(&base_reward_router).await?;
            assert!(result.is_some());

            let result = fixture.get_account(&base_reward_receiver).await?;
            assert!(result.is_some());
        }

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_bad_receiver() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        fixture
            .reward_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch_to_close = fixture.clock().await.epoch;

        // Warp to epoch to close
        {
            let config: jito_tip_router_core::config::Config =
                fixture.tip_router_client().get_ncn_config(ncn).await?;
            let epochs_after_consensus_before_close = config.epochs_after_consensus_before_close();

            fixture
                .warp_epoch_incremental(epochs_after_consensus_before_close + 1)
                .await?;
        }

        // Close NCN Reward Router
        {
            let group = NcnFeeGroup::default();
            let operator = test_ncn.operators[0].operator_pubkey;
            let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                group,
                &operator,
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(
                    ncn,
                    epoch_to_close,
                    ncn_reward_router,
                    Some(Pubkey::new_unique()),
                )
                .await;

            assert!(result.is_err());

            let result = fixture.get_account(&ncn_reward_router).await?;
            assert!(result.is_some());
        }

        // Close Base Router
        {
            let (base_reward_router, _, _) = BaseRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_close_epoch_account(
                    ncn,
                    epoch_to_close,
                    base_reward_router,
                    Some(Pubkey::new_unique()),
                )
                .await;

            assert!(result.is_err());

            let result = fixture.get_account(&base_reward_router).await?;
            assert!(result.is_some());
        }

        Ok(())
    }
}
