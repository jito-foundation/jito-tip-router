#[cfg(test)]
mod tests {

    use jito_restaking_core::MAX_FEE_BPS;
    use jito_tip_router_core::error::TipRouterError;

    use crate::fixtures::{
        test_builder::TestBuilder, tip_router_client::assert_tip_router_error, TestResult,
    };

    #[tokio::test]
    async fn test_removing_operator() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 3;
        const VAULT_COUNT: usize = 1;
        const OPERATOR_FEE_BPS: u16 = MAX_FEE_BPS;
        const INDEX_OF_OPERATOR_TO_REMOVE: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(OPERATOR_FEE_BPS))
            .await?;

        {
            fixture.warp_epoch_incremental(2).await?;
        }

        {
            // First Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;
        }

        {
            // Remove an operator
            let operator = test_ncn.operators[INDEX_OF_OPERATOR_TO_REMOVE].operator_pubkey;
            restaking_client
                .do_ncn_cooldown_operator(&test_ncn.ncn_root, &operator)
                .await?;

            // Warp to next epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        {
            // Second Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_removing_vault() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut restaking_client = fixture.restaking_program_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 3;
        const INDEX_OF_VAULT_TO_REMOVE: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(0))
            .await?;

        {
            fixture.warp_epoch_incremental(2).await?;
        }

        {
            // First Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;
        }

        {
            // Remove a vault
            let vault = test_ncn.vaults[INDEX_OF_VAULT_TO_REMOVE].vault_pubkey;
            restaking_client
                .do_cooldown_ncn_vault_ticket(&test_ncn.ncn_root, &vault)
                .await?;

            // Warp to next epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        {
            // Second Run
            fixture.snapshot_test_ncn(&test_ncn).await?;

            fixture.vote_test_ncn(&test_ncn).await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_stale_vault() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut stake_pool_client = fixture.stake_pool_client();
        let mut tip_router_client = fixture.tip_router_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, Some(0))
            .await?;

        stake_pool_client.do_initialize_stake_pool().await?;

        {
            // Fast forward to a new epoch
            fixture.warp_epoch_incremental(1).await?;
        }

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;
        fixture.add_admin_weights_for_test_ncn(&test_ncn).await?;
        fixture.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        fixture
            .add_operator_snapshots_to_test_ncn(&test_ncn)
            .await?;
        {
            let epoch = fixture.clock().await.epoch;
            let ncn = test_ncn.ncn_root.ncn_pubkey;

            let operator = test_ncn.operators[0].operator_pubkey;
            let vault = test_ncn.vaults[0].vault_pubkey;

            let result = tip_router_client
                .do_snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
                .await;

            assert_tip_router_error(result, TipRouterError::VaultNeedsUpdate);
        }

        Ok(())
    }
}
