#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_operator_snapshot() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut vault_client = fixture.vault_program_client();
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1).await?;

        fixture.warp_slot_incremental(1000).await?;

        let slot = fixture.clock().await.slot;

        tip_router_client
            .do_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, slot)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let vault_root = test_ncn.vaults[0].clone();
        let vault = vault_client.get_vault(&vault_root.vault_pubkey).await?;

        let mint = vault.supported_mint;
        let weight = 100;

        tip_router_client
            .do_admin_update_weight_table(ncn, slot, mint, weight)
            .await?;

        tip_router_client
            .do_initialize_epoch_snapshot(ncn, slot)
            .await?;

        let operator = test_ncn.operators[0].operator_pubkey;

        tip_router_client
            .do_initalize_operator_snapshot(operator, ncn, slot)
            .await?;

        Ok(())
    }
}