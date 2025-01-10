#[cfg(test)]
mod tests {

    use jito_tip_router_core::ncn_fee_group::NcnFeeGroup;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_all_test_ncn_functions_pt1() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 2;
        const VAULT_COUNT: usize = 3;

        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = fixture.clock().await.epoch;

        {
            fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;
            assert_eq!(epoch_state.epoch(), epoch);
        }

        {
            fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;
            assert!(epoch_state.set_weight_progress.is_complete());
            assert_eq!(epoch_state.set_weight_progress.tally(), VAULT_COUNT as u64);
            assert_eq!(epoch_state.set_weight_progress.total(), VAULT_COUNT as u64);
            assert_eq!(epoch_state.vault_count(), VAULT_COUNT as u64);
        }

        {
            fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;
            assert_eq!(epoch_state.operator_count(), OPERATOR_COUNT as u64);
            assert!(!epoch_state.epoch_snapshot_progress.is_invalid());
        }

        {
            fixture
                .add_operator_snapshots_to_test_ncn(&test_ncn)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            for i in 0..OPERATOR_COUNT {
                assert_eq!(epoch_state.operator_snapshot_progress[i].tally(), 0);
                assert_eq!(
                    epoch_state.operator_snapshot_progress[i].total(),
                    VAULT_COUNT as u64
                );
            }
        }

        {
            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert!(epoch_state.epoch_snapshot_progress.is_complete());
            assert_eq!(
                epoch_state.epoch_snapshot_progress.tally(),
                OPERATOR_COUNT as u64
            );
            assert_eq!(
                epoch_state.epoch_snapshot_progress.total(),
                OPERATOR_COUNT as u64
            );

            for i in 0..OPERATOR_COUNT {
                assert_eq!(
                    epoch_state.operator_snapshot_progress[i].tally(),
                    OPERATOR_COUNT as u64
                );
                assert_eq!(
                    epoch_state.operator_snapshot_progress[i].total(),
                    OPERATOR_COUNT as u64
                );
                assert!(epoch_state.operator_snapshot_progress[i].is_complete());
            }
        }

        {
            fixture.add_ballot_box_to_test_ncn(&test_ncn).await?;
            fixture.cast_votes_for_test_ncn(&test_ncn).await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert!(epoch_state.voting_progress.is_complete());
        }

        {
            fixture.add_routers_for_tests_ncn(&test_ncn).await?;
            stake_pool_client
                .update_stake_pool_balance(&pool_root)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert!(epoch_state.total_distribution_progress.is_complete());
            assert!(epoch_state.base_distribution_progress.is_complete());

            for i in 0..OPERATOR_COUNT {
                for j in 0..NcnFeeGroup::FEE_GROUP_COUNT {
                    let index = i * NcnFeeGroup::FEE_GROUP_COUNT + j;
                    assert!(epoch_state.ncn_distribution_progress[index].is_complete());
                }
            }
        }

        {
            fixture
                .route_in_base_rewards_for_test_ncn(&test_ncn, 10_000, &pool_root)
                .await?;
            fixture
                .route_in_ncn_rewards_for_test_ncn(&test_ncn, &pool_root)
                .await?;
        }

        // To be continued... Running into stack overflow issues

        Ok(())
    }

    #[tokio::test]
    async fn test_all_test_ncn_functions_pt2() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 2;
        const VAULT_COUNT: usize = 3;

        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = fixture.clock().await.epoch;

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;

        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;

        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;

        fixture
            .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
            .await?;

        fixture.add_ballot_box_to_test_ncn(&test_ncn).await?;
        fixture.cast_votes_for_test_ncn(&test_ncn).await?;

        fixture.add_routers_for_tests_ncn(&test_ncn).await?;
        stake_pool_client
            .update_stake_pool_balance(&pool_root)
            .await?;

        {
            fixture
                .route_in_base_rewards_for_test_ncn(&test_ncn, 10_000, &pool_root)
                .await?;

            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            // assert_eq!(epoch_state.total_distribution_progress.tally(), 0);
            assert_eq!(epoch_state.total_distribution_progress.total(), 10_000);

            // Because no base rewards fees
            assert_eq!(epoch_state.base_distribution_progress.tally(), 10_000);
            assert_eq!(epoch_state.base_distribution_progress.total(), 10_000);

            assert!(!epoch_state.total_distribution_progress.is_complete());
            assert!(epoch_state.base_distribution_progress.is_complete());
        }

        {
            fixture
                .route_in_ncn_rewards_for_test_ncn(&test_ncn, &pool_root)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert_eq!(epoch_state.total_distribution_progress.total(), 10_000);
            assert_eq!(epoch_state.total_distribution_progress.tally(), 10_000);
            assert!(epoch_state.total_distribution_progress.is_complete());

            for i in 0..OPERATOR_COUNT {
                for j in 0..NcnFeeGroup::FEE_GROUP_COUNT {
                    let index = i * NcnFeeGroup::FEE_GROUP_COUNT + j;

                    assert!(epoch_state.ncn_distribution_progress[index].is_complete());
                }
            }
        }

        Ok(())
    }
}
