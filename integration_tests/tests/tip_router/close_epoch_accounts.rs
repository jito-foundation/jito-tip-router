#[cfg(test)]
mod tests {

    use jito_tip_router_core::ballot_box::BallotBox;
    use jito_tip_router_core::base_reward_router::{BaseRewardReceiver, BaseRewardRouter};
    use jito_tip_router_core::epoch_snapshot::{EpochSnapshot, OperatorSnapshot};
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
    async fn cannot_reopen_accounts() -> TestResult<()> {
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
        // Try To Create Weight table again
        {
            let (weight_table, _, _) = WeightTable::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_initialize_weight_table(ncn, epoch_to_close)
                .await;

            assert_tip_router_error(result, TipRouterError::EpochIsClosingDown);

            let result = fixture.get_account(&weight_table).await?;
            assert!(result.is_none());
        }

        // Close Epoch Snapshot
        {
            let (epoch_snapshot, _, _) = EpochSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, epoch_snapshot, None)
                .await?;

            let result = fixture.get_account(&epoch_snapshot).await?;
            assert!(result.is_none());
        }
        // Try To Create Epoch Snapshot again
        {
            let (epoch_snapshot, _, _) = EpochSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_initialize_epoch_snapshot(ncn, epoch_to_close)
                .await;

            assert_tip_router_error(result, TipRouterError::EpochIsClosingDown);

            let result = fixture.get_account(&epoch_snapshot).await?;
            assert!(result.is_none());
        }

