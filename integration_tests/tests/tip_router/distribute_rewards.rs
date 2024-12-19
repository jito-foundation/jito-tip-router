#[cfg(test)]
mod tests {

    use jito_tip_router_core::{
        base_fee_group::BaseFeeGroup,
        base_reward_router::BaseRewardRouter,
        ncn_fee_group::{NcnFeeGroup, NcnFeeGroupType},
    };
    use solana_sdk::{
        clock::DEFAULT_SLOTS_PER_EPOCH, native_token::lamports_to_sol, signature::Keypair,
        signer::Signer,
    };

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_route_and_distribute_base_rewards() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut vault_client = fixture.vault_client();

        // Setup with 2 operators for interesting reward splits
        // 10% Operator fee
        let test_ncn = fixture.create_initial_test_ncn(2, 2, Some(1000)).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        let dao_wallet = Keypair::new();
        let dao_wallet_address = dao_wallet.pubkey();
        tip_router_client.airdrop(&dao_wallet_address, 1.0).await?;

        // Configure fees: 30% block engine, 27% DAO fee, 1.5% NCN fee
        tip_router_client
            .do_set_config_fees(
                Some(300), // block engine fee = 3%
                None,
                Some(dao_wallet_address), // DAO wallet
                Some(270),                // DAO fee = 2.7%
                None,
                Some(15), // NCN fee = .15%
                &test_ncn.ncn_root,
            )
            .await?;

        tip_router_client
            .do_set_config_fees(
                None,
                None,
                None,
                None,
                Some(NcnFeeGroup::new(NcnFeeGroupType::JTO)),
                Some(15), // NCN fee = .15%
                &test_ncn.ncn_root,
            )
            .await?;

        // // Set tracked mint NCN fee group
        // let epoch = fixture.clock().await.epoch;
        // tip_router_client
        //     .do_admin_set_st_mint(
        //         test_ncn.ncn_root.ncn_pubkey,
        //         1,
        //         NcnFeeGroup::new(NcnFeeGroupType::JTO),
        //         epoch,
        //     )
        //     .await?;
        let vault = vault_client
            .get_vault(&test_ncn.vaults[1].vault_pubkey)
            .await?;
        let st_mint = vault.supported_mint;

        tip_router_client
            .do_admin_set_st_mint(
                test_ncn.ncn_root.ncn_pubkey,
                st_mint,
                Some(NcnFeeGroup::jto()),
                None,
                None,
                None,
            )
            .await?;

        fixture
            .warp_slot_incremental(DEFAULT_SLOTS_PER_EPOCH * 2)
            .await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;

        //////
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Initialize the routers
        fixture.add_routers_for_tests_ncn(&test_ncn).await?;

        // Get initial balances
        let ncn_config = tip_router_client.get_ncn_config(ncn).await?;
        let epoch = fixture.clock().await.epoch;

        let dao_initial_balance = fixture
            .get_balance(
                &ncn_config
                    .fee_config
                    .base_fee_wallet(BaseFeeGroup::default())
                    .unwrap(),
            )
            .await?;

        let operator_1_initial_balance = fixture
            .get_balance(&test_ncn.operators[0].operator_pubkey)
            .await?;

        let operator_2_initial_balance = fixture
            .get_balance(&test_ncn.operators[1].operator_pubkey)
            .await?;

        let vault_1_initial_balance = fixture
            .get_balance(&test_ncn.vaults[0].vault_pubkey)
            .await?;

        let vault_2_initial_balance = fixture
            .get_balance(&test_ncn.vaults[1].vault_pubkey)
            .await?;

        // Route in 3_000 lamports
        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        // Send rewards to base reward router
        let sol_rewards = lamports_to_sol(3_000);
        // send rewards to the base reward router
        tip_router_client
            .airdrop(&base_reward_router, sol_rewards)
            .await?;

        // Route rewards
        tip_router_client.do_route_base_rewards(ncn, epoch).await?;

        // Check Rewards
        let base_reward_router_account =
            tip_router_client.get_base_reward_router(ncn, epoch).await?;

        let dao_rewards = base_reward_router_account
            .base_fee_group_reward(BaseFeeGroup::default())
            .unwrap();
        assert_eq!(dao_rewards, 2_700);

        let operator_1_rewards = {
            let reward_route = base_reward_router_account
                .ncn_fee_group_reward_route(&test_ncn.operators[0].operator_pubkey)
                .unwrap();
            let lst_rewards = reward_route.rewards(NcnFeeGroup::default()).unwrap();
            let jto_rewards = reward_route
                .rewards(NcnFeeGroup::new(NcnFeeGroupType::JTO))
                .unwrap();

            lst_rewards + jto_rewards
        };
        assert_eq!(operator_1_rewards, 150);

