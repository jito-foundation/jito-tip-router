#![allow(non_snake_case)]
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error as AnyhowError;
use futures::{Stream, StreamExt};
use hex;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use crate::solana_compat::ClusterType;
use crate::Pubkey;
use switchboard_utils::utils::median;
use tokio::time::interval;
use tokio::time::Duration;
use tokio_stream::wrappers::IntervalStream;
use switchboard_protos::OracleFeed;
use prost::Message;
use base64::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct StoreResponse {
    pub cid: String,
    pub feedHash: String,
    pub queueHex: String,
}

#[derive(Serialize, Deserialize)]
pub struct OracleFeedStoreResponse {
    pub cid: String,
    pub feedId: String,
}

#[derive(Serialize, Deserialize)]
pub struct FetchSolanaUpdatesResponse {
    pub success: bool,
    pub pullIx: String,
    pub responses: Vec<Response>,
    pub lookupTables: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub oracle: String,
    pub result: Option<Decimal>,
    pub errors: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulateSolanaFeedsResponse {
    pub feed: String,
    pub feedHash: String,
    pub results: Vec<Option<Decimal>>,
    #[serde(skip_deserializing, default)]
    pub result: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulateSuiFeedsResponse {
    pub feed: String,
    pub feedHash: String,
    // The TS endpoint returns the results as strings. You can choose to parse them into Decimal if desired.
    pub results: Vec<String>,
    // The result is already computed by the server; hence, no median calculation here.
    #[serde(skip_deserializing, default)]
    pub result: Option<Decimal>,
    #[serde(default)]
    pub stdev: Option<Decimal>,
    #[serde(default)]
    pub variance: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulateFeedsResponse {
    pub feedHash: String,
    pub results: Vec<Decimal>,
    #[serde(skip_deserializing, default)]
    pub result: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrossbarSimulateProtoResponse {
    pub feedHash: Option<String>,
    pub results: Vec<String>,
    #[serde(default)]
    pub logs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrossbarOracleFeedFetchResponse {
    pub data: String, // Base64 encoded proto
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SuiOracleResult {
    pub successValue: String,
    pub isNegative: bool,
    pub timestamp: u64,
    pub oracleId: String,
    #[serde(serialize_with = "bytes_to_hex", deserialize_with = "hex_to_bytes")]
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SuiFeedConfigs {
    pub feedHash: String,
    pub maxVariance: u64,
    pub minResponses: u64,
    pub minSampleSize: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SuiUpdateResponse {
    pub aggregator_id: Option<String>,
    pub results: Vec<SuiOracleResult>,
    pub feedConfigs: SuiFeedConfigs,
    pub queue: String,
    pub fee: u64,
    pub failures: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FetchSuiUpdatesResponse {
    pub responses: Vec<SuiUpdateResponse>,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CrossbarClient {
    crossbar_url: String,
    verbose: bool,
    client: Client,
}

fn hex_to_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    hex::decode(&s).map_err(DeError::custom)
}

fn bytes_to_hex<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Convert the byte vector into a hex string.
    let hex_string = hex::encode(bytes);
    serializer.serialize_str(&hex_string)
}

fn cluster_type_to_string(cluster_type: ClusterType) -> String {
    match cluster_type {
        ClusterType::MainnetBeta => "mainnet-beta",
        ClusterType::Testnet => "testnet",
        ClusterType::Devnet => "devnet",
        ClusterType::Development => "development",
    }
    .to_string()
}

impl Default for CrossbarClient {
    fn default() -> Self {
        Self::new("https://crossbar.switchboard.xyz", false)
    }
}

impl CrossbarClient {
    pub fn new(crossbar_url: &str, verbose: bool) -> Self {
        Self {
            crossbar_url: crossbar_url.to_string(),
            verbose,
            client: Client::new(),
        }
    }

    /// # Arguments
    /// * `feed_hash` - The feed hash of the jobs it performs
    /// # Returns
    /// * `Result<serde_json::Value>` - The response from the crossbar gateway,
    ///   containing the json formatted oracle jobs
    pub async fn fetch(&self, feed_hash: &str) -> Result<serde_json::Value, AnyhowError> {
        let url = format!("{}/fetch/{}", self.crossbar_url, feed_hash);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send fetch request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for feed hash '{}'. Response: {}",
                status.as_u16(),
                feed_hash,
                raw
            ));
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse fetch response for feed hash '{}'. URL: {}. Raw response (first 500 chars): {}",
                feed_hash,
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

    /// GET /v2/fetch/:feedHash
    /// Fetch OracleFeed data from a crossbar server using the provided feedId
    /// # Arguments
    /// * `feed_id` - The identifier of the OracleFeed to fetch
    /// # Returns
    /// * `Result<CrossbarOracleFeedFetchResponse, AnyhowError>` - The data fetched from the crossbar
    pub async fn fetch_oracle_feed(&self, feed_id: &str) -> Result<CrossbarOracleFeedFetchResponse, AnyhowError> {
        let url = format!("{}/v2/fetch/{}", self.crossbar_url, feed_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send v2 fetch request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for fetch_oracle_feed '{}'. Response: {}",
                status.as_u16(),
                feed_id,
                raw
            ));
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse fetch_oracle_feed response. URL: {}. Raw response (first 500 chars): {}",
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

    /// Store feed jobs in the crossbar gateway to a pinned IPFS address
    pub async fn store(
        &self,
        queue_address: Pubkey,
        jobs: &[serde_json::Value],
    ) -> Result<StoreResponse, AnyhowError> {
        let queue = bs58::decode(queue_address.to_string())
            .into_vec()
            .context("Failed to decode queue address")?;
        let queue_hex = bs58::encode(queue).into_string();
        let payload = serde_json::json!({ "queue": queue_hex, "jobs": jobs });

        let url = format!("{}/store", self.crossbar_url);
        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .header("Content-Type", "application/json")
            .send()
            .await
            .context("Failed to send store request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for store request with queue '{}'. Response: {}",
                status.as_u16(),
                queue_address,
                raw
            ));
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse store response for queue '{}'. URL: {}. Raw response (first 500 chars): {}",
                queue_address,
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

    /// POST /v2/store
    /// Store an OracleFeed on IPFS using crossbar.
    /// # Arguments
    /// * `feed` - The OracleFeed to store (as serde_json::Value matching IOracleFeed)
    /// # Returns
    /// * `Result<OracleFeedStoreResponse, AnyhowError>` - The stored data information
    pub async fn store_oracle_feed(
        &self,
        feed: &serde_json::Value,
    ) -> Result<OracleFeedStoreResponse, AnyhowError> {
        let url = format!("{}/v2/store", self.crossbar_url);
        let payload = serde_json::json!({ "feed": feed });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .header("Content-Type", "application/json")
            .send()
            .await
            .context("Failed to send v2 store request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for store_oracle_feed. Response: {}",
                status.as_u16(),
                raw
            ));
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse store_oracle_feed response. URL: {}. Raw response (first 500 chars): {}",
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

    pub async fn fetch_solana_updates(
        &self,
        network: ClusterType,
        feed_pubkeys: &[Pubkey],
        num_signatures: Option<usize>,
    ) -> Result<Vec<FetchSolanaUpdatesResponse>, AnyhowError> {
        if feed_pubkeys.is_empty() {
            return Err(anyhow!("Feed pubkeys are empty"));
        }

        let feeds_param: Vec<_> = feed_pubkeys.iter().map(|x| x.to_string()).collect();
        let feeds_param = feeds_param.join(",");
        let network_str = cluster_type_to_string(network);
        let mut url = format!(
            "{}/updates/solana/{}/{}",
            self.crossbar_url, network_str, feeds_param
        );
        if let Some(num_signatures) = num_signatures {
            url.push_str(&format!("?numSignatures={}", num_signatures));
        }

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send fetch solana updates request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for Solana feeds on network '{}'. Response: {}",
                status.as_u16(),
                network_str,
                raw
            ));
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse fetch_solana_updates response for feeds on network '{}'. URL: {}. Raw response (first 500 chars): {}",
                network_str,
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

    /// Simulate feed responses from the crossbar gateway for Solana feeds.
    /// In addition to deserializing the JSON, compute the median for each response
    /// and store it in the `result` field as an Option<Decimal>.
    pub async fn simulate_solana_feeds(
        &self,
        network: ClusterType,
        feed_pubkeys: &[Pubkey],
        include_receipts: bool,
    ) -> Result<Vec<SimulateSolanaFeedsResponse>, AnyhowError> {
        if feed_pubkeys.is_empty() {
            return Err(anyhow!("Feed pubkeys are empty"));
        }

        let feeds_param: Vec<_> = feed_pubkeys.iter().map(|x| x.to_string()).collect();
        let feeds_param = feeds_param.join(",");
        let network = cluster_type_to_string(network);
        let url = format!(
            "{}/simulate/solana/{}/{}",
            self.crossbar_url, network, feeds_param
        );
        let mut req = self.client.get(&url);
        if include_receipts {
            req = req.query(&[("includeReceipts", "true")]);
        }
        let resp = req.send().await?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response")?;
        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!("Bad status code {}", status.as_u16()));
        }

        let mut responses: Vec<SimulateSolanaFeedsResponse> = serde_json::from_str(&raw)?;
        // Compute the median result for each response
        for response in responses.iter_mut() {
            // Collect non-None decimals
            let valid: Vec<Decimal> = response.results.iter().filter_map(|x| *x).collect();
            response.result = if valid.is_empty() {
                None
            } else {
                Some(median(valid).expect("Failed to compute median"))
            };
        }
        Ok(responses)
    }

    /// Simulate feed responses from the crossbar gateway.
    /// In addition to deserializing the JSON, compute the median for each response
    /// and store it in the `result` field.
    pub async fn simulate_feeds(
        &self,
        feed_hashes: &[&str],
        include_receipts: bool,
    ) -> Result<Vec<SimulateFeedsResponse>, AnyhowError> {
        if feed_hashes.is_empty() {
            return Err(anyhow!("Feed hashes are empty"));
        }

        let feeds_param = feed_hashes.join(",");
        let url = format!("{}/simulate/{}", self.crossbar_url, feeds_param);
        let mut req = self.client.get(&url);
        if include_receipts {
            req = req.query(&[("includeReceipts", "true")]);
        }
        let resp = req
            .send()
            .await
            .context("Failed to send simulate feeds request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for feeds [{}]. Response: {}",
                status.as_u16(),
                feed_hashes.join(", "),
                raw
            ));
        }

        let mut responses: Vec<SimulateFeedsResponse> = serde_json::from_str(&raw)
            .with_context(|| format!(
                "Failed to parse simulate_feeds response for feeds [{}]. URL: {}. Raw response (first 500 chars): {}",
                feed_hashes.join(", "),
                url,
                &raw.chars().take(500).collect::<String>()
            ))?;

        // Compute the median result for each response
        for response in responses.iter_mut() {
            response.result = median(response.results.clone()).expect("Failed to compute median");
        }
        Ok(responses)
    }

    /// POST /v2/simulate/proto
    /// Simulate an OracleFeed from a protobuf object or feed hash
    /// # Arguments
    /// * `feed_or_hash` - The OracleFeed protobuf object (base64 string) or feed hash to simulate
    /// * `include_receipts` - Whether to include receipts in the response
    /// * `network` - Network to use for simulation
    /// # Returns
    /// * `Result<CrossbarSimulateProtoResponse, AnyhowError>` - The simulation results
    pub async fn simulate_proto(
        &self,
        feed_or_hash: &str, // Can be feedHash or base64 encoded proto
        include_receipts: bool,
        network: Option<&str>,
    ) -> Result<CrossbarSimulateProtoResponse, AnyhowError> {
        let mut oracle_feed_b64 = feed_or_hash.to_string();
        
        // Simple heuristic: if it looks like a hash (hex, 64 chars or 66 with 0x), fetch it.
        // Otherwise assume it's base64 encoded proto.
        let is_hash = feed_or_hash.starts_with("0x") || feed_or_hash.len() == 64; 
        
        if is_hash {
             // println!("DEBUG: Fetching feed for hash: {}", feed_or_hash);
             let fetch_resp = self.fetch_oracle_feed(feed_or_hash).await?;
             // println!("DEBUG: Fetched data: {}", fetch_resp.data);
             
             // DECODE AND RE-ENCODE logic
             let delimited_bytes = BASE64_STANDARD.decode(&fetch_resp.data).context("Failed to decode base64")?;
             let feed = OracleFeed::decode_length_delimited(delimited_bytes.as_slice()).context("Failed to decode length delimited proto")?;
             let standard_bytes = feed.encode_to_vec();
             oracle_feed_b64 = BASE64_STANDARD.encode(standard_bytes);
        }

        let url = format!("{}/v2/simulate/proto", self.crossbar_url);
        let payload = serde_json::json!({
            "oracleFeed": oracle_feed_b64,
            "includeReceipts": include_receipts,
            "network": network.unwrap_or("mainnet"),
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .header("Content-Type", "application/json")
            .send()
            .await
            .context("Failed to send v2 simulate request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
             if self.verbose {
                eprintln!(
                    "{}: {}",
                    status,
                    raw
                );
            }
            return Err(anyhow!(
                "Bad status code {} for simulate_proto. Response: {}",
                status.as_u16(),
                raw
            ));
        }
        
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse simulate_proto response. URL: {}. Raw response (first 500 chars): {}",
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

    /// Fetch the Sui feed update from the crossbar gateway.
    ///
    /// # Arguments
    /// * `network` - The Sui network identifier (e.g., "mainnet", "testnet")
    /// * `aggregator_addresses` - A slice of aggregator address strings.
    ///
    /// # Returns
    /// * `Result<FetchSuiUpdatesResponse, AnyhowError>` - The response containing Sui feed update data.
    pub async fn fetch_sui_updates(
        &self,
        network: &str,
        aggregator_addresses: &[&str],
    ) -> Result<FetchSuiUpdatesResponse, AnyhowError> {
        if aggregator_addresses.is_empty() {
            return Err(anyhow!("Aggregator addresses are empty"));
        }
        let feeds_param = aggregator_addresses.join(",");
        let url = format!(
            "{}/updates/sui/{}/{}",
            self.crossbar_url, network, feeds_param
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send fetch Sui updates request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for Sui feeds on network '{}'. Response: {}",
                status.as_u16(),
                network,
                raw
            ));
        }

        let mut update_response: FetchSuiUpdatesResponse = serde_json::from_str(&raw)
            .with_context(|| format!(
                "Failed to parse fetch_sui_updates response for feeds on network '{}'. URL: {}. Raw response (first 500 chars): {}",
                network,
                url,
                &raw.chars().take(500).collect::<String>()
            ))?;

        // If the server did not include aggregator_id or it is empty,
        // and if the number of responses matches the number of aggregator_addresses,
        // we assign the aggregator addresses to the corresponding responses.
        if update_response.responses.len() == aggregator_addresses.len() {
            for (resp_item, &agg_id) in update_response
                .responses
                .iter_mut()
                .zip(aggregator_addresses)
            {
                if resp_item.aggregator_id.is_none()
                    || resp_item.aggregator_id.as_ref().unwrap().is_empty()
                {
                    resp_item.aggregator_id = Some(agg_id.to_string());
                }
            }
        }
        Ok(update_response)
    }

    /// Simulate feed responses for Sui from the crossbar gateway.
    ///
    /// # Arguments
    /// * `network` - The Sui network identifier (e.g. "mainnet", "testnet")
    /// * `feed_ids` - The list of feed ids as string slices.
    ///
    /// # Returns
    /// * `Result<Vec<SimulateSuiFeedsResponse>, AnyhowError>` - The current simulated results for the requested feeds.
    pub async fn simulate_sui_feeds(
        &self,
        network: &str,
        feed_ids: &[&str],
    ) -> Result<Vec<SimulateSuiFeedsResponse>, AnyhowError> {
        if feed_ids.is_empty() {
            return Err(anyhow!("Feed ids are empty"));
        }
        let feeds_param = feed_ids.join(",");
        let url = format!(
            "{}/simulate/sui/{}/{}",
            self.crossbar_url, network, feeds_param
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send simulate sui feeds request")?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .context("Failed to fetch response for simulate sui feeds")?;
        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!("Bad status code {}", status.as_u16()));
        }
        // Parse the response. We assume the TS server returns JSON matching SimulateSuiFeedsResponse.
        let responses: Vec<SimulateSuiFeedsResponse> =
            serde_json::from_str(&raw).context("Failed to parse simulate sui feeds response")?;
        Ok(responses)
    }

    /// Stream the simulation of feed responses from the crossbar gateway.
    pub fn stream_simulate_feeds<'a>(
        &'a self,
        feed_hashes: Vec<&'a str>,
        poll_interval: Duration,
        include_receipts: bool,
    ) -> impl Stream<Item = Result<Vec<SimulateFeedsResponse>, AnyhowError>> + 'a {
        // Create an interval timer stream.
        let interval_stream = IntervalStream::new(interval(poll_interval));
        let feed_hashes = feed_hashes.clone();
        // For each tick, call the simulate_feeds function.
        interval_stream.then(move |_| {
            let feed_hashes = feed_hashes.clone();
            async move { self.simulate_feeds(&feed_hashes, include_receipts).await }
        })
    }

