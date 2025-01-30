use std::{path::PathBuf, str::FromStr};

use anchor_lang::AccountDeserialize;
use ellipsis_client::EllipsisClient;
use jito_bytemuck::AccountDeserialize as JitoAccountDeserialize;
use jito_tip_distribution_sdk::{derive_config_account_address, TipDistributionAccount};
use jito_tip_router_core::ballot_box::BallotBox;
use log::{debug, error, info};
use meta_merkle_tree::meta_merkle_tree::MetaMerkleTree;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

use crate::{
    tip_router::{cast_vote, get_ncn_config, set_merkle_roots_batched},
    Cli,
};

pub async fn submit_recent_epochs_to_ncn(
    client: &EllipsisClient,
    keypair: &Keypair,
    ncn_address: &Pubkey,
    tip_router_program_id: &Pubkey,
    tip_distribution_program_id: &Pubkey,
    num_monitored_epochs: u64,
    cli_args: &Cli,
) -> Result<(), anyhow::Error> {
    let epoch = client.get_epoch_info()?;
    let operator_address = Pubkey::from_str(&cli_args.operator_address)?;

    for i in 0..num_monitored_epochs {
        let process_epoch = epoch.epoch.checked_sub(i).unwrap();

        let meta_merkle_tree_dir = cli_args.meta_merkle_tree_dir.clone();
        let target_meta_merkle_tree_file = format!("meta_merkle_tree_{}.json", process_epoch);
        let target_meta_merkle_tree_path = meta_merkle_tree_dir.join(target_meta_merkle_tree_file);
        if !target_meta_merkle_tree_path.exists() {
            continue;
        }

        match submit_to_ncn(
            client,
            keypair,
            &operator_address,
            &target_meta_merkle_tree_path,
            process_epoch,
            ncn_address,
            tip_router_program_id,
            tip_distribution_program_id,
            cli_args.submit_as_memo,
        )
        .await
        {
            Ok(_) => {}
            Err(e) => error!("Failed to submit epoch {} to NCN: {:?}", process_epoch, e),
        }
    }

    Ok(())
}

