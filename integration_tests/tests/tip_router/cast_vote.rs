#[cfg(test)]
mod tests {
    use jito_tip_router_core::ballot_box::Ballot;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_cast_vote() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let slot = clock.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;
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