        // Close Operator Snapshot
        {
            let operator = test_ncn.operators[0].operator_pubkey;
            let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &operator,
                &ncn,
                epoch_to_close,
            );

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, operator_snapshot, None)
                .await?;

            let result = fixture.get_account(&operator_snapshot).await?;
            assert!(result.is_none());
        }
        // Try To Create Operator Snapshot again
        {
            let operator = test_ncn.operators[0].operator_pubkey;
            let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &operator,
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_initialize_operator_snapshot(operator, ncn, epoch_to_close)
                .await;

            assert_tip_router_error(result, TipRouterError::EpochIsClosingDown);

            let result = fixture.get_account(&operator_snapshot).await?;
            assert!(result.is_none());
        }

        // Close Ballot Box
        {
            let (ballot_box, _, _) = BallotBox::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, ballot_box, None)
                .await?;

            let result = fixture.get_account(&ballot_box).await?;
            assert!(result.is_none());
        }
        // Try To Create Ballot Box again
        {
            let (ballot_box, _, _) = BallotBox::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_initialize_ballot_box(ncn, epoch_to_close)
                .await;

            assert_tip_router_error(result, TipRouterError::EpochIsClosingDown);

            let result = fixture.get_account(&ballot_box).await?;
            assert!(result.is_none());
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

            tip_router_client
                .do_close_epoch_account(
                    ncn,
                    epoch_to_close,
                    base_reward_router,
                    Some(base_reward_receiver),
                )
                .await?;

            let result = fixture.get_account(&base_reward_router).await?;
            assert!(result.is_none());

            let result = fixture.get_account(&base_reward_receiver).await?;
            assert!(result.is_none());
        }
        // Try To Create Base Router again
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
                .do_initialize_base_reward_router(ncn, epoch_to_close)
                .await;

            assert_tip_router_error(result, TipRouterError::EpochIsClosingDown);

            let result = fixture.get_account(&base_reward_router).await?;
            assert!(result.is_none());

            let result = fixture.get_account(&base_reward_receiver).await?;
            assert!(result.is_none());
        }

        for group in NcnFeeGroup::all_groups() {
            // Close NCN Reward Router
            {
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

                tip_router_client
                    .do_close_epoch_account(
                        ncn,
                        epoch_to_close,
                        ncn_reward_router,
                        Some(ncn_reward_receiver),
                    )
                    .await?;

                let result = fixture.get_account(&ncn_reward_router).await?;
                assert!(result.is_none());

                let result = fixture.get_account(&ncn_reward_receiver).await?;
                assert!(result.is_none());
            }

            // Try To Create NCN Reward Router again
            {
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
                    .do_initialize_ncn_reward_router(group, ncn, operator, epoch_to_close)
                    .await;

                assert_tip_router_error(result, TipRouterError::EpochIsClosingDown);

                let result = fixture.get_account(&ncn_reward_router).await?;
                assert!(result.is_none());

                let result = fixture.get_account(&ncn_reward_receiver).await?;
                assert!(result.is_none());
            }
        }

        // Close Epoch State
        {
            let (epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, epoch_state, None)
                .await?;

            let result = fixture.get_account(&epoch_state).await?;
            assert!(result.is_none());
        }
        // Try To Create Epoch State again
        {
            let (epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let result = tip_router_client
                .do_full_initialize_epoch_state(ncn, epoch_to_close)
                .await;

            assert_tip_router_error(result, TipRouterError::MarkerExists);

            let result = fixture.get_account(&epoch_state).await?;
            assert!(result.is_none());
        }

        Ok(())
    }

    #[tokio::test]
    async fn cannot_close_wrong_epoch_or_ncn_accounts() -> TestResult<()> {
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

        let epoch_to_close = fixture.clock().await.epoch;

        let mut bad_test_ncn = fixture.create_just_test_ncn().await?;
        fixture
            .add_operators_to_test_ncn(&mut bad_test_ncn, OPERATOR_COUNT, None)
            .await?;
        fixture
            .add_vaults_to_test_ncn(&mut bad_test_ncn, VAULT_COUNT, None)
            .await?;
        fixture
            .add_delegation_in_test_ncn(&bad_test_ncn, 100)
            .await?;
        fixture
            .add_vault_registry_to_test_ncn(&bad_test_ncn)
            .await?;
        fixture.snapshot_test_ncn(&bad_test_ncn).await?;
        fixture.vote_test_ncn(&bad_test_ncn).await?;
        fixture
            .reward_test_ncn(&bad_test_ncn, 10_000, &pool_root)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let bad_ncn = bad_test_ncn.ncn_root.ncn_pubkey;

        // Warp to epoch to close
        {
            let config: jito_tip_router_core::config::Config =
                fixture.tip_router_client().get_ncn_config(ncn).await?;
            let epochs_after_consensus_before_close = config.epochs_after_consensus_before_close();

            fixture
                .warp_epoch_incremental(epochs_after_consensus_before_close + 1)
                .await?;
        }

        // Try Close Bad Weight Table
        {
            let (bad_epoch_weight_table, _, _) = WeightTable::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close + 1,
            );
            let (bad_ncn_weight_table, _, _) = WeightTable::find_program_address(
                &jito_tip_router_program::id(),
                &bad_ncn,
                epoch_to_close,
            );
            let (good_weight_table, _, _) = WeightTable::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let bad_epoch_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_epoch_weight_table, None)
                .await;

            let bad_ncn_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_ncn_weight_table, None)
                .await;

            assert!(bad_epoch_result.is_err());
            assert!(bad_ncn_result.is_err());

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, good_weight_table, None)
                .await?;
        }

        // Try Close Bad Epoch Snapshot
        {
            let (bad_epoch_epoch_snapshot, _, _) = EpochSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close + 1,
            );
            let (bad_ncn_epoch_snapshot, _, _) = EpochSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &bad_ncn,
                epoch_to_close,
            );
            let (good_epoch_snapshot, _, _) = EpochSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let bad_epoch_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_epoch_epoch_snapshot, None)
                .await;

            let bad_ncn_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_ncn_epoch_snapshot, None)
                .await;

            assert!(bad_epoch_result.is_err());
            assert!(bad_ncn_result.is_err());

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, good_epoch_snapshot, None)
                .await?;
        }

        // Try Close Bad Operator Snapshot
        {
            let operator = test_ncn.operators[0].operator_pubkey;
            let (bad_epoch_operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &operator,
                &ncn,
                epoch_to_close + 1,
            );
            let (bad_ncn_operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &operator,
                &bad_ncn,
                epoch_to_close,
            );
            let (good_operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &operator,
                &ncn,
                epoch_to_close,
            );

            let bad_epoch_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_epoch_operator_snapshot, None)
                .await;

            let bad_ncn_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_ncn_operator_snapshot, None)
                .await;

            assert!(bad_epoch_result.is_err());
            assert!(bad_ncn_result.is_err());

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, good_operator_snapshot, None)
                .await?;
        }

        // Try Close Bad Ballot Box
        {
            let (bad_epoch_ballot_box, _, _) = BallotBox::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close + 1,
            );
            let (bad_ncn_ballot_box, _, _) = BallotBox::find_program_address(
                &jito_tip_router_program::id(),
                &bad_ncn,
                epoch_to_close,
            );
            let (good_ballot_box, _, _) = BallotBox::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let bad_epoch_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_epoch_ballot_box, None)
                .await;

            let bad_ncn_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_ncn_ballot_box, None)
                .await;

            assert!(bad_epoch_result.is_err());
            assert!(bad_ncn_result.is_err());

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, good_ballot_box, None)
                .await?;
        }

        // Try Close Bad Base Reward Router
        {
            let (bad_epoch_base_reward_router, _, _) = BaseRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close + 1,
            );
            let (bad_ncn_base_reward_router, _, _) = BaseRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                &bad_ncn,
                epoch_to_close,
            );
            let (good_base_reward_router, _, _) = BaseRewardRouter::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let (receiver, _, _) = BaseRewardReceiver::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let bad_epoch_result = tip_router_client
                .do_close_epoch_account(
                    ncn,
                    epoch_to_close,
                    bad_epoch_base_reward_router,
                    Some(receiver),
                )
                .await;

            let bad_ncn_result = tip_router_client
                .do_close_epoch_account(
                    ncn,
                    epoch_to_close,
                    bad_ncn_base_reward_router,
                    Some(receiver),
                )
                .await;

            assert!(bad_epoch_result.is_err());
            assert!(bad_ncn_result.is_err());

            tip_router_client
                .do_close_epoch_account(
                    ncn,
                    epoch_to_close,
                    good_base_reward_router,
                    Some(receiver),
                )
                .await?;
        }

        // Try Close Bad NCN Reward Router (for each fee group)
        {
            let operator = test_ncn.operators[0].operator_pubkey;
            for group in NcnFeeGroup::all_groups() {
                let (bad_epoch_ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
                    &jito_tip_router_program::id(),
                    group,
                    &operator,
                    &ncn,
                    epoch_to_close + 1,
                );
                let (bad_ncn_ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
                    &jito_tip_router_program::id(),
                    group,
                    &operator,
                    &bad_ncn,
                    epoch_to_close,
                );
                let (good_ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
                    &jito_tip_router_program::id(),
                    group,
                    &operator,
                    &ncn,
                    epoch_to_close,
                );

                let (receiver, _, _) = NcnRewardReceiver::find_program_address(
                    &jito_tip_router_program::id(),
                    group,
                    &operator,
                    &ncn,
                    epoch_to_close,
                );

                let bad_epoch_result = tip_router_client
                    .do_close_epoch_account(
                        ncn,
                        epoch_to_close,
                        bad_epoch_ncn_reward_router,
                        Some(receiver),
                    )
                    .await;

                let bad_ncn_result = tip_router_client
                    .do_close_epoch_account(
                        ncn,
                        epoch_to_close,
                        bad_ncn_ncn_reward_router,
                        Some(receiver),
                    )
                    .await;

                assert!(bad_epoch_result.is_err());
                assert!(bad_ncn_result.is_err());

                tip_router_client
                    .do_close_epoch_account(
                        ncn,
                        epoch_to_close,
                        good_ncn_reward_router,
                        Some(receiver),
                    )
                    .await?;
            }
        }

        // Try Close Bad Epoch State
        {
            let (bad_epoch_epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close + 1,
            );
            let (bad_ncn_epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &bad_ncn,
                epoch_to_close,
            );
            let (good_epoch_state, _, _) = EpochState::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch_to_close,
            );

            let bad_epoch_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_epoch_epoch_state, None)
                .await;

            let bad_ncn_result = tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, bad_ncn_epoch_state, None)
                .await;

            assert!(bad_epoch_result.is_err());
            assert!(bad_ncn_result.is_err());

            tip_router_client
                .do_close_epoch_account(ncn, epoch_to_close, good_epoch_state, None)
                .await?;
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