pub async fn submit_to_ncn(
    client: &EllipsisClient,
    keypair: &Keypair,
    operator_address: &Pubkey,
    meta_merkle_tree_path: &PathBuf,
    merkle_root_epoch: u64,
    ncn_address: &Pubkey,
    tip_router_program_id: &Pubkey,
    tip_distribution_program_id: &Pubkey,
    submit_as_memo: bool,
) -> Result<(), anyhow::Error> {
    let epoch_info = client.get_epoch_info()?;
    let meta_merkle_tree = MetaMerkleTree::new_from_file(meta_merkle_tree_path)?;
    let config = get_ncn_config(client, tip_router_program_id, ncn_address).await?;

    // The meta merkle root files are tagged with the epoch they have created the snapshot for
    // Tip router accounts for that merkle root are created in the next epoch
    let tip_router_target_epoch = merkle_root_epoch + 1;

    // Check for ballot box
    let ballot_box_address = BallotBox::find_program_address(
        tip_router_program_id,
        ncn_address,
        tip_router_target_epoch,
    )
    .0;
    info!("Found ballot box address: {}", ballot_box_address);

    let ballot_box_account = match client.get_account(&ballot_box_address).await {
        Ok(account) => account,
        Err(e) => {
            info!(
                "Ballot box not created yet for epoch {}: {:?}",
                tip_router_target_epoch, e
            );
            return Ok(());
        }
    };
    info!("Retrieved ballot box account");

    let ballot_box = BallotBox::try_from_slice_unchecked(&ballot_box_account.data)?;
    info!("Deserialized ballot box data");

    let is_voting_valid = ballot_box.is_voting_valid(
        epoch_info.absolute_slot,
        config.valid_slots_after_consensus(),
    )?;
    info!("Voting validity checked: {}", is_voting_valid);

    // If exists, look for vote from current operator
    let vote = ballot_box
        .operator_votes()
        .iter()
        .find(|vote| vote.operator() == operator_address);
    info!("Checked for existing operator vote: {}", vote.is_some());

    let should_cast_vote = match vote {
        Some(vote) => {
            // If vote exists, cast_vote if different from current meta_merkle_root
            let tally = ballot_box
                .ballot_tallies()
                .get(u16::from(vote.ballot_index()) as usize)
                .ok_or(anyhow::anyhow!("Ballot tally not found"))?;

            tally.ballot().root() != meta_merkle_tree.merkle_root
        }
        None => true,
    };
    info!("Determined if vote should be cast: {}", should_cast_vote);

    if should_cast_vote && is_voting_valid {
        let res = cast_vote(
            client,
            keypair,
            tip_router_program_id,
            *ncn_address,
            keypair.pubkey(),
            keypair,
            meta_merkle_tree.merkle_root,
            tip_router_target_epoch,
            submit_as_memo,
        )
        .await;

        match res {
            Ok(signature) => {
                datapoint_info!(
                    "tip_router_cli.vote_cast",
                    ("operator_address", operator_address.to_string(), String),
                    ("epoch", tip_router_target_epoch, i64),
                    (
                        "merkle_root",
                        format!("{:?}", meta_merkle_tree.merkle_root),
                        String
                    ),
                    ("tx_sig", format!("{:?}", signature), String)
                );
                info!(
                    "Cast vote for epoch {} with signature {:?}",
                    tip_router_target_epoch, signature
                )
            }
            Err(e) => {
                datapoint_error!(
                    "tip_router_cli.vote_cast",
                    ("operator_address", operator_address.to_string(), String),
                    ("epoch", tip_router_target_epoch, i64),
                    (
                        "merkle_root",
                        format!("{:?}", meta_merkle_tree.merkle_root),
                        String
                    ),
                    ("status", "error", String),
                    ("error", format!("{:?}", e), String)
                );
                info!(
                    "Failed to cast vote for epoch {}: {:?}",
                    tip_router_target_epoch, e
                )
            }
        }
    }

    if ballot_box.is_consensus_reached() {
        // Fetch TipDistributionAccounts filtered by epoch and upload authority
        // Tip distribution accounts are derived from the epoch they are for
        let tip_distribution_accounts = get_tip_distribution_accounts_to_upload(
            client,
            merkle_root_epoch,
            tip_distribution_program_id,
        )
        .await?;

        // For each TipDistributionAccount returned, if it has no root uploaded, upload root with set_merkle_root
        match set_merkle_roots_batched(
            client,
            ncn_address,
            keypair,
            tip_distribution_program_id,
            tip_router_program_id,
            tip_router_target_epoch,
            tip_distribution_accounts,
            meta_merkle_tree,
        )
        .await
        {
            Ok(res) => {
                let num_success = res.iter().filter(|r| r.is_ok()).count();
                let num_failed = res.iter().filter(|r| r.is_err()).count();

                datapoint_info!(
                    "tip_router_cli.set_merkle_root",
                    ("operator_address", operator_address.to_string(), String),
                    ("epoch", tip_router_target_epoch, i64),
                    ("num_success", num_success, i64),
                    ("num_failed", num_failed, i64)
                );
                info!(
                    "Set merkle root for {} tip distribution accounts, failed for {}",
                    num_success, num_failed
                );
            }
            Err(e) => {
                datapoint_error!(
                    "tip_router_cli.set_merkle_root",
                    ("operator_address", operator_address.to_string(), String),
                    ("epoch", tip_router_target_epoch, i64),
                    ("status", "error", String),
                    ("error", format!("{:?}", e), String)
                );
                error!("Failed to set merkle roots: {:?}", e);
            }
        }
    }

    Ok(())
}

async fn get_tip_distribution_accounts_to_upload(
    client: &EllipsisClient,
    epoch: u64,

    tip_distribution_program_id: &Pubkey,
) -> Result<Vec<(Pubkey, TipDistributionAccount)>, anyhow::Error> {
    let config_address = derive_config_account_address(tip_distribution_program_id).0;

    // Filters assume merkle root is None
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8     // Discriminator
            + 32, // Pubkey - validator_vote_account
            config_address.to_bytes().to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8    // Discriminator
            + 32 // Pubkey - validator_vote_account
            + 32 // Pubkey - merkle_root_upload_authority
            + 1, // Option - "None" merkle_root
            epoch.to_le_bytes().to_vec(),
        )),
    ];

    let tip_distribution_accounts = client.get_program_accounts_with_config(
        tip_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )?;

    let tip_distribution_accounts = tip_distribution_accounts
        .into_iter()
        .filter_map(|(pubkey, account)| {
            let tip_distribution_account =
                TipDistributionAccount::try_deserialize(&mut account.data.as_slice());
            match tip_distribution_account {
                Ok(tip_distribution_account) => {
                    // Double check that GPA filter worked
                    if tip_distribution_account.epoch_created_at == epoch
                        && tip_distribution_account.merkle_root_upload_authority == config_address
                    {
                        Some((pubkey, tip_distribution_account))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        })
        .collect::<Vec<_>>();

    Ok(tip_distribution_accounts)
}
