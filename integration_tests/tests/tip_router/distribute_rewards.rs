#[cfg(test)]
mod tests {

    use jito_tip_router_core::{
        base_fee_group::BaseFeeGroup,
        base_reward_router::BaseRewardReceiver,
        constants::{JITOSOL_MINT, MAX_OPERATORS, MAX_VAULTS},
        ncn_fee_group::{NcnFeeGroup, NcnFeeGroupType},
    };
    use solana_sdk::{clock::DEFAULT_SLOTS_PER_EPOCH, signature::Keypair, signer::Signer};

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_route_and_distribute_base_rewards() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut vault_client = fixture.vault_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

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
        fixture.add_routers_for_test_ncn(&test_ncn).await?;

        // Get initial balances
        let ncn_config = tip_router_client.get_ncn_config(ncn).await?;
        let epoch = fixture.clock().await.epoch;

        let dao_initial_lst_balance = fixture
            .get_associated_token_account(
                &ncn_config
                    .fee_config
                    .base_fee_wallet(BaseFeeGroup::default())
                    .unwrap(),
                &JITOSOL_MINT,
            )
            .await?
            .map_or(0, |account| account.amount);

        let operator_1_initial_balance = fixture
            .get_associated_token_account(&test_ncn.operators[0].operator_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);

        let operator_2_initial_balance = fixture
            .get_associated_token_account(&test_ncn.operators[1].operator_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);

        let vault_1_initial_balance = fixture
            .get_associated_token_account(&test_ncn.vaults[0].vault_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);

        let vault_2_initial_balance = fixture
            .get_associated_token_account(&test_ncn.vaults[1].vault_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);

        // Route in 3_000 lamports to the base reward receiver
        let (base_reward_receiver, _, _) =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        tip_router_client
            .airdrop_lamports(&base_reward_receiver, 3000)
            .await?;

        let valid_slots_after_consensus = {
            let config = tip_router_client.get_ncn_config(ncn).await?;
            config.valid_slots_after_consensus()
        };

        fixture
            .warp_slot_incremental(valid_slots_after_consensus + 1)
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

        stake_pool_client
            .update_stake_pool_balance(&pool_root)
            .await?;

        // Distribute base rewards (DAO fee)
        tip_router_client
            .do_distribute_base_rewards(BaseFeeGroup::default(), ncn, epoch, &pool_root)
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
        let dao_final_lst_balance = fixture
            .get_associated_token_account(
                &ncn_config
                    .fee_config
                    .base_fee_wallet(BaseFeeGroup::default())
                    .unwrap(),
                &JITOSOL_MINT,
            )
            .await?
            .map_or(0, |account| account.amount);

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
                        .do_distribute_ncn_operator_rewards(
                            *group, operator, ncn, epoch, &pool_root,
                        )
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
                                    *group, vault, operator, ncn, epoch, &pool_root,
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

        let dao_reward = dao_final_lst_balance - dao_initial_lst_balance;
        assert_eq!(dao_reward, 2_700);

        // NCN Reward Routes
        assert_eq!(*operator_total_rewards.first().unwrap(), 150);
        assert_eq!(*operator_total_rewards.get(1).unwrap(), 150);

        // Operator 1 Rewards
        let operator_1_final_balance = fixture
            .get_associated_token_account(&test_ncn.operators[0].operator_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);
        let operator_1_reward = operator_1_final_balance - operator_1_initial_balance;
        assert_eq!(operator_1_reward, 14);

        // Operator 2 Rewards
        let operator_2_final_balance = fixture
            .get_associated_token_account(&test_ncn.operators[1].operator_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);
        let operator_2_reward = operator_2_final_balance - operator_2_initial_balance;
        assert_eq!(operator_2_reward, 14);

        // Vault 1 Rewards
        let vault_1_final_balance = fixture
            .get_associated_token_account(&test_ncn.vaults[0].vault_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);
        let vault_1_reward = vault_1_final_balance - vault_1_initial_balance;
        assert_eq!(vault_1_reward, 136);

        // Vault 2 Rewards
        let vault_2_final_balance = fixture
            .get_associated_token_account(&test_ncn.vaults[1].vault_pubkey, &JITOSOL_MINT)
            .await?
            .map_or(0, |account| account.amount);
        let vault_2_reward = vault_2_final_balance - vault_2_initial_balance;
        assert_eq!(vault_2_reward, 136);

