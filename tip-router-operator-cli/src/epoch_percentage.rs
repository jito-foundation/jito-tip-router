use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;

pub async fn get(client: &RpcClient) -> Result<f64> {
    let current_slot = client.get_slot().await? as f64;
    let epoch_percentage =
        (current_slot % DEFAULT_SLOTS_PER_EPOCH as f64) / DEFAULT_SLOTS_PER_EPOCH as f64;
    Ok(epoch_percentage)
}
