#[cfg(test)]
mod tests {

    use solana_sdk::pubkey::Pubkey;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_admin_update_weight_table() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        fixture.warp_slot_incremental(1000).await?;

        let slot = fixture.clock().await.slot;

        tip_router_client.setup_tip_router(&ncn_root).await?;

        tip_router_client
            .do_initialize_weight_table(ncn_root.ncn_pubkey, slot)
            .await?;

        let mint = Pubkey::new_unique();
        let weight = 100;

        tip_router_client
            .do_admin_update_weight_table(ncn_root.ncn_pubkey, slot, mint, weight)
            .await?;

        //TODO add functionality to update weight table
        Ok(())
    }
}
