use anyhow::Result;
use jito_bytemuck::AccountDeserialize;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
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
use log::{error, info};
use meta_merkle_tree::meta_merkle_tree::MetaMerkleTree;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::client_error::{ErrorKind, Result as ClientResult};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

const MAX_SET_MERKLE_ROOT_IXS_PER_TX: usize = 1;
use crate::priority_fees;

/// Fetch and deserialize
pub async fn get_ncn_config(
    client: &RpcClient,
    tip_router_program_id: &Pubkey,
    ncn_pubkey: &Pubkey,
) -> Result<Config> {
    let config_pda = Config::find_program_address(tip_router_program_id, ncn_pubkey).0;
    let config = client.get_account(&config_pda).await?;
    Ok(*Config::try_from_slice_unchecked(config.data.as_slice()).unwrap())
}

/// Generate and send a CastVote instruction with the merkle root.
#[allow(clippy::too_many_arguments)]
pub async fn cast_vote(
    client: &RpcClient,
    payer: &Keypair,
    tip_router_program_id: &Pubkey,
    ncn: &Pubkey,
    operator: &Pubkey,
    operator_voter: &Keypair,
    meta_merkle_root: [u8; 32],
    tip_router_epoch: u64,
    submit_as_memo: bool,
    compute_unit_price: u64,
) -> Result<Signature> {
    let epoch_state =
        EpochState::find_program_address(tip_router_program_id, ncn, tip_router_epoch).0;

    let ncn_config = Config::find_program_address(tip_router_program_id, ncn).0;

    let ballot_box =
        BallotBox::find_program_address(tip_router_program_id, ncn, tip_router_epoch).0;

    let epoch_snapshot =
        EpochSnapshot::find_program_address(tip_router_program_id, ncn, tip_router_epoch).0;

    let operator_snapshot = OperatorSnapshot::find_program_address(
        tip_router_program_id,
        operator,
        ncn,
        tip_router_epoch,
    )
    .0;

    let ix = if submit_as_memo {
        spl_memo::build_memo(meta_merkle_root.as_ref(), &[&operator_voter.pubkey()])
    } else {
        CastVoteBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(*ncn)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .operator(*operator)
            .operator_voter(operator_voter.pubkey())
            .meta_merkle_root(meta_merkle_root)
            .epoch(tip_router_epoch)
            .instruction()
    };

    info!("Submitting meta merkle root {:?}", meta_merkle_root);

    // Configure instruction with priority fees
    let instructions = priority_fees::configure_instruction(ix, compute_unit_price, None);

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer, operator_voter],
        client.get_latest_blockhash().await?,
    );
    Ok(client.send_and_confirm_transaction(&tx).await?)
}

#[allow(clippy::too_many_arguments)]
pub fn set_merkle_root_instructions(
    ncn_address: &Pubkey,
    distribution_program: &Pubkey,
    tip_router_program_id: &Pubkey,
    epoch: u64,
    tip_distribution_accounts: Vec<(Pubkey, TipDistributionAccount)>,
    meta_merkle_tree: &MetaMerkleTree,
) -> Vec<Instruction> {
    let ballot_box = BallotBox::find_program_address(tip_router_program_id, ncn_address, epoch).0;

    let config = Config::find_program_address(tip_router_program_id, ncn_address).0;

    let epoch_state = EpochState::find_program_address(tip_router_program_id, ncn_address, epoch).0;

    let tip_distribution_config = derive_config_account_address(distribution_program).0;

    // Given a list of target TipDistributionAccounts and a meta merkle tree, fetch each meta
    //  merkle root, create its instruction, and call set_merkle_root
    let instructions = tip_distribution_accounts
        .iter()
        .filter_map(|(key, tip_distribution_account)| {
            let meta_merkle_node = if let Some(node) = meta_merkle_tree.get_node(key) {
                node
            } else {
                error!("No node found for tip distribution account, maybe the account has zero tips? {:?}", key);
                return None;
            };

            let proof = if let Some(proof) = meta_merkle_node.proof {
                proof
            } else {
                error!("No proof found for tip distribution account {:?}", key);
                return None;
            };

            let vote_account = tip_distribution_account.validator_vote_account;

            let ix = SetMerkleRootBuilder::new()
                .epoch_state(epoch_state)
                .config(config)
                .ncn(*ncn_address)
                .ballot_box(ballot_box)
                .vote_account(vote_account)
                .tip_distribution_account(*key)
                .tip_distribution_config(tip_distribution_config)
                .tip_distribution_program(*distribution_program)
                .proof(proof)
                .merkle_root(meta_merkle_node.validator_merkle_root)
                .max_total_claim(meta_merkle_node.max_total_claim)
                .max_num_nodes(meta_merkle_node.max_num_nodes)
                .epoch(epoch)
                .instruction();

            Some(ix)
        })
        .collect::<Vec<_>>();
    instructions
}

