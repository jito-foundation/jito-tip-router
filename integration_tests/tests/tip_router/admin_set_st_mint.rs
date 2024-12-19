#[cfg(test)]
mod tests {

    use jito_tip_router_core::{constants::JTOSOL_SOL_FEED, ncn_fee_group::NcnFeeGroup};

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_admin_set_st_mint() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut vault_client = fixture.vault_client();

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let vault = vault_client
            .get_vault(&test_ncn.vaults[0].vault_pubkey)
            .await?;
        let st_mint = vault.supported_mint;
        let ncn_fee_group = Some(NcnFeeGroup::jto());
        let reward_multiplier_bps = Some(10);
        let switchboard_feed = Some(JTOSOL_SOL_FEED);
        let no_feed_weight = Some(100);

        tip_router_client
            .do_admin_set_st_mint(
                ncn,
                st_mint,
                ncn_fee_group,
                reward_multiplier_bps,
                switchboard_feed,
                no_feed_weight,
            )
            .await?;

        let vault_registry = tip_router_client.get_vault_registry(ncn).await?;

        let mint_entry = vault_registry.get_mint_entry(&st_mint).unwrap();

        assert_eq!(mint_entry.st_mint(), st_mint);
        assert_eq!(mint_entry.ncn_fee_group(), ncn_fee_group.unwrap());
        assert_eq!(
            mint_entry.reward_multiplier_bps(),
            reward_multiplier_bps.unwrap()
        );
        assert_eq!(mint_entry.switchboard_feed(), switchboard_feed.unwrap());
        assert_eq!(mint_entry.no_feed_weight(), no_feed_weight.unwrap());

        tip_router_client
            .do_admin_set_st_mint(ncn, st_mint, None, None, None, None)
            .await?;

        let mint_entry = vault_registry.get_mint_entry(&st_mint).unwrap();

        assert_eq!(mint_entry.st_mint(), st_mint);
        assert_eq!(mint_entry.ncn_fee_group(), ncn_fee_group.unwrap());
        assert_eq!(
            mint_entry.reward_multiplier_bps(),
            reward_multiplier_bps.unwrap()
        );
        assert_eq!(mint_entry.switchboard_feed(), switchboard_feed.unwrap());
        assert_eq!(mint_entry.no_feed_weight(), no_feed_weight.unwrap());

        Ok(())
    }
}
