#[cfg(test)]
mod tests {
    use jito_tip_router_core::{
        ballot_box::Ballot, constants::MAX_OPERATORS, error::TipRouterError,
    };
    use solana_sdk::pubkey::Pubkey;

    use crate::fixtures::{
        test_builder::TestBuilder, tip_router_client::assert_tip_router_error, TestResult,
    };

    #[tokio::test]
    async fn test_cast_vote() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let slot = clock.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;
        let epoch = clock.epoch;

        tip_router_client
            .do_full_initialize_ballot_box(ncn, epoch)
            .await?;

        let meta_merkle_root = [1u8; 32];

        let operator_admin = &test_ncn.operators[0].operator_admin;

        tip_router_client
            .do_cast_vote(ncn, operator, operator_admin, meta_merkle_root, epoch)
            .await?;

        let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;

        assert!(ballot_box.has_ballot(&Ballot::new(&meta_merkle_root)));
        assert_eq!(ballot_box.slot_consensus_reached(), slot);
        assert!(ballot_box.is_consensus_reached());

        Ok(())
    }

    #[tokio::test]
    async fn test_change_vote() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(3, 1, None).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let slot = clock.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;
        let epoch = clock.epoch;

        tip_router_client
            .do_full_initialize_ballot_box(ncn, epoch)
            .await?;

        {
            let meta_merkle_root = [2u8; 32];

            let operator_admin = &test_ncn.operators[0].operator_admin;

            tip_router_client
                .do_cast_vote(ncn, operator, operator_admin, meta_merkle_root, epoch)
                .await?;
        }

        let winning_meta_merkle_root = [1u8; 32];
        for operator in test_ncn.operators {
            let operator_admin = &operator.operator_admin;

            let meta_merkle_root = [1u8; 32];

            tip_router_client
                .do_cast_vote(
                    ncn,
                    operator.operator_pubkey,
                    operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
        }

        let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;

        assert!(ballot_box.has_ballot(&Ballot::new(&winning_meta_merkle_root)));
        assert_eq!(ballot_box.slot_consensus_reached(), slot);
        assert_eq!(ballot_box.unique_ballots(), 1);
        assert_eq!(
            ballot_box
                .get_winning_ballot_tally()
                .unwrap()
                .stake_weights()
                .stake_weight(),
            30_000
        );
        assert!(ballot_box.is_consensus_reached());

        Ok(())
    }

    #[tokio::test]
    async fn test_bad_ballot() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(3, 1, None).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;
        let epoch = clock.epoch;

        tip_router_client
            .do_full_initialize_ballot_box(ncn, epoch)
            .await?;

        let meta_merkle_root = [0u8; 32];

        let operator_admin = &test_ncn.operators[0].operator_admin;

        let result = tip_router_client
            .do_cast_vote(ncn, operator, operator_admin, meta_merkle_root, epoch)
            .await;

        assert_tip_router_error(result, TipRouterError::BadBallot);

        Ok(())
    }

    #[ignore = "long test"]
    #[tokio::test]
    async fn test_cast_vote_max_cu() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture
            .create_initial_test_ncn(MAX_OPERATORS, 1, None)
            .await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = clock.epoch;

        tip_router_client
            .do_full_initialize_ballot_box(ncn, epoch)
            .await?;

        for operator in test_ncn.operators {
            let operator_admin = &operator.operator_admin;

            let meta_merkle_root = Pubkey::new_unique().to_bytes();

            tip_router_client
                .do_cast_vote(
                    ncn,
                    operator.operator_pubkey,
                    operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;

            let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;
            assert!(ballot_box.has_ballot(&Ballot::new(&meta_merkle_root)));
        }

        let ballot_box = tip_router_client.get_ballot_box(ncn, epoch).await?;
        assert!(!ballot_box.is_consensus_reached());

        Ok(())
    }
}
