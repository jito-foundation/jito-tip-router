#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use jito_restaking_core::{config::Config, ncn_vault_ticket::NcnVaultTicket};
    use jito_tip_router_core::{
        base_fee_group::BaseFeeGroup,
        constants::{JITOSOL_SOL_FEED, JTO_SOL_FEED, MAX_OPERATORS, WEIGHT_PRECISION},
        ncn_fee_group::NcnFeeGroup,
    };
    use solana_sdk::{
        native_token::sol_to_lamports, pubkey::Pubkey, signature::Keypair, signer::Signer,
    };

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[ignore = "20-30 minute test"]
    #[tokio::test]
    async fn simulation_test() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();
        let mut vault_program_client = fixture.vault_client();
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 13;
        let base_fee_wallet: Pubkey =
            Pubkey::from_str("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF").unwrap();

        tip_router_client.airdrop(&base_fee_wallet, 1.0).await?;

        let mints = vec![
            (
                Keypair::new(),
                20_000,
                Some(JITOSOL_SOL_FEED),
                None,
                NcnFeeGroup::lst(),
            ), // JitoSOL
            (
                Keypair::new(),
                10_000,
                Some(JTO_SOL_FEED),
                None,
                NcnFeeGroup::jto(),
            ), // JTO
            (
                Keypair::new(),
                10_000,
                Some(JITOSOL_SOL_FEED),
                None,
                NcnFeeGroup::lst(),
            ), // BnSOL
            (
                Keypair::new(),
                7_000,
                None,
                Some(1 * WEIGHT_PRECISION),
                NcnFeeGroup::lst(),
            ), // nSol
        ];

        let delegations = vec![
            1,
            sol_to_lamports(1000.0),
            sol_to_lamports(10000.0),
            sol_to_lamports(100000.0),
            sol_to_lamports(1000000.0),
            sol_to_lamports(10000000.0),
        ];

        // Setup NCN
        let mut test_ncn = fixture.create_test_ncn().await?;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        // Set Fees
        {
            tip_router_client
                .do_set_config_fees(
                    Some(500),
                    Some(BaseFeeGroup::dao()),
                    Some(base_fee_wallet),
                    Some(270),
                    Some(NcnFeeGroup::lst()),
                    Some(15),
                    &test_ncn.ncn_root,
                )
                .await?;

            tip_router_client
                .do_set_config_fees(
                    None,
                    None,
                    None,
                    None,
                    Some(NcnFeeGroup::jto()),
                    Some(15),
                    &test_ncn.ncn_root,
                )
                .await?;

            fixture.warp_epoch_incremental(2).await?;
        }

        // Add operators and vaults
        {
            fixture
                .add_operators_to_test_ncn(&mut test_ncn, OPERATOR_COUNT, Some(100))
                .await?;
            // JitoSOL
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 3, Some(mints[0].0.insecure_clone()))
                .await?;
            // JTO
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 2, Some(mints[1].0.insecure_clone()))
                .await?;
            // BnSOL
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 1, Some(mints[2].0.insecure_clone()))
                .await?;
            // nSol
            fixture
                .add_vaults_to_test_ncn(&mut test_ncn, 1, Some(mints[3].0.insecure_clone()))
                .await?;
        }

        // Add delegation
        {
            let mut index = 0;
            for operator_root in test_ncn.operators.iter().take(OPERATOR_COUNT - 1) {
                // for operator_root in test_ncn.operators.iter() {
                for vault_root in test_ncn.vaults.iter() {
                    let delegation_amount = delegations[index % delegations.len()];

                    if delegation_amount > 0 {
                        vault_program_client
                            .do_add_delegation(
                                vault_root,
                                &operator_root.operator_pubkey,
                                delegation_amount as u64,
                            )
                            .await
                            .unwrap();
                    }
                }
                index += 1;
            }
        }

        // Register ST Mint
        {
            let restaking_config_address =
                Config::find_program_address(&jito_restaking_program::id()).0;
            let restaking_config = restaking_client
                .get_config(&restaking_config_address)
                .await?;

            let epoch_length = restaking_config.epoch_length();

            fixture
                .warp_slot_incremental(epoch_length * 2)
                .await
                .unwrap();

            for (mint, reward_multiplier_bps, switchboard_feed, no_feed_weight, group) in
                mints.iter()
            {
                tip_router_client
                    .do_admin_register_st_mint(
                        ncn,
                        mint.pubkey(),
                        *group,
                        *reward_multiplier_bps as u64,
                        *switchboard_feed,
                        *no_feed_weight,
                    )
                    .await?;
            }

            for vault in test_ncn.vaults.iter() {
                let vault = vault.vault_pubkey;
                let (ncn_vault_ticket, _, _) = NcnVaultTicket::find_program_address(
                    &jito_restaking_program::id(),
                    &ncn,
                    &vault,
                );

                tip_router_client
                    .do_register_vault(ncn, vault, ncn_vault_ticket)
                    .await?;
            }
        }

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture
            .add_switchboard_weights_for_test_ncn(&test_ncn)
            .await?;

        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;
        fixture
            .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
            .await?;
        fixture.add_ballot_box_to_test_ncn(&test_ncn).await?;

        // Cast votes
        {
            let epoch = fixture.clock().await.epoch;

            let zero_delegation_operator = test_ncn.operators.last().unwrap();
            let first_operator = &test_ncn.operators[0];
            let second_operator = &test_ncn.operators[1];
            let third_operator = &test_ncn.operators[2];

            for _ in 0..MAX_OPERATORS + 5 {
                let meta_merkle_root = Pubkey::new_unique().to_bytes();

                tip_router_client
                    .do_cast_vote(
                        ncn,
                        zero_delegation_operator.operator_pubkey,
                        &zero_delegation_operator.operator_admin,
                        meta_merkle_root,
                        epoch,
                    )
                    .await?;
            }

            let meta_merkle_root = Pubkey::new_unique().to_bytes();
            tip_router_client
                .do_cast_vote(
                    ncn,
                    zero_delegation_operator.operator_pubkey,
                    &zero_delegation_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            tip_router_client
                .do_cast_vote(
                    ncn,
                    first_operator.operator_pubkey,
                    &first_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            let meta_merkle_root = Pubkey::new_unique().to_bytes();
            tip_router_client
                .do_cast_vote(
                    ncn,
                    zero_delegation_operator.operator_pubkey,
                    &zero_delegation_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            tip_router_client
                .do_cast_vote(
                    ncn,
                    second_operator.operator_pubkey,
                    &second_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            tip_router_client
                .do_cast_vote(
                    ncn,
                    third_operator.operator_pubkey,
                    &third_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            let meta_merkle_root = Pubkey::new_unique().to_bytes();
            tip_router_client
                .do_cast_vote(
                    ncn,
                    zero_delegation_operator.operator_pubkey,
                    &zero_delegation_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            tip_router_client
                .do_cast_vote(
                    ncn,
                    first_operator.operator_pubkey,
                    &first_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            tip_router_client
                .do_cast_vote(
                    ncn,
                    second_operator.operator_pubkey,
                    &second_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            tip_router_client
                .do_cast_vote(
                    ncn,
                    third_operator.operator_pubkey,
                    &third_operator.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
            let meta_merkle_root = Pubkey::new_unique().to_bytes();
            for operator_root in test_ncn.operators.iter().take(OPERATOR_COUNT - 1) {
                let operator = operator_root.operator_pubkey;

                tip_router_client
                    .do_cast_vote(
                        ncn,
                        operator,
                        &operator_root.operator_admin,
                        meta_merkle_root,
                        epoch,
                    )
                    .await?;
            }

            let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;
            assert!(ballot_box.has_winning_ballot());
            assert!(ballot_box.is_consensus_reached());
            assert_eq!(
                ballot_box.get_winning_ballot().unwrap().root(),
                meta_merkle_root
            );
        }

        fixture.add_routers_for_test_ncn(&test_ncn).await?;
        stake_pool_client
            .update_stake_pool_balance(&pool_root)
            .await?;
        fixture
            .route_in_base_rewards_for_test_ncn(&test_ncn, 10_000, &pool_root)
            .await?;
        fixture
            .route_in_ncn_rewards_for_test_ncn(&test_ncn, &pool_root)
            .await?;
        fixture.close_epoch_accounts_for_test_ncn(&test_ncn).await?;

        Ok(())
    }
}

#[cfg(test)]
mod fuzz_tests {
    use crate::fixtures::{test_builder::TestBuilder, TestResult};
    use jito_restaking_core::{config::Config, ncn_vault_ticket::NcnVaultTicket};
    use jito_tip_router_core::{
        base_fee_group::BaseFeeGroup,
        constants::{JITOSOL_SOL_FEED, JTO_SOL_FEED, MAX_OPERATORS, WEIGHT_PRECISION},
        ncn_fee_group::NcnFeeGroup,
    };
    use solana_sdk::{
        native_token::sol_to_lamports, pubkey::Pubkey, signature::Keypair, signer::Signer,
    };
    use std::str::FromStr;

    struct MintConfig {
        keypair: Keypair,
        reward_multiplier: u64,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
        fee_group: NcnFeeGroup,
        vault_count: usize,
    }

    struct SimConfig {
        operator_count: usize,
        base_fee_wallet: Pubkey,
        mints: Vec<MintConfig>,
        delegations: Vec<u64>,
        base_engine_fee_bps: u16,
        dao_fee_bps: u16,
        operator_fee_bps: u16,
        lst_fee_bps: u16,
        jto_fee_bps: u16,
        rewards_amount: u64,
    }

    async fn run_simulation(config: SimConfig) -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();
        let mut vault_program_client = fixture.vault_client();
        let mut restaking_client = fixture.restaking_program_client();

        let total_vaults = config.mints.iter().map(|m| m.vault_count).sum::<usize>();
        assert_eq!(config.delegations.len(), total_vaults);

        tip_router_client
            .airdrop(&config.base_fee_wallet, 1.0)
            .await?;

        // Setup NCN
        let mut test_ncn = fixture.create_test_ncn().await?;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let pool_root = stake_pool_client.do_initialize_stake_pool().await?;

        // Set Fees
        {
            tip_router_client
                .do_set_config_fees(
                    Some(config.base_engine_fee_bps),
                    Some(BaseFeeGroup::dao()),
                    Some(config.base_fee_wallet),
                    Some(config.dao_fee_bps),
                    Some(NcnFeeGroup::lst()),
                    Some(config.lst_fee_bps),
                    &test_ncn.ncn_root,
                )
                .await?;

            tip_router_client
                .do_set_config_fees(
                    None,
                    None,
                    None,
                    None,
                    Some(NcnFeeGroup::jto()),
                    Some(config.jto_fee_bps),
                    &test_ncn.ncn_root,
                )
                .await?;

            fixture.warp_epoch_incremental(2).await?;
        }

        // Add operators and vaults
        {
            fixture
                .add_operators_to_test_ncn(
                    &mut test_ncn,
                    config.operator_count,
                    Some(config.operator_fee_bps),
                )
                .await?;

            for mint_config in config.mints.iter() {
                fixture
                    .add_vaults_to_test_ncn(
                        &mut test_ncn,
                        mint_config.vault_count,
                        Some(mint_config.keypair.insecure_clone()),
                    )
                    .await?;
            }
        }

        // Add delegation
        {
            let seed = Pubkey::new_unique()
                .to_bytes()
                .iter()
                .enumerate()
                .fold(0u64, |acc, (i, &byte)| {
                    acc.wrapping_add((byte as u64) << (i % 8 * 8))
                });

            for (vault_index, vault_root) in test_ncn.vaults.iter().enumerate() {
                let total_vault_delegation = config.delegations[vault_index];

                // Create a shuffled list of operators
                let mut operators: Vec<_> = test_ncn.operators.iter().collect();
                let shuffle_index = seed.wrapping_add(vault_index as u64);

                // Fisher-Yates shuffle
                for i in (1..operators.len()).rev() {
                    let j = (shuffle_index.wrapping_mul(i as u64) % (i as u64 + 1)) as usize;
                    operators.swap(i, j);
                }

                // Skip the first operator (effectively excluding them from delegation)
                let selected_operators = operators.iter().skip(1).take(config.operator_count - 2);
                let operator_count = config.operator_count - 2; // Reduced by one more to account for exclusion

                // Calculate per-operator delegation amount
                let delegation_per_operator = total_vault_delegation / operator_count as u64;

                if delegation_per_operator > 0 {
                    for operator_root in selected_operators {
                        vault_program_client
                            .do_add_delegation(
                                vault_root,
                                &operator_root.operator_pubkey,
                                delegation_per_operator,
                            )
                            .await
                            .unwrap();
                    }
                }
            }
        }

        // Register ST Mint
        {
            let restaking_config_address =
                Config::find_program_address(&jito_restaking_program::id()).0;
            let restaking_config = restaking_client
                .get_config(&restaking_config_address)
                .await?;
            let epoch_length = restaking_config.epoch_length();

            fixture.warp_slot_incremental(epoch_length * 2).await?;

            for mint_config in config.mints.iter() {
                tip_router_client
                    .do_admin_register_st_mint(
                        ncn,
                        mint_config.keypair.pubkey(),
                        mint_config.fee_group,
                        mint_config.reward_multiplier,
                        mint_config.switchboard_feed,
                        mint_config.no_feed_weight,
                    )
                    .await?;
            }

            for vault in test_ncn.vaults.iter() {
                let vault = vault.vault_pubkey;
                let (ncn_vault_ticket, _, _) = NcnVaultTicket::find_program_address(
                    &jito_restaking_program::id(),
                    &ncn,
                    &vault,
                );

                tip_router_client
                    .do_register_vault(ncn, vault, ncn_vault_ticket)
                    .await?;
            }
        }

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture
            .add_switchboard_weights_for_test_ncn(&test_ncn)
            .await?;

        {
            let epoch = fixture.clock().await.epoch;
            let epoch_state = tip_router_client.get_epoch_state(ncn, epoch).await?;
            assert!(epoch_state.set_weight_progress().is_complete())
        }

        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;
        fixture
            .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
            .await?;
        fixture.add_ballot_box_to_test_ncn(&test_ncn).await?;

        // Cast votes
        {
            let epoch = fixture.clock().await.epoch;

            // Do some random voting first
            let zero_delegation_operator = test_ncn.operators.last().unwrap();
            let vote_operators = &test_ncn.operators.clone(); // Take first few operators for random voting

            for _ in 0..MAX_OPERATORS + 55 {
                // Generate random merkle root
                let random_merkle_root = Pubkey::new_unique().to_bytes();
                let offset = random_merkle_root
                    .iter()
                    .map(|&x| x as usize)
                    .sum::<usize>();

                // Random operator votes for it
                let random_operator = &vote_operators[offset % vote_operators.len()];
                tip_router_client
                    .do_cast_vote(
                        ncn,
                        random_operator.operator_pubkey,
                        &random_operator.operator_admin,
                        random_merkle_root,
                        epoch,
                    )
                    .await?;

                // Zero delegation operator also votes
                tip_router_client
                    .do_cast_vote(
                        ncn,
                        zero_delegation_operator.operator_pubkey,
                        &zero_delegation_operator.operator_admin,
                        random_merkle_root,
                        epoch,
                    )
                    .await?;
            }

            // Then do the consensus vote
            let meta_merkle_root = Pubkey::new_unique().to_bytes();
            // First create a mutable copy of the operators that we can shuffle
            let mut operators_to_shuffle = test_ncn.operators.clone();

            // Use the merkle root bytes to create a deterministic shuffle
            let shuffle_seed: u64 = meta_merkle_root
                .iter()
                .enumerate()
                .fold(0u64, |acc, (i, &byte)| {
                    acc.wrapping_add((byte as u64) << (i % 8 * 8))
                });

            // Fisher-Yates shuffle using the seed
            for i in (1..operators_to_shuffle.len()).rev() {
                // Use the seed to generate a deterministic index
                let j = (shuffle_seed.wrapping_mul(i as u64) % (i as u64 + 1)) as usize;
                operators_to_shuffle.swap(i, j);
            }

            // Now use the shuffled operators
            for operator_root in operators_to_shuffle.iter() {
                let operator = operator_root.operator_pubkey;
                let _ = tip_router_client
                    .do_cast_vote(
                        ncn,
                        operator,
                        &operator_root.operator_admin,
                        meta_merkle_root,
                        epoch,
                    )
                    .await;
            }

            let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;
            assert!(ballot_box.has_winning_ballot());
            assert!(ballot_box.is_consensus_reached());
            assert_eq!(
                ballot_box.get_winning_ballot().unwrap().root(),
                meta_merkle_root
            );
        }

        fixture.add_routers_for_test_ncn(&test_ncn).await?;
        stake_pool_client
            .update_stake_pool_balance(&pool_root)
            .await?;
        fixture
            .route_in_base_rewards_for_test_ncn(&test_ncn, config.rewards_amount, &pool_root)
            .await?;
        fixture
            .route_in_ncn_rewards_for_test_ncn(&test_ncn, &pool_root)
            .await?;
        fixture.close_epoch_accounts_for_test_ncn(&test_ncn).await?;

        Ok(())
    }

    #[ignore = "20-30 minute test"]
    #[tokio::test]
    async fn test_basic_simulation() -> TestResult<()> {
        let base_fee_wallet =
            Pubkey::from_str("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF").unwrap();

        let config = SimConfig {
            operator_count: 13,
            base_fee_wallet,
            mints: vec![
                MintConfig {
                    keypair: Keypair::new(),
                    reward_multiplier: 20_000,
                    switchboard_feed: Some(JITOSOL_SOL_FEED),
                    no_feed_weight: None,
                    fee_group: NcnFeeGroup::lst(),
                    vault_count: 3,
                },
                MintConfig {
                    keypair: Keypair::new(),
                    reward_multiplier: 10_000,
                    switchboard_feed: Some(JTO_SOL_FEED),
                    no_feed_weight: None,
                    fee_group: NcnFeeGroup::jto(),
                    vault_count: 2,
                },
                MintConfig {
                    keypair: Keypair::new(),
                    reward_multiplier: 10_000,
                    switchboard_feed: Some(JITOSOL_SOL_FEED),
                    no_feed_weight: None,
                    fee_group: NcnFeeGroup::lst(),
                    vault_count: 1,
                },
                MintConfig {
                    keypair: Keypair::new(),
                    reward_multiplier: 7_000,
                    switchboard_feed: None,
                    no_feed_weight: Some(1 * WEIGHT_PRECISION),
                    fee_group: NcnFeeGroup::lst(),
                    vault_count: 1,
                },
            ],
            delegations: vec![
                // Need 7
                1,
                sol_to_lamports(1000.0),
                sol_to_lamports(10000.0),
                sol_to_lamports(100000.0),
                sol_to_lamports(1000000.0),
                sol_to_lamports(10000000.0),
                255,
            ],
            base_engine_fee_bps: 500,
            dao_fee_bps: 270,
            operator_fee_bps: 100,
            lst_fee_bps: 15,
            jto_fee_bps: 15,
            rewards_amount: sol_to_lamports(137000.0) + 1,
        };

        run_simulation(config).await
    }

    #[ignore = "20-30 minute test"]
    #[tokio::test]
    async fn test_high_operator_count_simulation() -> TestResult<()> {
        let base_fee_wallet =
            Pubkey::from_str("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF").unwrap();

        let config = SimConfig {
            operator_count: 50,
            base_fee_wallet,
            mints: vec![MintConfig {
                keypair: Keypair::new(),
                reward_multiplier: 20_000,
                switchboard_feed: Some(JITOSOL_SOL_FEED),
                no_feed_weight: None,
                fee_group: NcnFeeGroup::lst(),
                vault_count: 2,
            }],
            delegations: vec![sol_to_lamports(1000.0), sol_to_lamports(1000.0)],
            base_engine_fee_bps: 500,
            dao_fee_bps: 270,
            operator_fee_bps: 100,
            lst_fee_bps: 15,
            jto_fee_bps: 15,
            rewards_amount: 100_000,
        };

        run_simulation(config).await
    }

    #[ignore = "20-30 minute test"]
    #[tokio::test]
    async fn test_fuzz_simulation() -> TestResult<()> {
        let base_fee_wallet =
            Pubkey::from_str("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF").unwrap();

        // Create multiple test configurations with different parameters
        let test_configs = vec![
            // Test varying operator counts
            SimConfig {
                operator_count: 15, // Mid-size operator set
                base_fee_wallet,
                mints: vec![
                    MintConfig {
                        keypair: Keypair::new(),
                        reward_multiplier: 15_000,
                        switchboard_feed: Some(JITOSOL_SOL_FEED),
                        no_feed_weight: None,
                        fee_group: NcnFeeGroup::lst(),
                        vault_count: 2,
                    },
                    MintConfig {
                        keypair: Keypair::new(),
                        reward_multiplier: 12_000,
                        switchboard_feed: Some(JTO_SOL_FEED),
                        no_feed_weight: None,
                        fee_group: NcnFeeGroup::jto(),
                        vault_count: 1,
                    },
                ],
                delegations: vec![
                    sol_to_lamports(500.0),
                    sol_to_lamports(5000.0),
                    sol_to_lamports(50000.0),
                ],
                base_engine_fee_bps: 400,
                dao_fee_bps: 250,
                operator_fee_bps: 90,
                lst_fee_bps: 12,
                jto_fee_bps: 12,
                rewards_amount: sol_to_lamports(50000.0),
            },
            // Test extreme delegation amounts
            SimConfig {
                operator_count: 20,
                base_fee_wallet,
                mints: vec![MintConfig {
                    keypair: Keypair::new(),
                    reward_multiplier: 25_000,
                    switchboard_feed: None,
                    no_feed_weight: Some(2 * WEIGHT_PRECISION),
                    fee_group: NcnFeeGroup::lst(),
                    vault_count: 3,
                }],
                delegations: vec![
                    1, // Minimum delegation
                    sol_to_lamports(1.0),
                    sol_to_lamports(1_000_000.0), // Very large delegation
                ],
                base_engine_fee_bps: 600,
                dao_fee_bps: 300,
                operator_fee_bps: 150,
                lst_fee_bps: 20,
                jto_fee_bps: 20,
                rewards_amount: sol_to_lamports(900_000.0) - 1,
            },
            // Test mixed fee groups and feeds
            SimConfig {
                operator_count: 30,
                base_fee_wallet,
                mints: vec![
                    MintConfig {
                        keypair: Keypair::new(),
                        reward_multiplier: 18_000,
                        switchboard_feed: Some(JITOSOL_SOL_FEED),
                        no_feed_weight: None,
                        fee_group: NcnFeeGroup::lst(),
                        vault_count: 1,
                    },
                    MintConfig {
                        keypair: Keypair::new(),
                        reward_multiplier: 8_000,
                        switchboard_feed: Some(JTO_SOL_FEED),
                        no_feed_weight: None,
                        fee_group: NcnFeeGroup::jto(),
                        vault_count: 1,
                    },
                    MintConfig {
                        keypair: Keypair::new(),
                        reward_multiplier: 5_000,
                        switchboard_feed: None,
                        no_feed_weight: Some(WEIGHT_PRECISION / 2),
                        fee_group: NcnFeeGroup::lst(),
                        vault_count: 1,
                    },
                ],
                delegations: vec![
                    sol_to_lamports(100.0),
                    sol_to_lamports(1000.0),
                    sol_to_lamports(10000.0),
                ],
                base_engine_fee_bps: 450,
                dao_fee_bps: 200,
                operator_fee_bps: 80,
                lst_fee_bps: 10,
                jto_fee_bps: 10,
                rewards_amount: sol_to_lamports(75000.0),
            },
        ];

        // Run all configurations
        for (i, config) in test_configs.into_iter().enumerate() {
            println!("Running fuzz test configuration {}", i + 1);
            run_simulation(config).await?;
        }

        Ok(())
    }
}
