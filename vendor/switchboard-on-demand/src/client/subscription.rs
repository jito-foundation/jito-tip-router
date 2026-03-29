use switchboard_utils::SbError;
use borsh::{BorshDeserialize, BorshSerialize};
use crate::Pubkey;
use crate::cfg_client;
use std::str::FromStr;

/// Switchboard Subscriber Program ID
pub const SUBSCRIPTION_PROGRAM_ID: &str = "SbsCRB7Y3SGNEwVgvwE4R8ZYuhertgc7ekUXQbD6pNH";

/// SWTCH Token Mint Address
pub const SWTCH_MINT: &str = "SW1TCHLmRGTfW5xZknqQdpdarB8PD95sJYWpNp9TbFx";

/// SWTCH/USDT Feed ID for oracle pricing
pub const SWTCH_FEED_ID: &str =
    "148aaedb33ff23593805377f131af7fe01b4770ae2b9e4a2f7f8b3a180314711";

/// Subscription account information
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SubscriptionInfo {
    pub owner: Pubkey,
    pub tier_id: u16,
    pub tier_id_in_transition: u16,
    pub subscription_start_epoch: i64,
    pub subscription_end_epoch: i64,
    pub last_tier_change_epoch: i64,
    pub is_active: u8,
    pub authorized_users: Vec<Pubkey>,
    pub authorized_user_count: u8,
    pub total_tokens_paid: u64,
    pub last_payment_amount: u64,
    pub last_payment_token: Pubkey,
    pub contact_name: Vec<u8>,
    pub contact_email: Vec<u8>,
}

impl SubscriptionInfo {
    /// Deserialize from account data
    pub fn deserialize(data: &[u8]) -> Result<Self, SbError> {
        BorshDeserialize::try_from_slice(data).map_err(|e| SbError::CustomError {
            message: format!("Failed to deserialize subscription info: {}", e),
            source: std::sync::Arc::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e,
            )),
        })
    }

    /// Check if subscription is currently active
    pub fn is_subscription_active(&self) -> bool {
        self.is_active == 1
    }

    /// Check if subscription has expired
    pub fn is_expired(&self, current_epoch: i64) -> bool {
        current_epoch > self.subscription_end_epoch
    }

    /// Get remaining epochs
    pub fn remaining_epochs(&self, current_epoch: i64) -> i64 {
        (self.subscription_end_epoch - current_epoch).max(0)
    }

    /// Get contact name as string
    pub fn contact_name_str(&self) -> Option<String> {
        if self.contact_name.is_empty() {
            None
        } else {
            String::from_utf8(self.contact_name.clone()).ok()
        }
    }

    /// Get contact email as string
    pub fn contact_email_str(&self) -> Option<String> {
        if self.contact_email.is_empty() {
            None
        } else {
            String::from_utf8(self.contact_email.clone()).ok()
        }
    }

    /// Check if a user is authorized
    pub fn is_user_authorized(&self, user: &Pubkey) -> bool {
        self.authorized_users.contains(user)
    }
}

/// Subscription tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum SubscriptionTier {
    Free = 1,
    Pro = 2,
    Surge = 3,
    Enterprise = 10,
}

impl SubscriptionTier {
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            1 => Some(Self::Free),
            2 => Some(Self::Pro),
            3 => Some(Self::Surge),
            10 => Some(Self::Enterprise),
            _ => None,
        }
    }

    pub fn as_id(&self) -> u16 {
        *self as u16
    }
}

/// PDA derivation helpers
pub mod pda {
    use super::*;

    /// Derive the state PDA
    pub fn get_state_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"STATE"], program_id)
    }

    /// Derive the tier PDA
    pub fn get_tier_pda(tier_id: u16, program_id: &Pubkey) -> (Pubkey, u8) {
        let tier_id_bytes = tier_id.to_le_bytes();
        Pubkey::find_program_address(&[b"TIER", &tier_id_bytes], program_id)
    }

    /// Derive the subscription PDA
    pub fn get_subscription_pda(owner: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"SUBSCRIPTION", owner.as_ref()], program_id)
    }

    /// Derive the token vault PDA
    pub fn get_token_vault_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"TOKEN_VAULT", mint.as_ref()], program_id)
    }
}

/// Get the subscription program ID
pub fn get_program_id() -> Pubkey {
    Pubkey::from_str(SUBSCRIPTION_PROGRAM_ID).unwrap()
}

/// Get the SWTCH mint address
pub fn get_swtch_mint() -> Pubkey {
    Pubkey::from_str(SWTCH_MINT).unwrap()
}