    /// Stream the simulation of feed responses from the crossbar gateway for Solana feeds.
    pub fn stream_simulate_solana_feeds<'a>(
        &'a self,
        network: ClusterType,
        feed_pubkeys: &'a [Pubkey],
        poll_interval: Duration,
        include_receipts: bool,
    ) -> impl Stream<Item = Result<Vec<SimulateSolanaFeedsResponse>, AnyhowError>> + 'a {
        let interval_stream = IntervalStream::new(interval(poll_interval));
        interval_stream.then(move |_| {
            let network = network;
            async move { self.simulate_solana_feeds(network, feed_pubkeys, include_receipts).await }
        })
    }

    /// Stream the simulation of Sui feed responses from the crossbar gateway.
    pub fn stream_simulate_sui_feeds<'a>(
        &'a self,
        network: &'a str,
        feed_ids: Vec<&'a str>,
        poll_interval: Duration,
    ) -> impl Stream<Item = Result<Vec<SimulateSuiFeedsResponse>, AnyhowError>> + 'a {
        let interval_stream = IntervalStream::new(interval(poll_interval));
        interval_stream.then(move |_| {
            let feed_ids = feed_ids.clone();
            async move { self.simulate_sui_feeds(network, &feed_ids).await }
        })
    }

    /// Stream the Sui feed update responses from the crossbar gateway.
    ///
    /// # Arguments
    /// * `network` - The Sui network identifier (e.g., "mainnet", "testnet")
    /// * `aggregator_addresses` - A vector of aggregator address strings.
    /// * `poll_interval` - The polling interval for updates.
    ///
    /// # Returns
    /// * `impl Stream<Item = Result<FetchSuiUpdatesResponse, AnyhowError>>`
    ///    - A stream of Sui update responses.
    pub fn stream_sui_updates<'a>(
        &'a self,
        network: &'a str,
        aggregator_addresses: Vec<&'a str>,
        poll_interval: Duration,
    ) -> impl Stream<Item = Result<FetchSuiUpdatesResponse, AnyhowError>> + 'a {
        let interval_stream = IntervalStream::new(interval(poll_interval));
        interval_stream.then(move |_| {
            let aggregator_addresses = aggregator_addresses.clone();
            async move { self.fetch_sui_updates(network, &aggregator_addresses).await }
        })
    }

    /// Fetches gateway URLs from the crossbar service for a specific network
    ///
    /// # Arguments
    /// * `network` - The network to fetch gateways for ("mainnet" or "devnet")
    ///
    /// # Returns
    /// * `Result<Vec<String>, AnyhowError>` - A vector of gateway URLs
    pub async fn fetch_gateways(&self, network: &str) -> Result<Vec<String>, AnyhowError> {
        let url = format!("{}/gateways?network={}", self.crossbar_url, network);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send fetch gateways request")?;

        let status = resp.status();
        let raw = resp.text().await.context("Failed to fetch response text")?;

        if !status.is_success() {
            if self.verbose {
                eprintln!("{}: {}", status, raw);
            }
            return Err(anyhow!(
                "Bad status code {} for fetch gateways on network '{}'. Response: {}",
                status.as_u16(),
                network,
                raw
            ));
        }

        serde_json::from_str(&raw).with_context(|| {
            format!(
                "Failed to parse fetch_gateways response for network '{}'. URL: {}. Raw response (first 500 chars): {}",
                network,
                url,
                &raw.chars().take(500).collect::<String>()
            )
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_crossbar_client_default_initialization() {
        let key = Pubkey::from_str("D1MmZ3je8GCjLrTbWXotnZ797k6E56QkdyXyhPXZQocH").unwrap();
        let client = CrossbarClient::default();
        let resp = client
            .simulate_solana_feeds(ClusterType::MainnetBeta, &[key], false)
            .await
            .unwrap();
        println!("{:?}", resp);
    }
}
