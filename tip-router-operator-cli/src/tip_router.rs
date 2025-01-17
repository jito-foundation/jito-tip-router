use anyhow::Result;
use ellipsis_client::{ClientSubset, EllipsisClient, EllipsisClientResult};
use jito_bytemuck::AccountDeserialize;
use jito_tip_distribution_sdk::{
    derive_config_account_address, jito_tip_distribution::accounts::TipDistributionAccount,
};
use jito_tip_router_client::instructions::{CastVoteBuilder, SetMerkleRootBuilder};
use jito_tip_router_core::{
    ballot_box::BallotBox,
    config::Config,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
};
use log::info;
use meta_merkle_tree::meta_merkle_tree::MetaMerkleTree;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

/// Fetch and deserialize
pub async fn get_ncn_config(
    client: &EllipsisClient,
    tip_router_program_id: &Pubkey,
    ncn_pubkey: &Pubkey,
) -> Result<Config> {
    let config_pda = Config::find_program_address(tip_router_program_id, ncn_pubkey).0;
    let config = client.get_account(&config_pda).await?;
    Ok(*Config::try_from_slice_unchecked(config.data.as_slice()).unwrap())
}

/// Generate and send a CastVote instruction with the merkle root.
pub async fn cast_vote(
    client: &EllipsisClient,
    _payer: &Keypair,
    tip_router_program_id: &Pubkey,
    ncn: Pubkey,
    operator: Pubkey,
    operator_admin: &Keypair,
    meta_merkle_root: [u8; 32],
    epoch: u64,
) -> EllipsisClientResult<Signature> {
    let epoch_state = EpochState::find_program_address(tip_router_program_id, &ncn, epoch).0;

    let ncn_config = Config::find_program_address(tip_router_program_id, &ncn).0;

    let ballot_box = BallotBox::find_program_address(tip_router_program_id, &ncn, epoch).0;

    let epoch_snapshot = EpochSnapshot::find_program_address(tip_router_program_id, &ncn, epoch).0;

    let operator_snapshot =
        OperatorSnapshot::find_program_address(tip_router_program_id, &operator, &ncn, epoch).0;

    let _ix = CastVoteBuilder::new()
        .epoch_state(epoch_state)
        .config(ncn_config)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .epoch_snapshot(epoch_snapshot)
        .operator_snapshot(operator_snapshot)
        .operator(operator)
        .operator_admin(operator_admin.pubkey())
        .meta_merkle_root(meta_merkle_root)
        .epoch(epoch)
        .instruction();

    // Until we actually want to start voting on live or test NCN
    let ix = spl_memo::build_memo(&meta_merkle_root.to_vec(), &[&operator_admin.pubkey()]);
    info!("Submitting meta merkle root {:?}", meta_merkle_root);

    let tx = Transaction::new_with_payer(&[ix], Some(&operator_admin.pubkey()));
    client.process_transaction(tx, &[operator_admin]).await
}

pub async fn set_merkle_roots_batched(
    client: &EllipsisClient,
    ncn_address: &Pubkey,
    tip_distribution_program: &Pubkey,
    epoch: u64,
    tip_distribution_accounts: Vec<(Pubkey, TipDistributionAccount)>,
    meta_merkle_tree: MetaMerkleTree,
) -> Result<Vec<Signature>> {
    let ballot_box =
        BallotBox::find_program_address(&jito_tip_router_program::id(), ncn_address, epoch).0;

    let config = Config::find_program_address(&jito_tip_router_program::id(), ncn_address).0;

    let tip_distribution_config =
        derive_config_account_address(&jito_tip_distribution_sdk::jito_tip_distribution::ID).0;

    // Given a list of target TipDistributionAccounts and a meta merkle tree, fetch each meta merkle root, create its instruction, and call set_merkle_root
    let instructions = tip_distribution_accounts
        .iter()
        .filter_map(|(key, tip_distribution_account)| {
            let meta_merkle_node = meta_merkle_tree.get_node(key);

            let proof = if let Some(proof) = meta_merkle_node.proof {
                proof
            } else {
                // TODO emit big warning NO PROOF
                return None;
            };

            let vote_account = tip_distribution_account.validator_vote_account;

            let ix = SetMerkleRootBuilder::new()
                .config(config)
                .ncn(*ncn_address)
                .ballot_box(ballot_box)
                .vote_account(vote_account)
                .tip_distribution_account(*key)
                .tip_distribution_config(tip_distribution_config)
                .tip_distribution_program(*tip_distribution_program)
                .restaking_program(jito_restaking_program::id())
                .proof(proof)
                .merkle_root(meta_merkle_node.validator_merkle_root)
                .max_total_claim(meta_merkle_node.max_total_claim)
                .max_num_nodes(meta_merkle_node.max_num_nodes)
                .epoch(epoch)
                .instruction();
            Some(ix)
        })
        .collect::<Vec<_>>();

    // Parallel submit instructions

    Ok(vec![])
}
