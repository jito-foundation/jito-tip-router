#[cfg(test)]
mod tests {
    use std::u64;

    use jito_tip_router_core::{base_fee_group::BaseFeeGroup, ncn_fee_group::NcnFeeGroup};
    use solana_sdk::pubkey::Pubkey;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_set_config_fees_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        const NEW_BLOCK_ENGINE_FEE: u16 = 500;
        const NEW_BASE_FEE: u16 = 600;
        const NEW_NCN_FEE: u16 = 700;

        let new_base_fee_wallet = Pubkey::new_unique();

        // Initialize config first - note that ncn_admin is now required as signer
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        let base_fee_group = BaseFeeGroup::default();
        let ncn_fee_group = NcnFeeGroup::default();

        // Change fees and fee wallet
        tip_router_client
            .do_set_config_fees(
                Some(NEW_BLOCK_ENGINE_FEE),
                Some(base_fee_group),
                Some(new_base_fee_wallet),
                Some(NEW_BASE_FEE),
                Some(ncn_fee_group),
                Some(NEW_NCN_FEE),
                &ncn_root,
            )
            .await?;

        let ncn_config = tip_router_client
            .get_ncn_config(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(
            ncn_config.fee_config.block_engine_fee_bps(),
            NEW_BLOCK_ENGINE_FEE
        );

        assert_eq!(
            *ncn_config
                .fee_config
                .base_fee_wallet(base_fee_group)
                .unwrap(),
            new_base_fee_wallet
        );

        let current_fees = ncn_config.fee_config.current_fees(u64::MAX);

        assert_eq!(
            current_fees.base_fee_bps(base_fee_group).unwrap(),
            NEW_BASE_FEE
        );

        assert_eq!(
            current_fees.ncn_fee_bps(ncn_fee_group).unwrap(),
            NEW_NCN_FEE
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_set_config_all_fees_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let ncn_root = fixture.setup_ncn().await?;

        const NEW_BLOCK_ENGINE_FEE: u16 = 500;
        // 10_000 total, base 5000 / 8 = 625
        const NEW_BASE_FEE: u16 = 625;
        const NEW_NCN_FEE: u16 = 625;

        let new_base_fee_wallet = Pubkey::new_unique();

        // Initialize config first - note that ncn_admin is now required as signer
        tip_router_client
            .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;

        // Change fees and fee wallet
        tip_router_client
            .do_set_config_fees(
                Some(NEW_BLOCK_ENGINE_FEE),
                None,
                None,
                None,
                None,
                None,
                &ncn_root,
            )
            .await?;

        for group in BaseFeeGroup::all_groups().iter() {
            // Change fees and fee wallet
            tip_router_client
                .do_set_config_fees(
                    None,
                    Some(*group),
                    Some(new_base_fee_wallet),
                    Some(NEW_BASE_FEE),
                    None,
                    None,
                    &ncn_root,
                )
                .await?;
        }

        for group in NcnFeeGroup::all_groups().iter() {
            // Change fees and fee wallet
            tip_router_client
                .do_set_config_fees(
                    None,
                    None,
                    None,
                    None,
                    Some(*group),
                    Some(NEW_NCN_FEE),
                    &ncn_root,
                )
                .await?;
        }

        let ncn_config = tip_router_client
            .get_ncn_config(ncn_root.ncn_pubkey)
            .await?;

        assert_eq!(
            ncn_config.fee_config.block_engine_fee_bps(),
            NEW_BLOCK_ENGINE_FEE
        );

        let current_fees = ncn_config.fee_config.current_fees(u64::MAX);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(
                *ncn_config.fee_config.base_fee_wallet(*group).unwrap(),
                new_base_fee_wallet
            );

            assert_eq!(current_fees.base_fee_bps(*group).unwrap(), NEW_BASE_FEE);
        }

        for group in NcnFeeGroup::all_groups().iter() {
            assert_eq!(current_fees.ncn_fee_bps(*group).unwrap(), NEW_NCN_FEE);
        }

        Ok(())
    }
}