pub async fn send_set_merkle_root_txs(
    client: &RpcClient,
    keypair: &Keypair,
    instructions: Vec<Instruction>,
) -> Result<Vec<ClientResult<Signature>>> {
    let num_of_txs = instructions.len().div_ceil(MAX_SET_MERKLE_ROOT_IXS_PER_TX);
    let mut results = Vec::with_capacity(num_of_txs);
    for _ in 0..instructions.len() {
        results.push(Err(ErrorKind::Custom(
            "Default: Failed to submit instruction".to_string(),
        )
        .into()));
    }

    for (i, ixs) in instructions
        .chunks(MAX_SET_MERKLE_ROOT_IXS_PER_TX)
        .enumerate()
    {
        // TODO: Add compute unit instructions
        let mut tx = Transaction::new_with_payer(ixs, Some(&keypair.pubkey()));
        // Simple retry logic
        for _ in 0..5 {
            let blockhash = client.get_latest_blockhash().await?;
            tx.sign(&[keypair], blockhash);
            results[i] = client
                .send_transaction_with_config(
                    &tx,
                    RpcSendTransactionConfig {
                        skip_preflight: true,
                        preflight_commitment: None,
                        encoding: None,
                        max_retries: None,
                        min_context_slot: None,
                    },
                )
                .await;

            if results[i].is_ok() {
                break;
            }
        }
    }
    Ok(results)
}

#[allow(clippy::too_many_arguments)]
pub fn set_priority_fee_merkle_root_instructions(
    ncn_address: &Pubkey,
    distribution_program: &Pubkey,
    tip_router_program_id: &Pubkey,
    epoch: u64,
    tip_distribution_accounts: Vec<(Pubkey, PriorityFeeDistributionAccount)>,
    meta_merkle_tree: &MetaMerkleTree,
) -> Vec<Instruction> {
    let ballot_box = BallotBox::find_program_address(tip_router_program_id, ncn_address, epoch).0;

    let config = Config::find_program_address(tip_router_program_id, ncn_address).0;

    let epoch_state = EpochState::find_program_address(tip_router_program_id, ncn_address, epoch).0;

    let tip_distribution_config = derive_config_account_address(distribution_program).0;

    // Given a list of target TipDistributionAccounts and a meta merkle tree, fetch each meta
    //  merkle root, create its instruction, and call set_merkle_root
    let instructions = tip_distribution_accounts
        .iter()
        .filter_map(|(key, tip_distribution_account)| {
            let meta_merkle_node = meta_merkle_tree
                .get_node(key)
                .expect("Node exists in meta merkle");

            let proof = if let Some(proof) = meta_merkle_node.proof {
                proof
            } else {
                error!("No proof found for tip distribution account {:?}", key);
                return None;
            };

            let vote_account = tip_distribution_account.validator_vote_account;

            let ix = SetMerkleRootBuilder::new()
                .epoch_state(epoch_state)
                .config(config)
                .ncn(*ncn_address)
                .ballot_box(ballot_box)
                .vote_account(vote_account)
                .tip_distribution_account(*key)
                .tip_distribution_config(tip_distribution_config)
                .tip_distribution_program(*distribution_program)
                .proof(proof)
                .merkle_root(meta_merkle_node.validator_merkle_root)
                .max_total_claim(meta_merkle_node.max_total_claim)
                .max_num_nodes(meta_merkle_node.max_num_nodes)
                .epoch(epoch)
                .instruction();

            Some(ix)
        })
        .collect::<Vec<_>>();
    instructions
}
