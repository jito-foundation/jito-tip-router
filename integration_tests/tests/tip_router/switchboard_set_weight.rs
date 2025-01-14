#[cfg(test)]
mod tests {

    use crate::fixtures::{test_builder::TestBuilder, TestResult};

    #[tokio::test]
    async fn test_switchboard_set_weight_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;

        const OPERATOR_COUNT: usize = 1;
        const VAULT_COUNT: usize = 1;

        let test_ncn = fixture
            .create_initial_test_ncn(OPERATOR_COUNT, VAULT_COUNT, None)
            .await?;

        fixture.add_epoch_state_for_test_ncn(&test_ncn).await?;

        fixture
            .add_switchboard_weights_for_test_ncn(&test_ncn)
            .await?;

        Ok(())
    }
}
