#[cfg(test)]
mod tests {
    use jito_vault_core::vault_operator_delegation::VaultOperatorDelegation;
    use solana_program::instruction::InstructionError;
    use solana_sdk::pubkey::Pubkey;

    use crate::fixtures::{assert_ix_error, test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_snapshot_vault_operator_delegation() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.warp_slot_incremental(1000).await?;

        let epoch = fixture.clock().await.epoch;

        tip_router_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        tip_router_client
            .do_admin_set_weight(ncn, epoch, mint, weight)
            .await?;

        tip_router_client
            .do_initialize_epoch_snapshot(ncn, epoch)
            .await?;

        let operator = test_ncn.operators[0].operator_pubkey;

        tip_router_client
            .do_full_initialize_operator_snapshot(operator, ncn, epoch)
            .await?;

        tip_router_client
            .do_snapshot_vault_operator_delegation(vault_address, operator, ncn, epoch)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_snapshot_vault_operator_delegation_wrong_pda() {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await.unwrap();
        fixture
            .add_epoch_state_for_test_ncn(&test_ncn)
            .await
            .unwrap();

        fixture.warp_slot_incremental(1000).await.unwrap();

        let epoch = fixture.clock().await.epoch;

        tip_router_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await
            .unwrap();

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await.unwrap();

        let mint = vault.supported_mint;
        let weight = 100;

        tip_router_client
            .do_admin_set_weight(ncn, epoch, mint, weight)
            .await
            .unwrap();

        tip_router_client
            .do_initialize_epoch_snapshot(ncn, epoch)
            .await
            .unwrap();

        let operator = test_ncn.operators[0].operator_pubkey;

        tip_router_client
            .do_full_initialize_operator_snapshot(operator, ncn, epoch)
            .await
            .unwrap();

        // Pass a random pubkey that is empty and does not match the expected PDA.
        // The new guard must reject this with InvalidAccountData.
        let wrong_delegation = Pubkey::new_unique();
        let result = tip_router_client
            .snapshot_vault_operator_delegation_with_override(
                vault_address,
                operator,
                ncn,
                epoch,
                wrong_delegation,
            )
            .await;

        assert_ix_error(result, InstructionError::InvalidAccountData);
    }

    // Operator added after vault setup has no VaultOperatorDelegation, so the
    // correct PDA exists on-chain as an empty (uninitialized) account.
    // The instruction should succeed, recording zero stake weight.
    #[tokio::test]
    async fn test_snapshot_vault_operator_delegation_correct_pda_empty() {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut tip_router_client = fixture.tip_router_client();

        // Build the NCN manually so we can add operator[1] after vault setup.
        // add_vault_registry_to_test_ncn must run first because it calls
        // do_full_vault_update, which cranks VaultOperatorDelegation for every
        // operator in the list — an account that won't exist for operator[1].
        let mut test_ncn = fixture.create_test_ncn().await.unwrap();
        fixture
            .add_operators_to_test_ncn(&mut test_ncn, 1, None)
            .await
            .unwrap();
        fixture
            .add_vaults_to_test_ncn(&mut test_ncn, 1, None)
            .await
            .unwrap();
        fixture
            .add_delegation_in_test_ncn(&test_ncn, 100)
            .await
            .unwrap();
        // This internally warps epoch_length * 2, activating operator[0].
        fixture
            .add_vault_registry_to_test_ncn(&test_ncn)
            .await
            .unwrap();

        // operator[1] added after vault_registry: no VaultOperatorDelegation is
        // ever created for it, so its PDA will be empty on-chain.
        fixture
            .add_operators_to_test_ncn(&mut test_ncn, 1, None)
            .await
            .unwrap();

        // Warp a full epoch so operator[1]'s warmup period completes and
        // realloc_operator_snapshot initialises it as is_active=true.
        // Also re-crank the vault update (only for operator[0], since operator[1]
        // has no VaultOperatorDelegation) because the vault becomes stale after
        // the large slot warp.
        {
            let mut restaking_client = fixture.restaking_program_client();
            let config_address = jito_restaking_core::config::Config::find_program_address(
                &jito_restaking_program::id(),
            )
            .0;
            let restaking_config = restaking_client.get_config(&config_address).await.unwrap();
            fixture
                .warp_slot_incremental(restaking_config.epoch_length() * 2)
                .await
                .unwrap();
        }

        // Update the vault so it is current for the new epoch; pass only
        // operator[0] because operator[1] has no VaultOperatorDelegation.
        vault_client
            .do_full_vault_update(
                &test_ncn.vaults[0].vault_pubkey,
                &[test_ncn.operators[0].operator_pubkey],
            )
            .await
            .unwrap();

        fixture
            .add_epoch_state_for_test_ncn(&test_ncn)
            .await
            .unwrap();
        fixture.warp_slot_incremental(1000).await.unwrap();

        let epoch = fixture.clock().await.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        tip_router_client
            .do_full_initialize_weight_table(ncn, epoch)
            .await
            .unwrap();

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await.unwrap();

        tip_router_client
            .do_admin_set_weight(ncn, epoch, vault.supported_mint, 100)
            .await
            .unwrap();

        tip_router_client
            .do_initialize_epoch_snapshot(ncn, epoch)
            .await
            .unwrap();

        let operator = test_ncn.operators[1].operator_pubkey;
        tip_router_client
            .do_full_initialize_operator_snapshot(operator, ncn, epoch)
            .await
            .unwrap();

        // Correct PDA — but the account is empty because the delegation was never created.
        let correct_empty_pda = VaultOperatorDelegation::find_program_address(
            &jito_vault_program::id(),
            &vault_address,
            &operator,
        )
        .0;

        // Should succeed: PDA matches, data is empty → is_active=false, stake_weight=0.
        tip_router_client
            .snapshot_vault_operator_delegation_with_override(
                vault_address,
                operator,
                ncn,
                epoch,
                correct_empty_pda,
            )
            .await
            .unwrap();

        let snapshot = tip_router_client
            .get_operator_snapshot(operator, ncn, epoch)
            .await
            .unwrap();
        assert_eq!(snapshot.stake_weights().stake_weight(), 0);
        assert_eq!(snapshot.valid_operator_vault_delegations(), 0);
    }
}
