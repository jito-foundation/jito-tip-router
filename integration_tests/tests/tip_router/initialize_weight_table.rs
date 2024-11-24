#[cfg(test)]
mod tests {

    use crate::fixtures::test_builder::TestBuilder;

    #[tokio::test]
    async fn test_initialize_weight_table_ok() {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut restaking_client = fixture.restaking_program_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1).await.unwrap();

        fixture.warp_slot_incremental(1000).await.unwrap();

        let slot = fixture.clock().await.slot;

        tip_router_client
            .do_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, slot)
            .await
            .unwrap();

        let ncn_epoch = restaking_client.get_ncn_epoch(slot).await.unwrap();
        let weight_table = tip_router_client
            .get_weight_table(test_ncn.ncn_root.ncn_pubkey, ncn_epoch)
            .await
            .unwrap();

        assert!(weight_table.initialized());
        assert_eq!(weight_table.ncn(), test_ncn.ncn_root.ncn_pubkey);
        assert_eq!(weight_table.ncn_epoch(), ncn_epoch);
        assert_eq!(weight_table.slot_created(), slot);
    }
}
