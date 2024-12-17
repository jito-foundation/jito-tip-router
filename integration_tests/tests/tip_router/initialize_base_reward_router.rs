#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_base_reward_router() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        //////

        let slot = fixture.clock().await.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Initialize base reward router
        tip_router_client
            .do_initialize_base_reward_router(ncn, slot)
            .await?;

        // Get base reward router and verify it was initialized correctly
        let restaking_config_account = tip_router_client.get_restaking_config().await?;
        let ncn_epoch = slot / restaking_config_account.epoch_length();
        let base_reward_router = tip_router_client
            .get_base_reward_router(ncn, ncn_epoch)
            .await?;

        // Verify initial state
        assert_eq!(base_reward_router.reward_pool(), 0);
        assert_eq!(base_reward_router.rewards_processed(), 0);
        assert_eq!(base_reward_router.total_rewards(), 0);
        assert_eq!(base_reward_router.ncn(), &ncn);
        assert_eq!(base_reward_router.ncn_epoch(), ncn_epoch);
        assert_eq!(base_reward_router.slot_created(), slot);

        Ok(())
    }
}
