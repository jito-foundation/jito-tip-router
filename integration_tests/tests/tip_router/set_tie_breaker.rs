#[cfg(test)]
mod tests {

    use jito_tip_router_core::{ballot_box::Ballot, constants::DEFAULT_CONSENSUS_REACHED_SLOT};

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_set_tie_breaker() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        // Each operator gets 50% voting share
        let test_ncn = fixture.create_initial_test_ncn(2, 1).await?;

        ///// TipRouter Setup /////
        fixture.snapshot_test_ncn(&test_ncn).await?;

        let clock = fixture.clock().await;
        let slot = clock.slot;
        let restaking_config_account = tip_router_client.get_restaking_config().await?;
        let ncn_epoch = slot / restaking_config_account.epoch_length();
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        tip_router_client
            .do_initialize_ballot_box(ncn, ncn_epoch)
            .await?;

        let meta_merkle_root = [1; 32];

        let operator = test_ncn.operators[0].operator_pubkey;
        let operator_admin = &test_ncn.operators[0].operator_admin;

        // // Cast a vote so that this vote is one of the valid options
        // // Gets to 50% consensus weight
        tip_router_client
            .do_cast_vote(ncn, operator, operator_admin, meta_merkle_root, ncn_epoch)
            .await?;

        let ballot_box = tip_router_client.get_ballot_box(ncn, ncn_epoch).await?;
        assert!(ballot_box.has_ballot(&Ballot::new(meta_merkle_root)));
        assert_eq!(
            ballot_box.slot_consensus_reached(),
            DEFAULT_CONSENSUS_REACHED_SLOT
        );
        assert!(!ballot_box.is_consensus_reached());

        // Wait a bunch of epochs for voting window to expire (TODO use the exact length)
        fixture.warp_slot_incremental(1000000).await?;

        tip_router_client
            .do_set_tie_breaker(ncn, meta_merkle_root, ncn_epoch)
            .await?;

        let ballot_box = tip_router_client.get_ballot_box(ncn, ncn_epoch).await?;

        let ballot = Ballot::new(meta_merkle_root);
        assert!(ballot_box.has_ballot(&ballot));
        assert_eq!(
            ballot_box.get_winning_ballot_tally().unwrap().ballot(),
            ballot
        );
        // No official consensus reached so no slot set
        assert_eq!(
            ballot_box.slot_consensus_reached(),
            DEFAULT_CONSENSUS_REACHED_SLOT
        );
        assert!(ballot_box.is_consensus_reached());

        Ok(())
    }
}