cfg_client! {
    pub mod client_utils {
        use super::*;
        use crate::client::Queue;
        use std::sync::Arc;

        #[cfg(feature = "solana-v2")]
        use crate::solana_compat::solana_client::nonblocking::rpc_client::RpcClient;
        #[cfg(feature = "solana-v2")]
        use crate::solana_sdk::signature::Keypair;
        use crate::Instruction;

    /// Parameters for creating a new subscription
    pub struct CreateSubscriptionParams {
        /// Solana RPC client
        pub client: Arc<RpcClient>,
        /// Payer and subscription owner
        pub payer: Arc<Keypair>,
        /// Tier ID (1=Free, 2=Pro, 3=Surge, 10+=Enterprise)
        pub tier_id: u16,
        /// Number of epochs to pay for (minimum 2)
        pub epoch_amount: u64,
        /// Optional contact name
        pub contact_name: Option<String>,
        /// Optional contact email
        pub contact_email: Option<String>,
        /// Switchboard queue for oracle updates
        pub queue: Queue,
    }

    /// Parameters for upgrading a subscription
    pub struct UpgradeSubscriptionParams {
        /// Solana RPC client
        pub client: Arc<RpcClient>,
        /// Subscription owner
        pub owner: Arc<Keypair>,
        /// New tier ID (must be more expensive than current)
        pub new_tier_id: u16,
        /// Number of epochs requested (credit applied, minimum 1)
        pub epoch_amount: u64,
        /// Switchboard queue for oracle updates
        pub queue: Queue,
    }

    /// Parameters for downgrading a subscription
    pub struct DowngradeSubscriptionParams {
        /// Solana RPC client
        pub client: Arc<RpcClient>,
        /// Subscription owner
        pub owner: Arc<Keypair>,
        /// New tier ID (must be less expensive than current)
        pub new_tier_id: u16,
    }

    /// Parameters for extending a subscription
    pub struct ExtendSubscriptionParams {
        /// Solana RPC client
        pub client: Arc<RpcClient>,
        /// Subscription owner (or admin if admin_extend=true)
        pub owner: Arc<Keypair>,
        /// Number of epochs to add
        pub epoch_amount: u64,
        /// If true, no payment required (admin only)
        pub admin_extend: bool,
        /// Switchboard queue for oracle updates (required if admin_extend=false)
        pub queue: Option<Queue>,
    }

    /// Parameters for managing team members
    pub struct ManageTeamMemberParams {
        /// Solana RPC client
        pub client: Arc<RpcClient>,
        /// Subscription owner
        pub owner: Arc<Keypair>,
        /// User to add/remove
        pub user: Pubkey,
    }

    /// Subscription manager for handling all subscription operations
    pub struct SubscriptionManager {
        program_id: Pubkey,
    }

    impl SubscriptionManager {
        /// Create a new subscription manager
        pub fn new() -> Self {
            Self {
                program_id: get_program_id(),
            }
        }

        /// Fetch subscription info from chain
        pub async fn fetch_subscription(
            &self,
            client: &RpcClient,
            owner: &Pubkey,
        ) -> Result<SubscriptionInfo, SbError> {
            let (subscription_pda, _) = pda::get_subscription_pda(owner, &self.program_id);

            let account_data = client
                .get_account_data(&subscription_pda)
                .await
                .map_err(|e| SbError::CustomError {
                    message: format!("Failed to fetch subscription account: {}", e),
                    source: Arc::new(std::io::Error::new(std::io::ErrorKind::Other, e)),
                })?;

            SubscriptionInfo::deserialize(&account_data)
        }

        /// Add a team member to the subscription
        pub fn add_team_member_ix(
            &self,
            owner: &Pubkey,
            user: &Pubkey,
        ) -> Result<Instruction, SbError> {
            let (subscription_pda, _) = pda::get_subscription_pda(owner, &self.program_id);

            // Build instruction data: discriminator (8) + user (32)
            let mut data = Vec::with_capacity(40);
            // Discriminator for add_team_member
            data.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]);
            data.extend_from_slice(user.as_ref());

            Ok(Instruction {
                program_id: self.program_id,
                accounts: vec![
                    crate::AccountMeta::new(subscription_pda, false),
                    crate::AccountMeta::new_readonly(*owner, true),
                ],
                data,
            })
        }

        /// Remove a team member from the subscription
        pub fn remove_team_member_ix(
            &self,
            owner: &Pubkey,
            user: &Pubkey,
        ) -> Result<Instruction, SbError> {
            let (subscription_pda, _) = pda::get_subscription_pda(owner, &self.program_id);

            // Build instruction data: discriminator (8) + user (32)
            let mut data = Vec::with_capacity(40);
            // Discriminator for remove_team_member
            data.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0]);
            data.extend_from_slice(user.as_ref());

            Ok(Instruction {
                program_id: self.program_id,
                accounts: vec![
                    crate::AccountMeta::new(subscription_pda, false),
                    crate::AccountMeta::new_readonly(*owner, true),
                ],
                data,
            })
        }

        /// Check if a user is a team member
        pub async fn is_team_member(
            &self,
            client: &RpcClient,
            owner: &Pubkey,
            user: &Pubkey,
        ) -> Result<bool, SbError> {
            let subscription_info = self.fetch_subscription(client, owner).await?;
            Ok(subscription_info.is_user_authorized(user))
        }

        /// Get all team members
        pub async fn get_team_members(
            &self,
            client: &RpcClient,
            owner: &Pubkey,
        ) -> Result<Vec<Pubkey>, SbError> {
            let subscription_info = self.fetch_subscription(client, owner).await?;
            Ok(subscription_info.authorized_users)
        }
    }

    impl Default for SubscriptionManager {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Helper to get the quote account for SWTCH/USDT feed
    pub fn get_quote_account(_queue_pubkey: &Pubkey) -> Pubkey {
        // This would use OracleQuote::getCanonicalPubkey
        // For now, return a placeholder
        // TODO: Implement proper quote account derivation
        Pubkey::default()
    }
} // close cfg_client!
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_tier_conversion() {
        assert_eq!(SubscriptionTier::Free.as_id(), 1);
        assert_eq!(SubscriptionTier::Pro.as_id(), 2);
        assert_eq!(SubscriptionTier::Surge.as_id(), 3);
        assert_eq!(SubscriptionTier::Enterprise.as_id(), 10);

        assert_eq!(SubscriptionTier::from_id(1), Some(SubscriptionTier::Free));
        assert_eq!(SubscriptionTier::from_id(2), Some(SubscriptionTier::Pro));
        assert_eq!(
            SubscriptionTier::from_id(3),
            Some(SubscriptionTier::Surge)
        );
        assert_eq!(
            SubscriptionTier::from_id(10),
            Some(SubscriptionTier::Enterprise)
        );
        assert_eq!(SubscriptionTier::from_id(999), None);
    }

    #[test]
    fn test_pda_derivation() {
        let program_id = get_program_id();
        let owner = Pubkey::new_unique();
        let mint = get_swtch_mint();

        // Test state PDA derivation
        let (state_pda, _) = pda::get_state_pda(&program_id);
        assert!(state_pda != Pubkey::default());

        // Test tier PDA derivation
        let (tier_pda, _) = pda::get_tier_pda(2, &program_id);
        assert!(tier_pda != Pubkey::default());

        // Test subscription PDA derivation
        let (sub_pda, _) = pda::get_subscription_pda(&owner, &program_id);
        assert!(sub_pda != Pubkey::default());

        // Test token vault PDA derivation
        let (vault_pda, _) = pda::get_token_vault_pda(&mint, &program_id);
        assert!(vault_pda != Pubkey::default());
    }

    #[test]
    fn test_program_id() {
        let program_id = get_program_id();
        assert_eq!(
            program_id.to_string(),
            "SbsCRB7Y3SGNEwVgvwE4R8ZYuhertgc7ekUXQbD6pNH"
        );
    }

    #[test]
    fn test_swtch_mint() {
        let mint = get_swtch_mint();
        assert_eq!(
            mint.to_string(),
            "SW1TCHLmRGTfW5xZknqQdpdarB8PD95sJYWpNp9TbFx"
        );
    }

    #[test]
    fn test_subscription_info_helpers() {
        let sub_info = SubscriptionInfo {
            owner: Pubkey::new_unique(),
            tier_id: 2,
            tier_id_in_transition: 2,
            subscription_start_epoch: 100,
            subscription_end_epoch: 200,
            last_tier_change_epoch: 100,
            is_active: 1,
            authorized_users: vec![Pubkey::new_unique(), Pubkey::new_unique()],
            authorized_user_count: 2,
            total_tokens_paid: 1000,
            last_payment_amount: 500,
            last_payment_token: get_swtch_mint(),
            contact_name: b"Alice".to_vec(),
            contact_email: b"alice@example.com".to_vec(),
        };

        assert!(sub_info.is_subscription_active());
        assert!(!sub_info.is_expired(150));
        assert!(sub_info.is_expired(201));
        assert_eq!(sub_info.remaining_epochs(150), 50);
        assert_eq!(sub_info.contact_name_str(), Some("Alice".to_string()));
        assert_eq!(
            sub_info.contact_email_str(),
            Some("alice@example.com".to_string())
        );
    }
}