        let operator_2_rewards = {
            let reward_route = base_reward_router_account
                .ncn_fee_group_reward_route(&test_ncn.operators[1].operator_pubkey)
                .unwrap();
            let lst_rewards = reward_route.rewards(NcnFeeGroup::default()).unwrap();
            let jto_rewards = reward_route
                .rewards(NcnFeeGroup::new(NcnFeeGroupType::JTO))
                .unwrap();

            lst_rewards + jto_rewards
        };
        assert_eq!(operator_2_rewards, 150);

        // Distribute base rewards (DAO fee)
        tip_router_client
            .do_distribute_base_rewards(BaseFeeGroup::default(), ncn, epoch)
            .await?;

        // Distribute base NCN rewards (operator rewards)
        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            for group in NcnFeeGroup::all_groups().iter() {
                tip_router_client
                    .do_distribute_base_ncn_reward_route(*group, operator, ncn, epoch)
                    .await?;
            }
        }

        // Get final balances
        let dao_final_balance = fixture
            .get_balance(
                &ncn_config
                    .fee_config
                    .base_fee_wallet(BaseFeeGroup::default())
                    .unwrap(),
            )
            .await?;

        let operator_total_rewards = {
            let mut operator_total_rewards = Vec::new();

            for operator_root in test_ncn.operators.iter() {
                let operator = operator_root.operator_pubkey;

                let mut total_rewards = 0;
                for group in NcnFeeGroup::all_groups().iter() {
                    tip_router_client
                        .do_route_ncn_rewards(*group, ncn, operator, epoch)
                        .await?;

                    // Distribute to operators
                    tip_router_client
                        .do_distribute_ncn_operator_rewards(*group, operator, ncn, epoch)
                        .await?;

                    // Distribute to vaults
                    for vault_root in test_ncn.vaults.iter() {
                        let vault = vault_root.vault_pubkey;

                        {
                            let ncn_reward_router = tip_router_client
                                .get_ncn_reward_router(*group, operator, ncn, epoch)
                                .await?;

                            // Skip if the vault is not in the reward route
                            if ncn_reward_router.vault_reward_route(&vault).is_err() {
                                continue;
                            }

                            tip_router_client
                                .do_distribute_ncn_vault_rewards(
                                    *group, vault, operator, ncn, epoch,
                                )
                                .await?;
                        }
                    }

                    let ncn_router = tip_router_client
                        .get_ncn_reward_router(*group, operator, ncn, epoch)
                        .await?;

                    total_rewards += ncn_router.total_rewards();
                }

                operator_total_rewards.push(total_rewards);
            }
            operator_total_rewards
        };

        // Check reward distributions
        // 3_000 lamports in rewards
        // total fee_bps = 270 + 15 + 15 = 300
        // DAO = 270 -> 2700
        // LST = 15 -> 150
        // JTO = 15 -> 150

        let dao_reward = dao_final_balance - dao_initial_balance;
        assert_eq!(dao_reward, 2_700);

        // NCN Reward Routes
        assert_eq!(*operator_total_rewards.first().unwrap(), 150);
        assert_eq!(*operator_total_rewards.get(1).unwrap(), 150);

        // Operator 1 Rewards
        let operator_1_final_balance = fixture
            .get_balance(&test_ncn.operators[0].operator_pubkey)
            .await?;
        let operator_1_reward = operator_1_final_balance - operator_1_initial_balance;
        assert_eq!(operator_1_reward, 14);

        // Operator 2 Rewards
        let operator_2_final_balance = fixture
            .get_balance(&test_ncn.operators[1].operator_pubkey)
            .await?;
        let operator_2_reward = operator_2_final_balance - operator_2_initial_balance;
        assert_eq!(operator_2_reward, 14);

        // Vault 1 Rewards
        let vault_1_final_balance = fixture
            .get_balance(&test_ncn.vaults[0].vault_pubkey)
            .await?;
        let vault_1_reward = vault_1_final_balance - vault_1_initial_balance;
        assert_eq!(vault_1_reward, 136);

        // Vault 2 Rewards
        let vault_2_final_balance = fixture
            .get_balance(&test_ncn.vaults[1].vault_pubkey)
            .await?;
        let vault_1_reward = vault_2_final_balance - vault_2_initial_balance;
        assert_eq!(vault_1_reward, 136);

        Ok(())
    }
}
