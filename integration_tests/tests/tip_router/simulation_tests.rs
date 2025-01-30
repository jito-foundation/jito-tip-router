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
            sol_to_lamports(1000.0),
            sol_to_lamports(1000.0),
            sol_to_lamports(1000.0),
            sol_to_lamports(1000.0),
            sol_to_lamports(1000.0),
            sol_to_lamports(1000.0),
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

            for i in 0..MAX_OPERATORS + 5 {
                let mut meta_merkle_root = [0; 32];
                meta_merkle_root[0] = i as u8;
                meta_merkle_root[1] = i as u8 % 2;
                meta_merkle_root[2] = i as u8 % 3;
                meta_merkle_root[3] = i as u8 % 5;
                meta_merkle_root[4] = i as u8 % 8;
                meta_merkle_root[5] = i as u8 % 13;
                meta_merkle_root[6] = i as u8 % 21;
                meta_merkle_root[7] = i as u8 % 34;
                meta_merkle_root[8] = i as u8 % 55;
                meta_merkle_root[9] = (i as u8).max(1);

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

            let meta_merkle_root = [1u8; 32];
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
            let meta_merkle_root = [2u8; 32];
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
            let meta_merkle_root = [9u8; 32];
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
            let meta_merkle_root = [11u8; 32];
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
