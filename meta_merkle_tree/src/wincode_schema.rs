use solana_program::{hash::Hash, pubkey::Pubkey};
use wincode::{SchemaRead, SchemaWrite};

wincode::pod_wrapper! {
    unsafe struct PodPubkey(Pubkey);
    unsafe struct PodHash(Hash);
}

#[derive(SchemaWrite, SchemaRead)]
#[wincode(from = "crate::generated_merkle_tree::GeneratedMerkleTreeCollection")]
pub struct CollectionW {
    generated_merkle_trees: Vec<TreeW>,
    bank_hash: String,
    epoch: u64,
    slot: u64,
}

#[derive(SchemaWrite, SchemaRead)]
#[wincode(from = "crate::generated_merkle_tree::GeneratedMerkleTree")]
pub struct TreeW {
    distribution_program: PodPubkey,
    distribution_account: PodPubkey,
    merkle_root_upload_authority: PodPubkey,
    merkle_root: PodHash,
    tree_nodes: Vec<NodeW>,
    max_total_claim: u64,
    max_num_nodes: u64,
}

#[derive(SchemaWrite, SchemaRead)]
#[wincode(from = "crate::generated_merkle_tree::TreeNode")]
pub struct NodeW {
    claimant: PodPubkey,
    claim_status_pubkey: PodPubkey,
    claim_status_bump: u8,
    staker_pubkey: PodPubkey,
    withdrawer_pubkey: PodPubkey,
    amount: u64,
    proof: Option<Vec<[u8; 32]>>,
}
