#[cfg(test)]
mod tests {
    use jito_bytemuck::Discriminator;
    use jito_tip_router_core::{constants::MAX_REALLOC_BYTES, weight_table::WeightTable};

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_weight_table_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        fixture.warp_slot_incremental(1000).await?;

        let clock = fixture.clock().await;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        tip_router_client
            .do_initialize_weight_table(ncn, epoch)
            .await?;

        let address =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let raw_account = fixture.get_account(&address).await?.unwrap();
        assert_eq!(raw_account.data.len(), MAX_REALLOC_BYTES as usize);
        assert_eq!(raw_account.owner, jito_tip_router_program::id());
        assert_eq!(raw_account.data[0], 0);

        let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

        tip_router_client
            .do_realloc_weight_table(ncn, epoch, num_reallocs)
            .await?;

        let raw_account = fixture.get_account(&address).await?.unwrap();
        assert_eq!(raw_account.data.len(), { WeightTable::SIZE });
        assert_eq!(raw_account.owner, jito_tip_router_program::id());
        assert_eq!(raw_account.data[0], WeightTable::DISCRIMINATOR);

        let weight_table = tip_router_client.get_weight_table(ncn, epoch).await?;

        assert_eq!(weight_table.ncn(), ncn);
        assert_eq!(weight_table.ncn_epoch(), epoch);

        Ok(())
    }
}
