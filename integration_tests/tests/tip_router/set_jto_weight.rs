#[cfg(test)]
mod tests {

    use jito_tip_router_core::constants::JTO_USD_FEED;

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_switchboard_feed() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        let test_account = fixture.get_account(&JTO_USD_FEED).await?;

        println!("{:?}", test_account);

        assert!(false);
        Ok(())
    }
}
