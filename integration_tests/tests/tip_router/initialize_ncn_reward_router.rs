#[cfg(test)]
mod tests {

    use jito_tip_router_core::ncn_fee_group::NcnFeeGroup;
    use solana_sdk::pubkey::Pubkey;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_ncn_reward_router() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();

        let test_ncn = fixture.create_initial_test_ncn(1, 1, None).await?;

        ///// TipRouter Setup /////
        fixture.warp_slot_incremental(1000).await?;

        fixture.snapshot_test_ncn(&test_ncn).await?;
        fixture.vote_test_ncn(&test_ncn).await?;
        //////

        let clock = fixture.clock().await;
        let slot = clock.slot;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let operator = test_ncn.operators[0].operator_pubkey;
        let ncn_fee_group = NcnFeeGroup::default();

        // Initialize NCN reward router
        tip_router_client
            .do_initialize_ncn_reward_router(ncn_fee_group, ncn, operator, epoch)
            .await?;

        // Get NCN reward router and verify initialization
        let ncn_reward_router = tip_router_client
            .get_ncn_reward_router(ncn_fee_group, operator, ncn, epoch)
            .await?;

        // Verify initial state
        assert_eq!(ncn_reward_router.ncn(), ncn);
        assert_eq!(ncn_reward_router.operator(), operator);
        assert_eq!(ncn_reward_router.ncn_epoch(), epoch);
        assert_eq!(ncn_reward_router.slot_created(), slot);
        assert_eq!(ncn_reward_router.reward_pool(), 0);
        assert_eq!(ncn_reward_router.rewards_processed(), 0);
        assert_eq!(ncn_reward_router.total_rewards(), 0);
        assert_eq!(ncn_reward_router.operator_rewards(), 0);

        // Verify a default vault reward route exists and is empty
        assert_eq!(ncn_reward_router.vault_reward_routes()[0].rewards(), 0);
        assert_eq!(
            ncn_reward_router.vault_reward_routes()[0].vault(),
            Pubkey::default()
        );

        Ok(())
    }
}
