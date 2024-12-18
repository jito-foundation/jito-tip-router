#[cfg(test)]
mod tests {

    use jito_tip_router_core::{
        base_reward_router::BaseRewardRouter, constants::MAX_REALLOC_BYTES,
    };

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

        let clock = fixture.clock().await;
        let epoch = clock.epoch;
        let slot = clock.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        // Initialize base reward router
        tip_router_client
            .do_initialize_base_reward_router(ncn, epoch)
            .await?;

        // Check initial size is MAX_REALLOC_BYTES
        let address =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let raw_account = fixture.get_account(&address).await?.unwrap();
        assert_eq!(raw_account.data.len(), MAX_REALLOC_BYTES as usize);
        assert_eq!(raw_account.owner, jito_tip_router_program::id());
        assert_eq!(raw_account.data[0], 0);

        // Calculate number of reallocs needed
        let num_reallocs =
            (BaseRewardRouter::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

        // Realloc to full size
        tip_router_client
            .do_realloc_base_reward_router(ncn, epoch, num_reallocs)
            .await?;

        // Get base reward router and verify it was initialized correctly
        let base_reward_router = tip_router_client.get_base_reward_router(ncn, epoch).await?;

        // Verify initial state
        assert_eq!(base_reward_router.reward_pool(), 0);
        assert_eq!(base_reward_router.rewards_processed(), 0);
        assert_eq!(base_reward_router.total_rewards(), 0);
        assert_eq!(base_reward_router.ncn(), &ncn);
        assert_eq!(base_reward_router.slot_created(), slot);

        Ok(())
    }
}
