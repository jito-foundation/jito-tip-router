#[cfg(test)]
mod tests {

    use jito_tip_router_core::{
        constants::MAX_REALLOC_BYTES, epoch_snapshot::OperatorSnapshot, error::TipRouterError,
    };

    use crate::fixtures::{
        test_builder::TestBuilder, tip_router_client::assert_tip_router_error, TestResult,
    };

    #[tokio::test]
    async fn test_initialize_operator_snapshot() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.warp_slot_incremental(1000).await?;

        fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;

        let clock = fixture.clock().await;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;

        // Initialize operator snapshot
        tip_router_client
            .do_initialize_operator_snapshot(operator, ncn, epoch)
            .await?;

        // Check initial size is MAX_REALLOC_BYTES
        let address = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
        )
        .0;
        let raw_account = fixture.get_account(&address).await?.unwrap();
        assert_eq!(raw_account.data.len(), MAX_REALLOC_BYTES as usize);
        assert_eq!(raw_account.owner, jito_tip_router_program::id());
        assert_eq!(raw_account.data[0], 0);

        // Calculate number of reallocs needed
        let num_reallocs =
            (OperatorSnapshot::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

        // Realloc to full size
        tip_router_client
            .do_realloc_operator_snapshot(operator, ncn, epoch, num_reallocs)
            .await?;

        // Get operator snapshot and verify it was initialized correctly
        let operator_snapshot = tip_router_client
            .get_operator_snapshot(operator, ncn, epoch)
            .await?;

        // Verify initial state
        assert_eq!(*operator_snapshot.operator(), operator);
        assert_eq!(*operator_snapshot.ncn(), ncn);

        Ok(())
    }

    #[tokio::test]
    async fn test_add_operator_after_epoch_snapshot() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let mut test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;
        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture.warp_slot_incremental(1000).await?;

        fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;

        // Add New Operator
        fixture
            .add_operators_to_test_ncn(&mut test_ncn, 1, None)
            .await?;

        let clock = fixture.clock().await;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        // Last added operator
        let operator = test_ncn.operators[1].operator_pubkey;

        // Initialize operator snapshot
        let result = tip_router_client
            .do_initialize_operator_snapshot(operator, ncn, epoch)
            .await;

        assert_tip_router_error(result, TipRouterError::OperatorIsNotInSnapshot);

        Ok(())
    }
}