        Ok(())
    }

    #[ignore = "20-30 minute test"]
    #[tokio::test]
    async fn test_route_rewards_to_max_accounts() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut stake_pool_client = fixture.stake_pool_client();
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        let operator_count = MAX_OPERATORS;
        let vault_count = MAX_VAULTS;
        let should_distribute = true;
        let sol_rewards = 100.0;

        // Setup with 2 operators for interesting reward splits
        // 10% Operator fee
        let test_ncn = fixture
            .create_initial_test_ncn(operator_count, vault_count, Some(1000))
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

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

        // Set all Base fee groups to something
        for group in BaseFeeGroup::all_groups().iter() {
            tip_router_client
                .do_set_config_fees(
                    None,
                    Some(*group),
                    Some(dao_wallet_address),
                    Some(15),
                    None,
                    None,
                    &test_ncn.ncn_root,
                )
                .await?;
        }

        // Set all NCN fee groups to something
        for group in NcnFeeGroup::all_groups().iter() {
            tip_router_client
                .do_set_config_fees(
                    None,
                    None,
                    None,
                    None,
                    Some(*group),
                    Some(15), // NCN fee = .15%
                    &test_ncn.ncn_root,
                )
                .await?;
        }

        // Set all vaults to a different type of reward
        let vault_registry = tip_router_client.get_vault_registry(ncn).await?;
        for (index, mint_entry) in vault_registry.get_valid_mint_entries().iter().enumerate() {
            let group_index = index % NcnFeeGroup::all_groups().len();

            tip_router_client
                .do_admin_set_st_mint(
                    ncn,
                    *mint_entry.st_mint(),
                    Some(NcnFeeGroup::all_groups()[group_index]),
                    None,
                    None,
                    None,
                )
                .await?;
        }

        fixture
            .warp_slot_incremental(DEFAULT_SLOTS_PER_EPOCH * 2)
            .await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;

        // Initialize the routers
        fixture.add_routers_for_test_ncn(&test_ncn).await?;

        // Get initial balances
        let epoch = fixture.clock().await.epoch;

        // Route in 3_000 lamports
        let (base_reward_receiver, _, _) =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        // send rewards to the base reward router
        tip_router_client
            .airdrop(&base_reward_receiver, sol_rewards)
            .await?;

        // Check that ballot box and router are full to simulate the max cu
        let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;
        let vote_count = ballot_box
            .operator_votes()
            .iter()
            .filter(|v| !v.is_empty())
            .count();
        assert_eq!(vote_count, operator_count);

        // Do routing
        tip_router_client.do_route_base_rewards(ncn, epoch).await?;

        // Check that all routes have rewards
        let base_reward_router = tip_router_client.get_base_reward_router(ncn, epoch).await?;

        let ncn_fee_group_reward_routes = base_reward_router.ncn_fee_group_reward_routes();
        for route in ncn_fee_group_reward_routes.iter() {
            assert!(!route.is_empty());
            assert!(route.has_rewards().unwrap());
        }

        // Distribute base rewards (DAO fee)
        if should_distribute {
            for group in BaseFeeGroup::all_groups().iter() {
                tip_router_client
                    .do_distribute_base_rewards(*group, ncn, epoch, &pool_root)
                    .await?;
            }
        }

        // Route base NCN rewards (operator rewards)
        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            for group in NcnFeeGroup::all_groups().iter() {
                tip_router_client
                    .do_distribute_base_ncn_reward_route(*group, operator, ncn, epoch)
                    .await?;

                // Check max operator stake weights
                let operator_snapshot = tip_router_client
                    .get_operator_snapshot(operator, ncn, epoch)
                    .await?;
                let vault_operator_stake_weights = operator_snapshot.vault_operator_stake_weight();
                for vault_operator_stake_weight in vault_operator_stake_weights.iter() {
                    assert!(!vault_operator_stake_weight.is_empty())
                }

                tip_router_client
                    .do_route_ncn_rewards(*group, ncn, operator, epoch)
                    .await?;

                // Check that the reward router is full
                let ncn_reward_router = tip_router_client
                    .get_ncn_reward_router(*group, operator, ncn, epoch)
                    .await?;

                let mut route_count: u16 = 0;
                let mut reward_count: u16 = 0;
                for route in ncn_reward_router.vault_reward_routes().iter() {
                    if !route.is_empty() {
                        route_count += 1;
                    }

                    if route.has_rewards() {
                        reward_count += 1;
                    }
                }
                assert_eq!(route_count, reward_count);

                if should_distribute {
                    // Distribute to operators
                    tip_router_client
                        .do_distribute_ncn_operator_rewards(
                            *group, operator, ncn, epoch, &pool_root,
                        )
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
                                    *group, vault, operator, ncn, epoch, &pool_root,
                                )
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
