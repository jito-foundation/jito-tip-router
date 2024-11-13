#[cfg(test)]
mod tests {
    use jito_tip_router_core::{ncn_config::NcnConfig, tracked_mints::TrackedMints};
    use solana_program::instruction::InstructionError;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use crate::fixtures::{assert_ix_error, test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_tracked_mints_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        tip_router_client
            .do_initialize_tracked_mints(ncn_root.ncn_pubkey)
            .await?;

        let tracked_mints = tip_router_client
            .get_tracked_mints(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(tracked_mints.ncn, ncn_root.ncn_pubkey);
        assert_eq!(tracked_mints.mint_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_tracked_mints_wrong_ncn_config_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Try to initialize with wrong NCN config
        let wrong_ncn_config = Keypair::new().pubkey();
        let (tracked_mints_key, _, _) = TrackedMints::find_program_address(
            &jito_tip_router_program::id(),
            &ncn_root.ncn_pubkey,
        );

        let transaction_error = tip_router_client
            .initialize_tracked_mints(&wrong_ncn_config, &tracked_mints_key, &ncn_root.ncn_pubkey)
            .await;

        assert_ix_error(transaction_error, InstructionError::InvalidAccountOwner);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_tracked_mints_wrong_ncn_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Try to initialize with wrong NCN
        let wrong_ncn = Keypair::new().pubkey();
        let (tracked_mints_key, _, _) =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &wrong_ncn);

        let transaction_error = tip_router_client
            .initialize_tracked_mints(
                &NcnConfig::find_program_address(
                    &jito_tip_router_program::id(),
                    &ncn_root.ncn_pubkey,
                )
                .0,
                &tracked_mints_key,
                &wrong_ncn,
            )
            .await;

        assert_ix_error(transaction_error, InstructionError::InvalidAccountData);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_tracked_mints_double_init_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        tip_router_client
            .do_initialize_tracked_mints(ncn_root.ncn_pubkey)
            .await?;

        fixture.warp_slot_incremental(1).await?;

        // Second initialization should fail
        let transaction_error = tip_router_client
            .do_initialize_tracked_mints(ncn_root.ncn_pubkey)
            .await;

        assert_ix_error(transaction_error, InstructionError::InvalidAccountOwner);
        Ok(())
    }
}
