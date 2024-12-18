#[cfg(test)]
mod tests {

    use jito_tip_router_core::{constants::JTOSOL_SOL_FEED, ncn_fee_group::NcnFeeGroup};

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_switchboard_set_weight() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut vault_client = fixture.vault_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;

        // Not sure if this is needed
        self.warp_slot_incremental(1000).await?;

        let clock = self.clock().await;
        let epoch = clock.epoch;
        tip_router_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        Ok(())
    }
}
