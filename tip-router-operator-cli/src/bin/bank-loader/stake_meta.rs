use {
    anyhow::{anyhow, Context, Result},
    log::info,
    meta_merkle_tree::generated_merkle_tree::StakeMetaCollection,
    solana_runtime::bank::Bank,
    solana_sdk::pubkey::Pubkey,
    std::{
        fs::File,
        io::{BufWriter, Write},
        path::{Path, PathBuf},
        sync::Arc,
        time::Instant,
    },
    tip_router_operator_cli::{
        stake_meta_file_name, stake_meta_generator::generate_stake_meta_collection_with_stats,
    },
};

use crate::cli::StakeMetaCluster;

#[derive(Debug)]
pub(crate) struct StakeMetaConfig {
    pub(crate) output_dir: PathBuf,
    pub(crate) cluster: StakeMetaCluster,
    pub(crate) tip_distribution_program_id: Pubkey,
    pub(crate) priority_fee_distribution_program_id: Pubkey,
    pub(crate) tip_payment_program_id: Pubkey,
}

pub(crate) fn generate(bank: Bank, config: &StakeMetaConfig) -> Result<StakeMetaCollection> {
    let bank = Arc::new(bank);
    let started = Instant::now();
    info!(
        "stake_meta_cluster: {} tip_distribution_program_id: {} priority_fee_distribution_program_id: {} tip_payment_program_id: {}",
        config.cluster,
        config.tip_distribution_program_id,
        config.priority_fee_distribution_program_id,
        config.tip_payment_program_id
    );

    let stake_meta_collection = generate_stake_meta_collection_with_stats(
        &bank,
        &config.tip_distribution_program_id,
        &config.priority_fee_distribution_program_id,
        &config.tip_payment_program_id,
    )
    .map_err(|error| anyhow!("{error:?}"))?;

    info!("stake_meta_duration_ms: {}", started.elapsed().as_millis());

    let write_started = Instant::now();
    let output_path = write_stake_meta_collection(&stake_meta_collection, &config.output_dir)?;
    info!(
        "stake_meta_write_duration_ms: {} output_path: {}",
        write_started.elapsed().as_millis(),
        output_path.display()
    );
    info!(
        "Created StakeMetaCollection: epoch: {} slot: {} num_stake_metas: {} bank_hash: {}",
        stake_meta_collection.epoch,
        stake_meta_collection.slot,
        stake_meta_collection.stake_metas.len(),
        stake_meta_collection.bank_hash
    );

    Ok(stake_meta_collection)
}

fn write_stake_meta_collection(
    stake_meta_collection: &StakeMetaCollection,
    output_dir: &Path,
) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create stake meta output dir {}",
            output_dir.display()
        )
    })?;

    let output_path = output_dir.join(stake_meta_file_name(stake_meta_collection.epoch));
    let file = File::create(&output_path).with_context(|| {
        format!(
            "failed to create stake meta output file {}",
            output_path.display()
        )
    })?;
    // serde_json streams each JSON token as a separate write call; without buffering every
    // token is its own write(2) syscall (~tens of millions for 1.5M delegations).
    let mut writer = BufWriter::with_capacity(1 << 20, file);
    serde_json::to_writer(&mut writer, stake_meta_collection).with_context(|| {
        format!(
            "failed to write stake meta collection to {}",
            output_path.display()
        )
    })?;
    writer.flush().with_context(|| {
        format!(
            "failed to flush stake meta collection to {}",
            output_path.display()
        )
    })?;

    Ok(output_path)
}
