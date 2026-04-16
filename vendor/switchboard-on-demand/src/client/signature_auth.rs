use switchboard_utils::SbError;
use sha2::{Digest, Sha256};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use crate::solana_sdk::signature::{Keypair, Signer};

/// Configuration for signature-based authentication
#[derive(Clone)]
pub struct SignatureAuthConfig {
    /// Keypair for signing
    pub keypair: Arc<Keypair>,
    /// Refresh interval in seconds (default: 5 minutes for signature freshness)
    pub refresh_interval: Option<Duration>,
}

/// Signed authentication data
#[derive(Clone, Debug)]
pub struct SignedAuthData {
    /// Base58-encoded signature
    pub signature: String,
    /// Base58-encoded public key
    pub public_key: String,
    /// Blockhash used for signing
    pub blockhash: String,
    /// Timestamp when signature was created
    pub timestamp: u64,
}

/// Manages signature-based authentication for Switchboard service
/// Signs blockhash periodically to prove subscription and freshness
pub struct SignatureAuth {
    config: SignatureAuthConfig,
    current_auth_data: Arc<RwLock<Option<SignedAuthData>>>,
    last_refresh_time: Arc<RwLock<SystemTime>>,
}

impl SignatureAuth {
    /// Create a new SignatureAuth instance
    pub fn new(config: SignatureAuthConfig) -> Self {
        Self {
            config,
            current_auth_data: Arc::new(RwLock::new(None)),
            last_refresh_time: Arc::new(RwLock::new(SystemTime::UNIX_EPOCH)),
        }
    }

    /// Check if signature auth is configured and enabled
    pub fn is_enabled(&self) -> bool {
        true
    }

    /// Get the current auth headers, refreshing if necessary
    pub async fn get_auth_headers(
        &self,
        current_blockhash: &str,
    ) -> Result<std::collections::HashMap<String, String>, SbError> {
        if !self.is_enabled() {
            return Ok(std::collections::HashMap::new());
        }

        // Check if we need to refresh
        let refresh_interval = self
            .config
            .refresh_interval
            .unwrap_or(Duration::from_secs(5 * 60));
        let now = SystemTime::now();
        let last_refresh = *self.last_refresh_time.read().unwrap();

        if self.current_auth_data.read().unwrap().is_none()
            || now.duration_since(last_refresh).unwrap_or(Duration::MAX) >= refresh_interval
        {
            self.refresh(current_blockhash).await?;
        }

        // Get auth data
        let auth_data = self
            .current_auth_data
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| SbError::CustomError {
                message: "No auth data available".to_string(),
                source: std::sync::Arc::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No auth data",
                )),
            })?;

        // Return auth headers
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "X-Switchboard-Signature".to_string(),
            auth_data.signature,
        );
        headers.insert("X-Switchboard-Pubkey".to_string(), auth_data.public_key);
        headers.insert(
            "X-Switchboard-Blockhash".to_string(),
            auth_data.blockhash,
        );
        headers.insert(
            "X-Switchboard-Timestamp".to_string(),
            auth_data.timestamp.to_string(),
        );

        Ok(headers)
    }

    /// Get the current auth data as an object (for WebSocket messages)
    pub async fn get_auth_data(&self) -> Result<SignedAuthData, SbError> {
        let auth_data = self.current_auth_data.read().unwrap().clone();
        auth_data.ok_or_else(|| SbError::CustomError {
            message: "No auth data available".to_string(),
            source: std::sync::Arc::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No auth data",
            )),
        })
    }

    /// Refresh the signature by fetching latest blockhash and signing
    pub async fn refresh(&self, blockhash: &str) -> Result<(), SbError> {
        if !self.is_enabled() {
            return Ok(());
        }

        // Create message to sign
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let message = Self::create_message(blockhash, timestamp);

        // Sign the message with Ed25519
        let signature = self.config.keypair.sign_message(&message);

        // Store the auth data (use base58 encoding - Solana standard)
        let auth_data = SignedAuthData {
            signature: bs58::encode(signature.as_ref()).into_string(),
            public_key: self.config.keypair.pubkey().to_string(),
            blockhash: blockhash.to_string(),
            timestamp,
        };

        *self.current_auth_data.write().unwrap() = Some(auth_data);
        *self.last_refresh_time.write().unwrap() = SystemTime::now();

        Ok(())
    }

    /// Create the message to sign
    /// Format: SHA256(blockhash:timestamp)
    fn create_message(blockhash: &str, timestamp: u64) -> Vec<u8> {
        // Message format: blockhash:timestamp
        let message_str = format!("{}:{}", blockhash, timestamp);

        // Hash the message with SHA256 (matches TypeScript implementation)
        let mut hasher = Sha256::new();
        hasher.update(message_str.as_bytes());
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_sdk::signature::Keypair;

    #[tokio::test]
    async fn test_signature_auth_creation() {
        let keypair = Keypair::new();
        let config = SignatureAuthConfig {
            keypair: Arc::new(keypair),
            refresh_interval: Some(Duration::from_secs(300)),
        };

        let auth = SignatureAuth::new(config);
        assert!(auth.is_enabled());
    }

    #[tokio::test]
    async fn test_signature_refresh() {
        let keypair = Keypair::new();
        let config = SignatureAuthConfig {
            keypair: Arc::new(keypair),
            refresh_interval: Some(Duration::from_secs(300)),
        };

        let auth = SignatureAuth::new(config);
        let blockhash = "11111111111111111111111111111111";

        // Initial refresh
        auth.refresh(blockhash).await.unwrap();

        // Get auth data
        let auth_data = auth.get_auth_data().await.unwrap();
        assert!(!auth_data.signature.is_empty());
        assert!(!auth_data.public_key.is_empty());
        assert_eq!(auth_data.blockhash, blockhash);
    }

    #[tokio::test]
    async fn test_auth_headers() {
        let keypair = Keypair::new();
        let config = SignatureAuthConfig {
            keypair: Arc::new(keypair),
            refresh_interval: Some(Duration::from_secs(300)),
        };

        let auth = SignatureAuth::new(config);
        let blockhash = "11111111111111111111111111111111";

        let headers = auth.get_auth_headers(blockhash).await.unwrap();
        assert!(headers.contains_key("X-Switchboard-Signature"));
        assert!(headers.contains_key("X-Switchboard-Pubkey"));
        assert!(headers.contains_key("X-Switchboard-Blockhash"));
        assert!(headers.contains_key("X-Switchboard-Timestamp"));
    }
}
