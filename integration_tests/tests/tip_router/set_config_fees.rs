#[cfg(test)]
mod tests {
    // use jito_tip_router_core::{error::TipRouterError, ncn_fee_group::NcnFeeGroup};
    // use solana_sdk::{
    //     clock::DEFAULT_SLOTS_PER_EPOCH,
    //     signature::{Keypair, Signer},
    // };

    // use crate::fixtures::{
    //     test_builder::TestBuilder, tip_router_client::assert_tip_router_error, TestResult,
    // };

    // #[tokio::test]
    // async fn test_set_config_fees_ok() -> TestResult<()> {
    //     let mut fixture = TestBuilder::new().await;
    //     let mut tip_router_client = fixture.tip_router_client();
    //     let ncn_root = fixture.setup_ncn().await?;

    //     // Initialize config first - note that ncn_admin is now required as signer
    //     tip_router_client
    //         .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
    //         .await?;

    //     // Change fees and fee wallet
    //     let new_fee_wallet = Keypair::new();
    //     tip_router_client
    //         .do_set_config_fees(
    //             100, // dao_fee_bps
    //             200, // ncn_fee_bps
    //             300, // block_engine_fee_bps
    //             new_fee_wallet.pubkey(),
    //             &ncn_root,
    //         )
    //         .await?;

    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_set_config_fees_exceed_max_fails() -> TestResult<()> {
    //     let mut fixture = TestBuilder::new().await;
    //     let mut tip_router_client = fixture.tip_router_client();
    //     let ncn_root = fixture.setup_ncn().await?;

    //     // Initialize config first
    //     tip_router_client
    //         .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
    //         .await?;

    //     // Try to set fees above max
    //     let transaction_error = tip_router_client
    //         .do_set_config_fees(10_001, 0, 0, ncn_root.ncn_admin.pubkey(), &ncn_root)
    //         .await;

    //     assert_tip_router_error(transaction_error, TipRouterError::FeeCapExceeded);
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_set_config_fees_wrong_admin_fails() -> TestResult<()> {
    //     let mut fixture = TestBuilder::new().await;
    //     let mut tip_router_client = fixture.tip_router_client();
    //     let mut ncn_root = fixture.setup_ncn().await?;

    //     tip_router_client
    //         .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
    //         .await?;

    //     let wrong_admin = Keypair::new();
    //     ncn_root.ncn_admin = wrong_admin;
    //     let transaction_error = tip_router_client
    //         .do_set_config_fees(100, 200, 300, ncn_root.ncn_admin.pubkey(), &ncn_root)
    //         .await;

    //     assert_tip_router_error(transaction_error, TipRouterError::IncorrectFeeAdmin);
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_set_config_fees_across_epoch() -> TestResult<()> {
    //     let mut fixture = TestBuilder::new().await;
    //     let mut tip_router_client = fixture.tip_router_client();
    //     let ncn_root = fixture.setup_ncn().await?;

    //     // Initialize config first
    //     tip_router_client
    //         .do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
    //         .await?;

    //     // Set new fees
    //     let new_fee_wallet = Keypair::new();
    //     tip_router_client
    //         .do_set_config_fees(100, 200, 0, new_fee_wallet.pubkey(), &ncn_root)
    //         .await?;

    //     // Advance epoch
    //     fixture
    //         .warp_slot_incremental(2 * DEFAULT_SLOTS_PER_EPOCH)
    //         .await?;

    //     let config = tip_router_client
    //         .get_ncn_config(ncn_root.ncn_pubkey)
    //         .await?;
    //     let clock = fixture.clock().await;
    //     assert_eq!(config.fee_config.dao_fee_bps(clock.epoch as u64), 100);
    //     assert_eq!(
    //         config
    //             .fee_config
    //             .ncn_fee_bps(NcnFeeGroup::default(), clock.epoch as u64)
    //             .unwrap(),
    //         200
    //     );
    //     assert_eq!(
    //         config.fee_config.block_engine_fee_bps(clock.epoch as u64),
    //         0
    //     );
    //     assert_eq!(config.fee_config.fee_wallet(), new_fee_wallet.pubkey());

    //     Ok(())
    // }
}
