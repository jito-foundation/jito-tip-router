#[cfg(test)]
mod tests {

    use jito_tip_router_core::ballot_box::Ballot;
    use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_cast_vote() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1).await?;

        fixture.warp_slot_incremental(1000).await?;

        //
        let clock = fixture.clock().await;
        let slot = clock.slot;

        tip_router_client
            .do_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, slot)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault_address = vault_root.vault_pubkey;
        let vault = vault_client.get_vault(&vault_address).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        tip_router_client
            .do_admin_update_weight_table(ncn, slot, mint, weight)
            .await?;

        tip_router_client
            .do_initialize_epoch_snapshot(ncn, slot)
            .await?;

        let operator = test_ncn.operators[0].operator_pubkey;

        tip_router_client
            .do_initalize_operator_snapshot(operator, ncn, slot)
            .await?;

        tip_router_client
            .do_snapshot_vault_operator_delegation(vault_address, operator, ncn, slot)
            .await?;
        //

        let restaking_config_account = tip_router_client.get_restaking_config().await?;
        let ncn_epoch = slot / restaking_config_account.epoch_length();

        tip_router_client
            .do_initialize_ballot_box(ncn, ncn_epoch)
            .await?;

        let meta_merkle_root = [1u8; 32];

        let operator_admin = &test_ncn.operators[0].operator_admin;

        tip_router_client
            .do_cast_vote(ncn, operator, operator_admin, meta_merkle_root, ncn_epoch)
            .await?;

        let ballot_box = tip_router_client.get_ballot_box(ncn, ncn_epoch).await?;

        assert!(ballot_box.has_ballot(&Ballot::new(meta_merkle_root)));
        assert_eq!(ballot_box.slot_consensus_reached(), slot);
        assert!(ballot_box.is_consensus_reached());

        Ok(())
    }
}
