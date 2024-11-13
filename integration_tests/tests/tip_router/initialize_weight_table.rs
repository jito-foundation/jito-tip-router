#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_weight_table_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        fixture.warp_slot_incremental(1000).await?;

        let slot = fixture.clock().await.slot;

        tip_router_client.setup_tip_router(&ncn_root).await?;

        tip_router_client
            .do_initialize_weight_table(ncn_root.ncn_pubkey, slot)
            .await?;
        Ok(())
    }
}
