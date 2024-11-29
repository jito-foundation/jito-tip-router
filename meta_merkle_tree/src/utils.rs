use crate::merkle_tree::MerkleTree;

pub fn get_proof(merkle_tree: &MerkleTree, index: usize) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let path = merkle_tree.find_path(index).expect("path to index");
    for branch in path.get_proof_entries() {
        if let Some(hash) = branch.get_left_sibling() {
            proof.push(hash.to_bytes());
        } else if let Some(hash) = branch.get_right_sibling() {
            proof.push(hash.to_bytes());
        } else {
            panic!("expected some hash at each level of the tree");
        }
    }
    proof
}
