use anyhow::{anyhow, Result};
use solana_client::{client_error::ClientErrorKind, rpc_client::RpcClient};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::{thread::sleep, time::Duration};

pub fn catchup(rpc_url: String, our_localhost_port: u16) -> Result<String> {
    let rpc_client = RpcClient::new(rpc_url.clone());
    let config = CommitmentConfig::default();
    let node_json_rpc_url: Option<String> = Some(format!("http://localhost:{our_localhost_port}"));

    let sleep_interval = Duration::from_secs(5);
    println!("Connecting...");

    let node_client = RpcClient::new(node_json_rpc_url.unwrap());
    let node_pubkey = node_client.get_identity()?;

    let reported_node_pubkey = loop {
        match node_client.get_identity() {
            Ok(reported_node_pubkey) => break reported_node_pubkey,
            Err(err) => {
                if let ClientErrorKind::Reqwest(err) = err.kind() {
                    println!("Connection failed: {err}");
                    sleep(sleep_interval);
                    continue;
                }
                return Err(anyhow!("Failed to get node identity: {}", err));
            }
        }
    };

    if reported_node_pubkey != node_pubkey {
        return Err(anyhow!(
            "The identity reported by node RPC URL does not match. Expected: {node_pubkey:?}. Reported: {reported_node_pubkey:?}"
        ));
    }

    if rpc_client.get_identity()? == node_pubkey {
        return Err(anyhow!(
            "Both RPC URLs reference the same node, unable to monitor for catchup. Try a different --url"
        ));
    }

    let mut previous_rpc_slot = i64::MAX;
    let mut previous_slot_distance: i64 = 0;
    let mut retry_count: u64 = 0;
    let max_retry_count = 5;
    let mut get_slot_while_retrying = |client: &RpcClient| loop {
        match client.get_slot_with_commitment(config) {
            Ok(r) => {
                retry_count = 0;
                return Ok(r);
            }
            Err(e) => {
                if retry_count >= max_retry_count {
                    return Err(e);
                }
                retry_count = retry_count.saturating_add(1);
                sleep(Duration::from_secs(1));
            }
        };
    };

    let start_node_slot: i64 = get_slot_while_retrying(&node_client)?.try_into()?;
    let start_rpc_slot: i64 = get_slot_while_retrying(&rpc_client)?.try_into()?;
    let start_slot_distance = start_rpc_slot.saturating_sub(start_node_slot);
    let mut total_sleep_interval = Duration::ZERO;

    loop {
        let rpc_slot: i64 = get_slot_while_retrying(&rpc_client)?.try_into()?;
        let node_slot: i64 = get_slot_while_retrying(&node_client)?.try_into()?;

        if node_slot > std::cmp::min(previous_rpc_slot, rpc_slot) {
            return Ok(format!(
                "{node_pubkey} has caught up (us:{node_slot} them:{rpc_slot})",
            ));
        }

        let slot_distance = rpc_slot.saturating_sub(node_slot);
        let slots_per_second = previous_slot_distance.saturating_sub(slot_distance) as f64
            / sleep_interval.as_secs_f64();

        let average_time_remaining = if slot_distance == 0 || total_sleep_interval.is_zero() {
            "".to_string()
        } else {
            let distance_delta = start_slot_distance.saturating_sub(slot_distance);
            let average_catchup_slots_per_second =
                distance_delta as f64 / total_sleep_interval.as_secs_f64();
            let average_time_remaining =
                (slot_distance as f64 / average_catchup_slots_per_second).round();
            if !average_time_remaining.is_normal() {
                "".to_string()
            } else if average_time_remaining < 0.0 {
                format!(" (AVG: {average_catchup_slots_per_second:.1} slots/second (falling))")
            } else {
                let total_node_slot_delta = node_slot.saturating_sub(start_node_slot);
                let average_node_slots_per_second =
                    total_node_slot_delta as f64 / total_sleep_interval.as_secs_f64();
                let expected_finish_slot = (node_slot as f64
                    + average_time_remaining * average_node_slots_per_second)
                    .round();
                format!(
                    " (AVG: {:.1} slots/second, ETA: slot {} in {:?})",
                    average_catchup_slots_per_second,
                    expected_finish_slot,
                    Duration::from_secs_f64(average_time_remaining)
                )
            }
        };

        println!(
            "{} slot(s) {} (us:{} them:{}){}",
            slot_distance.abs(),
            if slot_distance >= 0 {
                "behind"
            } else {
                "ahead"
            },
            node_slot,
            rpc_slot,
            if slot_distance == 0 || previous_rpc_slot == i64::MAX {
                "".to_string()
            } else {
                format!(
                    ", {} node is {} at {:.1} slots/second{}",
                    if slot_distance >= 0 { "our" } else { "their" },
                    if slots_per_second < 0.0 {
                        "falling behind"
                    } else {
                        "gaining"
                    },
                    slots_per_second,
                    average_time_remaining
                )
            },
        );

        sleep(sleep_interval);
        previous_rpc_slot = rpc_slot;
        previous_slot_distance = slot_distance;
        total_sleep_interval = total_sleep_interval.saturating_add(sleep_interval);
    }
}
