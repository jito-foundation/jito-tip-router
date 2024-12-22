mod tests {
    use jito_tip_router_client::types::ConfigAdminRole;
    use jito_tip_router_core::{config::Config as NcnConfig, error::TipRouterError};
    use solana_program::pubkey::Pubkey;
    use solana_sdk::{instruction::InstructionError, signature::Keypair};

    use crate::fixtures::{
        assert_ix_error, restaking_client::NcnRoot, test_builder::TestBuilder,
        tip_router_client::assert_tip_router_error, TestResult,
    };

    #[tokio::test]
    async fn test_set_new_admin_success() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        let new_fee_admin = Pubkey::new_unique();
        tip_router_client
            .do_set_new_admin(ConfigAdminRole::FeeAdmin, new_fee_admin, &ncn_root)
            .await?;

        let config = tip_router_client
            .get_ncn_config(ncn_root.ncn_pubkey)
            .await?;
        assert_eq!(config.fee_admin, new_fee_admin);

        fixture.warp_slot_incremental(1).await?;

        let new_tie_breaker = Pubkey::new_unique();
        tip_router_client
            .do_set_new_admin(ConfigAdminRole::TieBreakerAdmin, new_tie_breaker, &ncn_root)
            .await?;

        let config = tip_router_client
            .get_ncn_config(ncn_root.ncn_pubkey)
            .await?;
        assert_eq!(config.tie_breaker_admin, new_tie_breaker);
        Ok(())
    }

    #[tokio::test]
    async fn test_set_new_admin_incorrect_accounts() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        fixture.warp_slot_incremental(1).await?;
        let mut restaking_program_client = fixture.restaking_program_client();
        let wrong_ncn_root = restaking_program_client.do_initialize_ncn(None).await?;

        let result = tip_router_client
            .set_new_admin(
                NcnConfig::find_program_address(
                    &jito_tip_router_program::id(),
                    &ncn_root.ncn_pubkey,
                )
                .0,
                ConfigAdminRole::FeeAdmin,
                Pubkey::new_unique(),
                &wrong_ncn_root,
            )
            .await;

        assert_ix_error(result, InstructionError::InvalidAccountData);

        let wrong_ncn_root = NcnRoot {
            ncn_pubkey: ncn_root.ncn_pubkey,
            ncn_admin: Keypair::new(),
        };

        let result = tip_router_client
            .do_set_new_admin(
                ConfigAdminRole::FeeAdmin,
                Pubkey::new_unique(),
                &wrong_ncn_root,
            )
            .await;

        assert_tip_router_error(result, TipRouterError::IncorrectNcnAdmin);
        Ok(())
    }
}
