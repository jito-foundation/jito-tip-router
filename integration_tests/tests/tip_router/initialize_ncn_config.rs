#[cfg(test)]
mod tests {
    use jito_tip_router_core::error::TipRouterError;
    use solana_program::instruction::InstructionError;
    use solana_sdk::signature::{Keypair, Signer};

    use crate::fixtures::{
        assert_ix_error, restaking_client::NcnRoot, test_builder::TestBuilder,
        tip_router_client::assert_tip_router_error, TestResult,
    };

    #[tokio::test]
    async fn test_initialize_ncn_config_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_ncn_config_double_init_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;
        fixture.warp_slot_incremental(1).await?;
        let transaction_error = tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await;
        assert_ix_error(transaction_error, InstructionError::InvalidAccountOwner);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_ncn_config_invalid_ncn_fails() -> TestResult<()> {
        let fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let fake_ncn = Keypair::new();
        let fake_admin = Keypair::new();
        let fake_ncn_root = NcnRoot {
            ncn_pubkey: fake_ncn.pubkey(),
            ncn_admin: fake_admin,
        };
        tip_router_client
            .airdrop(&fake_ncn_root.ncn_admin.pubkey(), 1.0)
            .await?;
        let transaction_error = tip_router_client
            .do_initialize_config(fake_ncn_root.ncn_pubkey, &fake_ncn_root.ncn_admin)
            .await;
        assert_ix_error(transaction_error, InstructionError::InvalidAccountOwner);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_ncn_config_fees_exceed_max_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        let ncn_admin_pubkey = ncn_root.ncn_admin.pubkey();
        let transaction_error = tip_router_client
            .initialize_config(
                ncn_root.ncn_pubkey,
                &ncn_root.ncn_admin,
                &ncn_admin_pubkey,
                &ncn_admin_pubkey,
                0,
                0,
                10_001,
                0,
                0,
            )
            .await;
        assert_tip_router_error(transaction_error, TipRouterError::FeeCapExceeded);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_ncn_config_invalid_parameters() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        // Test invalid epochs_before_stall
        let result = tip_router_client
            .initialize_config(
                ncn_root.ncn_pubkey,
                &ncn_root.ncn_admin,
                &ncn_root.ncn_admin.pubkey(),
                &ncn_root.ncn_admin.pubkey(),
                0,
                0,
                0,
                0, // Invalid - too low
                10001,
            )
            .await;
        assert_tip_router_error(result, TipRouterError::InvalidEpochsBeforeStall);

        // Test invalid valid_slots_after_consensus
        let result = tip_router_client
            .initialize_config(
                ncn_root.ncn_pubkey,
                &ncn_root.ncn_admin,
                &ncn_root.ncn_admin.pubkey(),
                &ncn_root.ncn_admin.pubkey(),
                0,
                0,
                0,
                5,
                50, // Invalid - too low
            )
            .await;
        assert_tip_router_error(result, TipRouterError::InvalidSlotsAfterConsensus);

        Ok(())
    }
}
