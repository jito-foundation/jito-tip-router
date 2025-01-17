#[cfg(test)]
mod tests {

    use jito_tip_router_core::{
        constants::MAX_OPERATORS, epoch_state::AccountStatus, ncn_fee_group::NcnFeeGroup,
    };

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_all_test_ncn_functions_pt1() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 2;
        const VAULT_COUNT: usize = 3;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(100))
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
            assert!(epoch_state.set_weight_progress().is_complete());
            assert_eq!(
                epoch_state.set_weight_progress().tally(),
                VAULT_COUNT as u64
            );
            assert_eq!(
                epoch_state.set_weight_progress().total(),
                VAULT_COUNT as u64
            );
            assert_eq!(epoch_state.vault_count(), VAULT_COUNT as u64);
        }

        {
            fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;
            assert_eq!(epoch_state.operator_count(), OPERATOR_COUNT as u64);
            assert!(!epoch_state.epoch_snapshot_progress().is_invalid());
        }

        {
            fixture
                .add_operator_snapshots_to_test_ncn(&test_ncn)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            for i in 0..OPERATOR_COUNT {
                assert_eq!(epoch_state.operator_snapshot_progress(i).tally(), 0);
                assert_eq!(
                    epoch_state.operator_snapshot_progress(i).total(),
                    VAULT_COUNT as u64
                );
            }
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
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(100))
            .await?;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = fixture.clock().await.epoch;

        {
            fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        }

        {
            fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;
        }

        {
            fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        }

        {
            fixture
                .add_operator_snapshots_to_test_ncn(&test_ncn)
                .await?;
        }

        {
            fixture
                .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert!(epoch_state.epoch_snapshot_progress().is_complete());
            assert_eq!(
                epoch_state.epoch_snapshot_progress().tally(),
                OPERATOR_COUNT as u64
            );
            assert_eq!(
                epoch_state.epoch_snapshot_progress().total(),
                OPERATOR_COUNT as u64
            );

            for i in 0..OPERATOR_COUNT {
                assert_eq!(
                    epoch_state.operator_snapshot_progress(i).tally(),
                    VAULT_COUNT as u64
                );
                assert_eq!(
                    epoch_state.operator_snapshot_progress(i).total(),
                    VAULT_COUNT as u64
                );
                assert!(epoch_state.operator_snapshot_progress(i).is_complete());
            }
        }

        {
            fixture.add_ballot_box_to_test_ncn(&test_ncn).await?;
            fixture.cast_votes_for_test_ncn(&test_ncn).await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert!(epoch_state.voting_progress().is_complete());
        }

        {
            fixture.add_routers_for_tests_ncn(&test_ncn).await?;
            stake_pool_client
                .update_stake_pool_balance(&pool_root)
                .await?;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert!(epoch_state.total_distribution_progress().is_complete());
            assert!(epoch_state.base_distribution_progress().is_complete());

            for i in 0..OPERATOR_COUNT {
                for group in NcnFeeGroup::all_groups() {
                    assert!(epoch_state
                        .ncn_distribution_progress(i, group)
                        .is_complete());
                }
            }
        }

        // To be continued... Running into stack overflow issues

        Ok(())
    }

    #[tokio::test]
    async fn test_all_test_ncn_functions_pt3() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 2;
        const VAULT_COUNT: usize = 3;
        const OPERATOR_FEE_BPS: u16 = 1000;
        const BASE_FEE_BPS: u16 = 1000;
        const NCN_FEE_BPS: u16 = 1000;
        const TOTAL_REWARDS: u64 = 10_000;

        let expected_base_rewards = 5000;
        let expected_ncn_rewards = 5000;
        let expected_operator_router_rewards = expected_ncn_rewards / OPERATOR_COUNT as u64;
        let _expected_operator_rewards = 500 / OPERATOR_COUNT as u64;
        let _expected_vault_rewards =
            (expected_ncn_rewards - _expected_operator_rewards) / VAULT_COUNT as u64;

        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        let test_ncn = fixture
            .create_custom_initial_test_ncn(
                OPERATOR_COUNT,
                VAULT_COUNT,
                OPERATOR_FEE_BPS,
                BASE_FEE_BPS,
                NCN_FEE_BPS,
            )
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
                .route_in_base_rewards_for_test_ncn(&test_ncn, TOTAL_REWARDS, &pool_root)
                .await?;

            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert_eq!(
                epoch_state.total_distribution_progress().tally(),
                expected_base_rewards
            );
            assert_eq!(
                epoch_state.total_distribution_progress().total(),
                TOTAL_REWARDS
            );

            assert_eq!(
                epoch_state.base_distribution_progress().tally(),
                TOTAL_REWARDS
            );
            assert_eq!(
                epoch_state.base_distribution_progress().total(),
                TOTAL_REWARDS
            );

            assert!(!epoch_state.total_distribution_progress().is_complete());
            assert!(epoch_state.base_distribution_progress().is_complete());
        }

        {
            fixture
                .route_in_ncn_rewards_for_test_ncn(&test_ncn, &pool_root)
                .await?;

            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;

            assert_eq!(
                epoch_state.total_distribution_progress().total(),
                TOTAL_REWARDS
            );
            assert_eq!(
                epoch_state.total_distribution_progress().tally(),
                TOTAL_REWARDS
            );
            assert!(epoch_state.total_distribution_progress().is_complete());

            for i in 0..OPERATOR_COUNT {
                for group in NcnFeeGroup::all_groups() {
                    // We only use the first operator and fee group
                    if group != NcnFeeGroup::default() {
                        continue;
                    }

                    assert_eq!(
                        epoch_state.ncn_distribution_progress(i, group).total(),
                        expected_operator_router_rewards
                    );
                    assert_eq!(
                        epoch_state.ncn_distribution_progress(i, group).tally(),
                        expected_operator_router_rewards
                    );
                    assert!(epoch_state
                        .ncn_distribution_progress(i, group)
                        .is_complete());
                }
            }

            {
                // Test all accounts are "created"

                assert_eq!(
                    epoch_state.account_status().epoch_state().unwrap(),
                    AccountStatus::Created
                );
                assert_eq!(
                    epoch_state.account_status().weight_table().unwrap(),
                    AccountStatus::Created
                );
                assert_eq!(
                    epoch_state.account_status().epoch_snapshot().unwrap(),
                    AccountStatus::Created
                );
                for i in 0..MAX_OPERATORS {
                    if i < OPERATOR_COUNT {
                        assert_eq!(
                            epoch_state.account_status().operator_snapshot(i).unwrap(),
                            AccountStatus::Created
                        );
                    } else {
                        assert_eq!(
                            epoch_state.account_status().operator_snapshot(i).unwrap(),
                            AccountStatus::DNE
                        );
                    }
                }
                assert_eq!(
                    epoch_state.account_status().ballot_box().unwrap(),
                    AccountStatus::Created
                );
                assert_eq!(
                    epoch_state.account_status().base_reward_router().unwrap(),
                    AccountStatus::Created
                );
                for i in 0..MAX_OPERATORS {
                    for group in NcnFeeGroup::all_groups() {
                        if i < OPERATOR_COUNT {
                            assert_eq!(
                                epoch_state
                                    .account_status()
                                    .ncn_reward_router(i, group)
                                    .unwrap(),
                                AccountStatus::Created
                            );
                        } else {
                            assert_eq!(
                                epoch_state
                                    .account_status()
                                    .ncn_reward_router(i, group)
                                    .unwrap(),
                                AccountStatus::DNE
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
