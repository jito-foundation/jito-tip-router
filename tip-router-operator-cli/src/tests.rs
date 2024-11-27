use ellipsis_client::EllipsisClient;
use solana_sdk::{signature::Keypair, clock::Slot, hash::Hash};
use std::path::PathBuf;
use tempfile::TempDir;
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use crate::snapshot::SnapshotCreator;
use std::fs;

// Take the part that does the processing not the ticks/polling, oh a new epoch has happen and have some state and expected output and based on on-chain state get the expected output.

pub struct TestContext {
    pub client: EllipsisClient,
    pub keypair: Keypair,
    pub output_dir: PathBuf,
    pub temp_dir: TempDir,
    pub blockstore_dir: PathBuf,
}

impl TestContext {
    pub async fn new() -> Self {
        let keypair = Keypair::new();
        let client = EllipsisClient::from_rpc(RpcClient::new("http://localhost:8899"), &keypair).unwrap();
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("snapshots");
        let blockstore_dir = temp_dir.path().join("blockstore");
        
        // Create directories
        fs::create_dir_all(&output_dir).unwrap();
        fs::create_dir_all(&blockstore_dir).unwrap();
        
        Self {
            client,
            keypair,
            output_dir,
            temp_dir,
            blockstore_dir,
        }
    }

    pub fn create_dummy_snapshot(&self, slot: Slot) -> Result<()> {
        let snapshot_path = self.output_dir.join(format!("snapshot-{}.tar.zstd", slot));
        fs::write(snapshot_path, b"dummy snapshot data")?;
        Ok(())
    }
}

#[tokio::test]
async fn test_snapshot_creator_initialization() -> Result<()> {
    let ctx = TestContext::new().await;
    
    let snapshot_creator = SnapshotCreator::new(
        &ctx.client.url(),           
        ctx.output_dir.to_str().unwrap().to_string(),
        5,                          
        "zstd".to_string(),     
        ctx.keypair.insecure_clone(),
        ctx.blockstore_dir,     
    )?;

    assert!(snapshot_creator.get_output_dir().exists());
    Ok(())
}

#[tokio::test]
async fn test_snapshot_cleanup() -> Result<()> {
    let ctx = TestContext::new().await;
    
    // Create snapshot creator with max 2 snapshots
    let snapshot_creator = SnapshotCreator::new(
        &ctx.client.url(),           
        ctx.output_dir.to_str().unwrap().to_string(),
        2,                          
        "zstd".to_string(),     
        ctx.keypair.insecure_clone(),
        ctx.blockstore_dir.clone(),     // Add .clone() here
    )?;

    // Create dummy snapshots
    ctx.create_dummy_snapshot(100)?;
    ctx.create_dummy_snapshot(200)?;
    ctx.create_dummy_snapshot(300)?;

    // Trigger cleanup
    snapshot_creator.cleanup_old_snapshots().await?;

    // Verify only 2 most recent snapshots remain
    let snapshots: Vec<_> = fs::read_dir(&ctx.output_dir)?
        .filter_map(|entry| entry.ok())
        .collect();
    
    assert_eq!(snapshots.len(), 2, "Should only keep 2 most recent snapshots");
    
    // Verify we kept the most recent ones
    let snapshot_300 = ctx.output_dir.join("snapshot-300.tar.zstd");
    let snapshot_200 = ctx.output_dir.join("snapshot-200.tar.zstd");
    assert!(snapshot_300.exists());
    assert!(snapshot_200.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_snapshot_validation_failure() -> Result<()> {
    let ctx = TestContext::new().await;
    
    let snapshot_creator = SnapshotCreator::new(
        &ctx.client.url(),           
        ctx.output_dir.to_str().unwrap().to_string(),
        5,                          
        "zstd".to_string(),     
        ctx.keypair.insecure_clone(),
        ctx.blockstore_dir,     
    )?;

    // Try to validate non-existent snapshot
    let invalid_slot = 999;
    let result = snapshot_creator.validate_snapshot(invalid_slot).await;
    assert!(result.is_err());
    
    // Create corrupt snapshot
    let corrupt_snapshot_path = ctx.output_dir.join(format!("snapshot-{}.tar.zstd", invalid_slot));
    fs::write(&corrupt_snapshot_path, b"corrupt data")?;
    
    // Validation should fail for corrupt snapshot
    let result = snapshot_creator.validate_snapshot(invalid_slot).await;
    assert!(result.is_err());

    Ok(())
}