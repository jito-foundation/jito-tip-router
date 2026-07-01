use {
    anyhow::{anyhow, Context, Result},
    log::info,
    meta_merkle_tree::generated_merkle_tree::StakeMetaCollection,
    solana_runtime::bank::Bank,
    std::{
        fs::File,
        path::{Path, PathBuf},
        sync::Arc,
        time::Instant,
    },
    tip_router_operator_cli::{
        stake_meta_file_name, stake_meta_generator::generate_stake_meta_collection_with_stats,
    },
};

#[derive(Debug)]
pub(crate) struct StakeMetaConfig {
    pub(crate) output_dir: PathBuf,
}

pub(crate) fn generate(bank: Bank, config: &StakeMetaConfig) -> Result<StakeMetaCollection> {
    let bank = Arc::new(bank);
    let started = Instant::now();

    let stake_meta_collection = generate_stake_meta_collection_with_stats(
        &bank,
        &jito_tip_distribution_sdk::id(),
        &jito_priority_fee_distribution_sdk::id(),
        &jito_tip_payment_sdk::id(),
    )
    .map_err(|error| anyhow!("{error:?}"))?;

    info!("stake_meta_duration_ms: {}", started.elapsed().as_millis());

    let _output_path = write_stake_meta_collection(&stake_meta_collection, &config.output_dir)?;
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
    serde_json::to_writer_pretty(file, stake_meta_collection).with_context(|| {
        format!(
            "failed to write stake meta collection to {}",
            output_path.display()
        )
    })?;

    Ok(output_path)
}
