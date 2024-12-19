#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_epoch_snapshot_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        fixture.warp_slot_incremental(1000).await?;

        let slot = fixture.clock().await.slot;

        tip_router_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, slot)
            .await?;

        let vault_root = test_ncn.vaults[0].clone();
        let vault = vault_client.get_vault(&vault_root.vault_pubkey).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        tip_router_client
            .do_admin_set_weight(test_ncn.ncn_root.ncn_pubkey, slot, mint, weight)
            .await?;

        tip_router_client
            .do_initialize_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, slot)
            .await?;

        Ok(())
    }
}
