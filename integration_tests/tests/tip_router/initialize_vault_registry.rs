#[cfg(test)]
mod tests {
    use jito_tip_router_core::{ncn_config::NcnConfig, vault_registry::VaultRegistry};
    use solana_program::instruction::InstructionError;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use crate::fixtures::{assert_ix_error, test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_initialize_vault_registry_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        tip_router_client
            .do_full_initialize_vault_registry(ncn_root.ncn_pubkey)
            .await?;

        let vault_registry = tip_router_client
            .get_vault_registry(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(vault_registry.ncn, ncn_root.ncn_pubkey);
        assert_eq!(vault_registry.vault_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_vault_registry_wrong_ncn_config_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Try to initialize with wrong NCN config
        let wrong_ncn_config = Keypair::new().pubkey();
        let (vault_registry_key, _, _) = VaultRegistry::find_program_address(
            &jito_tip_router_program::id(),
            &ncn_root.ncn_pubkey,
        );

        let transaction_error = tip_router_client
            .initialize_vault_registry(&wrong_ncn_config, &vault_registry_key, &ncn_root.ncn_pubkey)
            .await;

        assert_ix_error(transaction_error, InstructionError::InvalidAccountOwner);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_vault_registry_wrong_ncn_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Try to initialize with wrong NCN
        let wrong_ncn = Keypair::new().pubkey();
        let (vault_registry_key, _, _) =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &wrong_ncn);

        let transaction_error = tip_router_client
            .initialize_vault_registry(
                &NcnConfig::find_program_address(
                    &jito_tip_router_program::id(),
                    &ncn_root.ncn_pubkey,
                )
                .0,
                &vault_registry_key,
                &wrong_ncn,
            )
            .await;

        assert_ix_error(transaction_error, InstructionError::InvalidAccountData);
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_vault_registry_double_init_fails() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        tip_router_client
            .do_full_initialize_vault_registry(ncn_root.ncn_pubkey)
            .await?;

        fixture.warp_slot_incremental(1).await?;

        // Second initialization should fail
        let transaction_error = tip_router_client
            .do_full_initialize_vault_registry(ncn_root.ncn_pubkey)
            .await;

        assert_ix_error(transaction_error, InstructionError::InvalidAccountOwner);
        Ok(())
    }
}
