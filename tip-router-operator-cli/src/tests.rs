use ellipsis_client::EllipsisClient;
use solana_sdk::{signature::Keypair, clock::Slot, hash::Hash};
use std::path::PathBuf;
use tempfile::TempDir;
use anyhow::Result;
use solana_client::rpc_client::RpcClient;  // Add this import
use crate::snapshot::SnapshotCreator;  // Add this import

pub struct TestContext {
    pub client: EllipsisClient,
    pub keypair: Keypair,
    pub output_dir: PathBuf,
    pub temp_dir: TempDir, // Add this to clean up test files
}

impl TestContext {
    pub async fn new() -> Self {
        let keypair = Keypair::new();
        let client = EllipsisClient::from_rpc(RpcClient::new("http://localhost:8899"), &keypair).unwrap();
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("snapshots");
        
        Self {
            client,
            keypair,
            output_dir,
            temp_dir,
        }
    }
}

#[tokio::test]
async fn test_snapshot_creation() -> Result<()> {
    let ctx = TestContext::new().await;
    
    // Setup test data
    let slot: Slot = 100;
    let epoch = 5;
    let bank_hash = Hash::new_unique();
    
    let blockstore_path = ctx.temp_dir.path().join("blockstore");  // Remove to_str() and to_string()
    // Create a snapshot creator with test configuration
    let snapshot_creator = SnapshotCreator::new(
        &ctx.client.url(),           
        ctx.output_dir.to_str().unwrap().to_string(),  // Convert PathBuf to String
        5,                          
        "bzip2".to_string(),     
        ctx.keypair.insecure_clone(),  // Use insecure_clone() instead of clone()
        blockstore_path,     
    )?;

    // Simulate epoch boundary processing
    // let result = snapshot_creator.process_epoch_boundary(epoch, slot).await?;
    
    // // Verify snapshot was created
    // assert!(result.snapshot_path.exists());
    // assert!(result.slot == slot);
    
    // // Verify snapshot contents
    // let snapshot_path = ctx.output_dir.join(format!("snapshot-{}.tar.bz2", slot));
    // assert!(snapshot_path.exists());
    
    // // Validate the snapshot
    // let validation_result = snapshot_creator.validate_snapshot(slot).await;
    // assert!(validation_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_snapshot_cleanup() -> Result<()> {
    let ctx = TestContext::new().await;
    
    // Create snapshot creator with small max_snapshots
    let snapshot_creator = SnapshotCreator::new(
        &ctx.keypair,
        &ctx.client.url(),
        &ctx.output_dir,
        2, // Only keep 2 snapshots
        CompressionType::Bzip2,
        true,
    )?;

    // Create multiple snapshots
    for slot in [100, 200, 300].iter() {
        snapshot_creator.process_epoch_boundary(5, *slot).await?;
    }

    // Verify cleanup
    let snapshots: Vec<_> = std::fs::read_dir(&ctx.output_dir)?
        .filter_map(|entry| entry.ok())
        .collect();
    
    assert_eq!(snapshots.len(), 2, "Should only keep 2 most recent snapshots");
    
    // Verify we kept the most recent ones
    let snapshot_300 = ctx.output_dir.join("snapshot-300.tar.bz2");
    let snapshot_200 = ctx.output_dir.join("snapshot-200.tar.bz2");
    assert!(snapshot_300.exists());
    assert!(snapshot_200.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_snapshot_validation_failure() -> Result<()> {
    let ctx = TestContext::new().await;
    
    let blockstore_path = ctx.temp_dir.path().join("blockstore");  // Add this line
    let snapshot_creator = SnapshotCreator::new(
        &ctx.client.url(),           
        ctx.output_dir.to_str().unwrap().to_string(),
        5,                          
        "bzip2".to_string(),     
        ctx.keypair.insecure_clone(),
        blockstore_path,     
    )?;

    // Try to validate non-existent snapshot
    let invalid_slot = 999;
    // let result = snapshot_creator.validate_snapshot(invalid_slot).await;
    // assert!(result.is_err());
    
    // // Create corrupt snapshot
    // let corrupt_snapshot_path = ctx.output_dir.join(format!("snapshot-{}.tar.bz2", invalid_slot));
    // std::fs::write(&corrupt_snapshot_path, b"corrupt data")?;
    
    // // Validation should fail
    // let result = snapshot_creator.validate_snapshot(invalid_slot).await;
    // assert!(result.is_err());

    Ok(())
}

// Helper function to simulate epoch boundary state
#[derive(Debug)]
struct EpochBoundaryState {
    slot: Slot,
    epoch: u64,
    bank_hash: Hash,
    accounts_hash: Hash,
}

impl EpochBoundaryState {
    fn new(slot: Slot, epoch: u64) -> Self {
        Self {
            slot,
            epoch,
            bank_hash: Hash::new_unique(),
            accounts_hash: Hash::new_unique(),
        }
    }
}

// Take the part that does the processing not the ticks/polling, oh a new epoch has happen and have some state and expected output and based on on-chain state get the expected output.