/// Example: Real-time price streaming with Surge WebSocket client
///
/// This example demonstrates how to:
/// 1. Fetch gateway from Crossbar
/// 2. Connect to Surge WebSocket server
/// 3. Subscribe to price feeds
/// 4. Receive real-time updates
/// 5. Handle different event types
/// 6. Display latency metrics
///
/// Run with: cargo run --example surge_streaming --features client

use switchboard_on_demand::client::crossbar::CrossbarClient;
use switchboard_on_demand::client::surge::{
    Surge, SurgeEvent, FeedSubscription, ConnectionState,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Switchboard Surge WebSocket Streaming Example");
    println!("=================================================\n");

    // Configuration
    let api_key = std::env::var("SURGE_API_KEY")
        .unwrap_or_else(|_| "sb_live_demo_key".to_string());
    let crossbar_url = std::env::var("CROSSBAR_URL")
        .unwrap_or_else(|_| "https://crossbar.switchboard.xyz".to_string());
    let network = std::env::var("NETWORK")
        .unwrap_or_else(|_| "mainnet".to_string());

    println!("🔑 Using API key: {}...", &api_key[..15.min(api_key.len())]);
    println!("🌐 Crossbar: {}", crossbar_url);
    println!("🔗 Network: {}\n", network);

    // Fetch available gateways from Crossbar
    println!("🔍 Fetching available gateways...");
    let crossbar = CrossbarClient::new(&crossbar_url, true);
    let gateways = crossbar.fetch_gateways(&network).await?;

    if gateways.is_empty() {
        eprintln!("❌ No gateways available for network: {}", network);
        return Ok(());
    }

    println!("✅ Found {} gateway(s)", gateways.len());
    let gateway_url = &gateways[0];
    println!("📡 Using gateway: {}\n", gateway_url);

    // Create Surge client with the fetched gateway
    let surge = Surge::init(api_key, gateway_url.clone(), true);

    // IMPORTANT: Subscribe to events BEFORE connecting so we don't miss any events
    let mut event_rx = surge.subscribe_events();

    // Connect to WebSocket
    println!("🔌 Connecting to Surge WebSocket...");
    surge.connect().await?;

    // Wait for connection
    let mut retries = 0;
    while surge.get_state().await != ConnectionState::Connected && retries < 10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        retries += 1;
    }

    if surge.get_state().await != ConnectionState::Connected {
        eprintln!("❌ Failed to connect to Surge WebSocket");
        return Ok(());
    }

    println!("✅ Connected successfully!\n");

    // Subscribe to feeds
    println!("📊 Subscribing to price feeds...");
    let feeds = vec![
        FeedSubscription::Symbol {
            symbol: "BTC/USD".to_string(),
            source: Some("WEIGHTED".to_string()),
        },
    ];

    surge.subscribe(feeds).await?;
    println!("✅ Subscribed to 1 feed (BTC/USD WEIGHTED)\n");

    // Listen for events
    println!("👂 Listening for updates...\n");
    println!("{:-<80}", "");
    let mut update_count = 0;

    // Listen for 30 seconds or 10 updates
    let start_time = tokio::time::Instant::now();
    let timeout_duration = Duration::from_secs(30);

    while start_time.elapsed() < timeout_duration && update_count < 10 {
        // Use timeout to check elapsed time periodically
        match tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
            Ok(Ok(event)) => {
                eprintln!("[Example] Received event: {:?}", std::mem::discriminant(&event));
                match event {
                    SurgeEvent::Update(update) => {
                        update_count += 1;
                        println!("\n📊 UPDATE #{}", update_count);
                        println!("{:-<80}", "");

                        // Display feed hashes
                        let feeds = update.get_signed_feeds();
                        println!("🔑 Feed Hashes:");
                        for (i, feed_hash) in feeds.iter().enumerate() {
                            println!("   {}. {}", i + 1, feed_hash);
                        }

                        // Display formatted prices
                        println!("\n💰 Prices:");
                        let prices = update.get_formatted_prices();
                        for (feed_hash, price) in prices {
                            println!("   {}: {}", &feed_hash[..16], price);
                        }

                        // Display trigger type
                        if update.is_triggered_by_price_change() {
                            println!("\n📈 Trigger: Price Change");
                        } else {
                            println!("\n⏰ Trigger: Heartbeat");
                        }

                        // Display latency metrics
                        let metrics = update.get_latency_metrics();
                        println!("\n⚡ Latency Metrics:");
                        println!("   Exchange → Oracle: {:?}", metrics.exchange_to_oracle_update);
                        println!("   Oracle → Client: {:?}", metrics.oracle_update_to_client);
                        println!("   End-to-End: {:?}", metrics.end_to_end);

                        // Display oracle response details
                        let raw = update.get_raw_response();
                        if let Some(oracle_resp) = &raw.oracle_response {
                            println!("\n🔐 Oracle Details:");
                            println!("   Slot: {}", oracle_resp.slot);
                            println!("   Oracle Index: {}", oracle_resp.oracle_idx);
                            println!("   Oracle Pubkey: {}...", &oracle_resp.oracle_pubkey[..16]);
                            if let Some(ed25519) = &oracle_resp.ed25519_enclave_signer {
                                println!("   Signature Scheme: Ed25519");
                                println!("   Signer: {}...", &ed25519[..16]);
                            } else {
                                println!("   Signature Scheme: Secp256k1");
                            }
                        }

                        println!("{:-<80}", "");
                    }
                    SurgeEvent::UnsignedUpdate(update) => {
                        println!("\n📈 UNSIGNED UPDATE");
                        println!("{:-<80}", "");
                        println!("Symbols: {:?}", update.get_symbols());
                        println!("Prices: {:?}", update.get_prices());
                        println!("{:-<80}", "");
                    }
                    SurgeEvent::Connected => {
                        println!("✅ Connected");
                    }
                    SurgeEvent::Disconnected => {
                        println!("❌ Disconnected");
                        break;
                    }
                    SurgeEvent::Error(err) => {
                        eprintln!("⚠️  Error: {}", err);
                    }
                    SurgeEvent::Subscribed(msg) => {
                        println!("✅ Subscribed");
                        if let Some(feed_quotes) = msg.feed_quotes {
                            println!("   Feed quotes: {}", feed_quotes.len());
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                // Broadcast channel error (e.g., lagged)
                eprintln!("⚠️  Event channel error: {}", e);
                break;
            }
            Err(_) => {
                // Timeout - continue loop
                continue;
            }
        }
    }

    println!("\n\n📊 Summary:");
    println!("   Total updates received: {}", update_count);
    println!("   Duration: {:?}", start_time.elapsed());

    // Disconnect
    println!("\n🔌 Disconnecting...");
    surge.disconnect().await?;
    println!("✅ Disconnected successfully");

    Ok(())
}
