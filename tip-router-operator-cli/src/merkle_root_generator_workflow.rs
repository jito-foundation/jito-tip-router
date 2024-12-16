use std::fmt::Debug;

use log::*;
use meta_merkle_tree::generated_merkle_tree::{
    GeneratedMerkleTreeCollection, MerkleRootGeneratorError, StakeMetaCollection,
};
use thiserror::Error;

pub async fn generate_merkle_root(
    stake_meta_coll: StakeMetaCollection,
    protocol_fee_bps: u16,
) -> Result<GeneratedMerkleTreeCollection, MerkleRootGeneratorError> {
    let merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
        stake_meta_coll,
        protocol_fee_bps,
    )?;

    Ok(merkle_tree_coll)
}
