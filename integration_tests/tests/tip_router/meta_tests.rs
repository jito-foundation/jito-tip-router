#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_all_test_ncn_functions() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let mut test_ncn = fixture.create_test_ncn().await?;
        fixture
            .add_operators_to_test_ncn(&mut test_ncn, OPERATOR_COUNT, None)
            .await?;
        fixture
            .add_vaults_to_test_ncn(&mut test_ncn, VAULT_COUNT)
            .await?;
        fixture.add_delegation_in_test_ncn(&test_ncn, 100).await?;
        fixture.add_tracked_mints_to_test_ncn(&test_ncn).await?;
        fixture.add_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;
        fixture
            .add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
            .await?;
        fixture.add_ballot_box_to_test_ncn(&test_ncn).await?;
        fixture.cast_votes_for_test_ncn(&test_ncn).await?;
        fixture.add_routers_for_tests_ncn(&test_ncn).await?;
        fixture
            .route_in_base_rewards_for_test_ncn(&test_ncn, 10_000)
            .await?;
        fixture.route_in_ncn_rewards_for_test_ncn(&test_ncn).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_intermission_test_ncn_functions() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;

        let slot = fixture.clock().await.slot;
        let ncn_epoch = restaking_client.get_ncn_epoch(slot).await?;

        let epoch_snapshot = tip_router_client
            .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(epoch_snapshot.finalized());

        fixture.vote_test_ncn(&test_ncn).await?;

        let ballot_box = tip_router_client
            .get_ballot_box(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(ballot_box.has_winning_ballot());

        fixture.reward_test_ncn(&test_ncn, 10_000).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_operators() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 10;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;

        let slot = fixture.clock().await.slot;
        let ncn_epoch = restaking_client.get_ncn_epoch(slot).await?;

        let epoch_snapshot = tip_router_client
            .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(epoch_snapshot.finalized());

        fixture.vote_test_ncn(&test_ncn).await?;

        let ballot_box = tip_router_client
            .get_ballot_box(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(ballot_box.has_winning_ballot());

        fixture.reward_test_ncn(&test_ncn, 10_000).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_vaults() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 10;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;

        let slot = fixture.clock().await.slot;
        let ncn_epoch = restaking_client.get_ncn_epoch(slot).await?;

        let epoch_snapshot = tip_router_client
            .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(epoch_snapshot.finalized());

        fixture.vote_test_ncn(&test_ncn).await?;

        let ballot_box = tip_router_client
            .get_ballot_box(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(ballot_box.has_winning_ballot());

        fixture.reward_test_ncn(&test_ncn, 10_000).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_operators_and_vaults() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 10;
        const VAULT_COUNT: usize = 10;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;
        fixture.snapshot_test_ncn(&test_ncn).await?;

        let slot = fixture.clock().await.slot;
        let ncn_epoch = restaking_client.get_ncn_epoch(slot).await?;

        let epoch_snapshot = tip_router_client
            .get_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(epoch_snapshot.finalized());

        fixture.vote_test_ncn(&test_ncn).await?;

        let ballot_box = tip_router_client
            .get_ballot_box(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await?;

        assert!(ballot_box.has_winning_ballot());

        fixture.reward_test_ncn(&test_ncn, 10_000).await?;

        Ok(())
    }
}
