#[cfg(test)]
mod tests {

    use jito_restaking_core::MAX_FEE_BPS;
    use jito_tip_router_core::{
        constants::JITOSOL_MINT, error::TipRouterError, ncn_fee_group::NcnFeeGroup,
    };

    use crate::fixtures::{
        test_builder::TestBuilder, tip_router_client::assert_tip_router_error, TestResult,
    };

    #[tokio::test]
    async fn test_removing_operator() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut restaking_client = fixture.restaking_program_client();
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 3;
        const VAULT_COUNT: usize = 1;
        const OPERATOR_FEE_BPS: u16 = MAX_FEE_BPS;
        const AMOUNT_TO_REWARD: u64 = 50_000 * OPERATOR_COUNT as u64;
        const INDEX_OF_OPERATOR_TO_REMOVE: usize = 1;

        let expected_active_operator_balance_run_1 = AMOUNT_TO_REWARD / OPERATOR_COUNT as u64;
        let expected_active_operator_balance_run_2 = expected_active_operator_balance_run_1
            + (AMOUNT_TO_REWARD / (OPERATOR_COUNT - 1) as u64);

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(OPERATOR_FEE_BPS))
            .await?;

        {
            // Set all fees to 0
            tip_router_client
                .do_set_config_fees(
                    Some(0),
                    None,
                    None,
                    Some(0),
                    None,
                    Some(1),
                    &test_ncn.ncn_root,
                )
                .await?;

            fixture.warp_epoch_incremental(2).await?;
        }

        {
            // First Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;

            fixture
                .reward_test_ncn(&test_ncn, AMOUNT_TO_REWARD, &pool_root)
                .await?;

            let clock = fixture.clock().await;
            let epoch = clock.epoch;

            let router = tip_router_client
                .get_base_reward_router(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            assert_eq!(router.total_rewards(), AMOUNT_TO_REWARD);

            for operator_root in &test_ncn.operators {
                let operator = operator_root.operator_pubkey;

                let router = tip_router_client
                    .get_ncn_reward_router(
                        NcnFeeGroup::default(),
                        operator,
                        test_ncn.ncn_root.ncn_pubkey,
                        epoch,
                    )
                    .await?;

                let operator_balance = fixture
                    .get_associated_token_account(&operator, &JITOSOL_MINT)
                    .await?
                    .map_or(0, |account| account.amount);

                assert_eq!(router.total_rewards(), operator_balance);
                assert_eq!(operator_balance, expected_active_operator_balance_run_1);
            }
        }

        {
            // Remove an operator
            let operator = test_ncn.operators[INDEX_OF_OPERATOR_TO_REMOVE].operator_pubkey;
            restaking_client
                .do_ncn_cooldown_operator(&test_ncn.ncn_root, &operator)
                .await?;

            // Warp to next epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        {
            // Second Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;

            fixture
                .reward_test_ncn(&test_ncn, AMOUNT_TO_REWARD, &pool_root)
                .await?;

            let clock = fixture.clock().await;
            let epoch = clock.epoch;

            let router = tip_router_client
                .get_base_reward_router(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            assert_eq!(router.total_rewards(), AMOUNT_TO_REWARD);

            for (index, operator_root) in test_ncn.operators.iter().enumerate() {
                let operator = operator_root.operator_pubkey;

                let operator_balance = fixture
                    .get_associated_token_account(&operator, &JITOSOL_MINT)
                    .await?
                    .map_or(0, |account| account.amount);

                if index == INDEX_OF_OPERATOR_TO_REMOVE {
                    assert_eq!(operator_balance, expected_active_operator_balance_run_1);
                } else {
                    assert_eq!(operator_balance, expected_active_operator_balance_run_2);
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_removing_vault() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut restaking_client = fixture.restaking_program_client();
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 3;
        const AMOUNT_TO_REWARD: u64 = 50_000 * VAULT_COUNT as u64;
        const INDEX_OF_VAULT_TO_REMOVE: usize = 1;

        let expected_active_vault_balance_run_1 = AMOUNT_TO_REWARD / VAULT_COUNT as u64;
        let expected_active_vault_balance_run_2 =
            expected_active_vault_balance_run_1 + (AMOUNT_TO_REWARD / (VAULT_COUNT - 1) as u64);

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(0))
            .await?;

        {
            // Set all fees to 0
            tip_router_client
                .do_set_config_fees(
                    Some(0),
                    None,
                    None,
                    Some(0),
                    None,
                    Some(1),
                    &test_ncn.ncn_root,
                )
                .await?;

            fixture.warp_epoch_incremental(2).await?;
        }

        {
            // First Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;

            fixture
                .reward_test_ncn(&test_ncn, AMOUNT_TO_REWARD, &pool_root)
                .await?;

            let clock = fixture.clock().await;
            let epoch = clock.epoch;

            let router = tip_router_client
                .get_base_reward_router(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            assert_eq!(router.total_rewards(), AMOUNT_TO_REWARD);

            for vault_root in &test_ncn.vaults {
                let vault = vault_root.vault_pubkey;

                let vault_balance = fixture
                    .get_associated_token_account(&vault, &JITOSOL_MINT)
                    .await?
                    .map_or(0, |account| account.amount);

                assert_eq!(vault_balance, expected_active_vault_balance_run_1);
            }
        }

        {
            // Remove a vault
            let vault = test_ncn.vaults[INDEX_OF_VAULT_TO_REMOVE].vault_pubkey;
            restaking_client
                .do_cooldown_ncn_vault_ticket(&test_ncn.ncn_root, &vault)
                .await?;

            // Warp to next epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        {
            // Second Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;

            fixture
                .reward_test_ncn(&test_ncn, AMOUNT_TO_REWARD, &pool_root)
                .await?;

            let clock = fixture.clock().await;
            let epoch = clock.epoch;

            let router = tip_router_client
                .get_base_reward_router(test_ncn.ncn_root.ncn_pubkey, epoch)
                .await?;

            assert_eq!(router.total_rewards(), AMOUNT_TO_REWARD);

            for (index, vault_root) in test_ncn.vaults.iter().enumerate() {
                let vault = vault_root.vault_pubkey;

                let vault_balance = fixture
                    .get_associated_token_account(&vault, &JITOSOL_MINT)
                    .await?
                    .map_or(0, |account| account.amount);

                if index == INDEX_OF_VAULT_TO_REMOVE {
                    assert_eq!(vault_balance, expected_active_vault_balance_run_1);
                } else {
                    assert_eq!(vault_balance, expected_active_vault_balance_run_2);
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_stale_vault() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(0))
            .await?;

        stake_pool_client.do_initialize_stake_pool().await?;

        {
            // Fast forward to a new epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;
        {
            let epoch = fixture.clock().await.epoch;
            let ncn = test_ncn.ncn_root.ncn_pubkey;

            let operator = test_ncn.operators[0].operator_pubkey;
            let vault = test_ncn.vaults[0].vault_pubkey;

            let result = tip_router_client
                .do_snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
                .await;

            assert_tip_router_error(result, TipRouterError::VaultNeedsUpdate);
        }

        Ok(())
    }
}
