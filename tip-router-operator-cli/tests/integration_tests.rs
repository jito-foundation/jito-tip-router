use std::{
    fs,
    path::{Path, PathBuf},
};

use anchor_lang::prelude::AnchorSerialize;
use jito_tip_distribution_sdk::jito_tip_distribution::ID as TIP_DISTRIBUTION_ID;
use jito_tip_payment_sdk::jito_tip_payment::ID as TIP_PAYMENT_ID;
use jito_tip_router_program::ID as TIP_ROUTER_ID;
use meta_merkle_tree::generated_merkle_tree::{
    Delegation, GeneratedMerkleTreeCollection, MerkleRootGeneratorError, StakeMeta,
    StakeMetaCollection, TipDistributionMeta,
};
use solana_program::stake::state::StakeStateV2;
use solana_program_test::*;
use solana_sdk::{
    account::AccountSharedData,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use tempfile::TempDir;
use tip_router_operator_cli::{get_meta_merkle_root, TipAccountConfig};

#[allow(dead_code)]
struct TestContext {
    pub context: ProgramTestContext,
    pub tip_distribution_program_id: Pubkey,
    pub tip_payment_program_id: Pubkey,
    pub payer: Keypair,
    pub stake_accounts: Vec<Keypair>,
    pub vote_account: Keypair,
    pub temp_dir: TempDir,
    pub output_dir: PathBuf,
}

impl TestContext {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)?;

        let program_test = ProgramTest::default();

        let mut context = program_test.start_with_context().await;

        let payer = Keypair::from_bytes(&[
            150, 240, 104, 157, 252, 242, 234, 79, 21, 27, 145, 68, 254, 17, 186, 35, 13, 209, 129,
            229, 55, 39, 221, 2, 10, 15, 172, 77, 153, 153, 104, 177, 139, 35, 180, 131, 48, 220,
            136, 28, 111, 206, 79, 164, 184, 15, 55, 187, 195, 222, 117, 207, 143, 84, 114, 234,
            214, 170, 73, 166, 23, 140, 14, 138,
        ])
        .unwrap();

        let vote_account = Keypair::from_bytes(&[
            82, 63, 68, 226, 112, 24, 184, 190, 189, 221, 199, 191, 113, 6, 183, 211, 49, 118, 207,
            131, 38, 112, 192, 34, 209, 45, 157, 156, 33, 180, 25, 211, 171, 205, 243, 31, 145,
            173, 120, 114, 64, 56, 53, 106, 167, 105, 39, 7, 29, 221, 214, 110, 30, 189, 102, 134,
            182, 90, 143, 73, 233, 179, 44, 215,
        ])
        .unwrap();

        // Fund payer account
        let tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &context.payer.pubkey(),
                &payer.pubkey(),
                10_000_000_000, // Increased balance for multiple accounts
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await?;

        // Create multiple stake accounts
        let stake_accounts = vec![
            Keypair::from_bytes(&[
                36, 145, 249, 6, 56, 206, 144, 159, 252, 235, 120, 107, 227, 51, 95, 155, 16, 93,
                244, 249, 80, 188, 177, 237, 116, 119, 71, 26, 61, 226, 174, 9, 73, 94, 136, 174,
                207, 186, 99, 252, 235, 4, 227, 102, 95, 202, 6, 191, 229, 155, 236, 132, 35, 200,
                218, 165, 164, 223, 77, 9, 74, 55, 87, 167,
            ])
            .unwrap(),
            Keypair::from_bytes(&[
                171, 218, 192, 44, 77, 53, 91, 116, 35, 211, 6, 39, 143, 37, 139, 113, 125, 95, 21,
                51, 238, 233, 23, 186, 6, 224, 117, 203, 24, 130, 12, 102, 184, 8, 146, 226, 205,
                37, 237, 60, 24, 44, 119, 124, 26, 16, 34, 91, 30, 156, 166, 43, 70, 30, 42, 226,
                84, 246, 174, 88, 117, 46, 140, 65,
            ])
            .unwrap(),
            Keypair::from_bytes(&[
                69, 215, 21, 39, 99, 64, 106, 141, 233, 163, 199, 154, 22, 184, 130, 157, 255, 77,
                25, 80, 243, 130, 18, 90, 221, 96, 45, 14, 189, 207, 193, 123, 189, 104, 24, 197,
                242, 185, 90, 22, 166, 44, 253, 177, 199, 207, 211, 235, 146, 157, 84, 203, 205,
                56, 142, 65, 79, 75, 247, 114, 151, 204, 190, 147,
            ])
            .unwrap(),
        ];

        // Get rent and space requirements
        let rent = context.banks_client.get_rent().await?;
        let stake_space = std::mem::size_of::<StakeStateV2>();
        let stake_rent = rent.minimum_balance(stake_space);

        // Initialize each stake account
        for stake_account in stake_accounts.iter() {
            let tx = Transaction::new_signed_with_payer(
                &[
                    system_instruction::create_account(
                        &payer.pubkey(),
                        &stake_account.pubkey(),
                        stake_rent,
                        stake_space as u64,
                        &solana_program::stake::program::id(),
                    ),
                    solana_program::stake::instruction::initialize(
                        &stake_account.pubkey(),
                        &(solana_sdk::stake::state::Authorized {
                            staker: payer.pubkey(),
                            withdrawer: payer.pubkey(),
                        }),
                        &solana_sdk::stake::state::Lockup::default(),
                    ),
                ],
                Some(&payer.pubkey()),
                &[&payer, stake_account],
                context.last_blockhash,
            );
            context.banks_client.process_transaction(tx).await?;

            // Update blockhash between transactions
            context.last_blockhash = context.banks_client.get_latest_blockhash().await?;
        }

        // Create and initialize vote account (if needed)
        // Add vote account initialization here if required

        Ok(Self {
            context,
            tip_distribution_program_id: TIP_DISTRIBUTION_ID,
            tip_payment_program_id: TIP_PAYMENT_ID,
            payer,
            stake_accounts, // Store all stake accounts instead of just one
            vote_account,
            temp_dir,
            output_dir,
        })
    }

    fn create_test_stake_meta(
        &self,
        total_tips: u64,
        validator_fee_bps: u16,
    ) -> StakeMetaCollection {
        let stake_meta = StakeMeta {
            validator_vote_account: self.vote_account.pubkey(),
            validator_node_pubkey: self.stake_accounts[0].pubkey(),
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                total_tips,
                merkle_root_upload_authority: self.payer.pubkey(),
                tip_distribution_pubkey: self.tip_distribution_program_id,
                validator_fee_bps,
            }),
            delegations: vec![Delegation {
                stake_account_pubkey: self.stake_accounts[0].pubkey(),
                staker_pubkey: self.payer.pubkey(),
                withdrawer_pubkey: self.payer.pubkey(),
                lamports_delegated: 1_000_000,
            }],
            total_delegated: 1_000_000,
            commission: 10,
        };

        StakeMetaCollection {
            epoch: 0,
            stake_metas: vec![stake_meta],
            bank_hash: "test_bank_hash".to_string(),
            slot: 0,
            tip_distribution_program_id: self.tip_distribution_program_id,
        }
    }
}

#[tokio::test]
async fn test_meta_merkle_creation_from_ledger() {
    // 1. Setup - create necessary variables/arguments
    let ledger_path = Path::new("tests/fixtures/test-ledger");
    let account_paths = vec![ledger_path.join("accounts/run")];
    let full_snapshots_path = PathBuf::from("tests/fixtures/test-ledger");
    let desired_slot = &144;
    let tip_distribution_program_id = &TIP_DISTRIBUTION_ID;
    let out_path = "tests/fixtures/output.json";
    let tip_payment_program_id = &TIP_PAYMENT_ID;
    let ncn_address = Pubkey::new_unique();
    let operator_address = Pubkey::new_unique();
    let epoch = 0u64;
    const PROTOCOL_FEE_BPS: u64 = 300;

    // 2. Call the function
    let meta_merkle_tree = get_meta_merkle_root(
        ledger_path,
        account_paths,
        full_snapshots_path.clone(),
        full_snapshots_path,
        desired_slot,
        tip_distribution_program_id,
        out_path,
        tip_payment_program_id,
        &jito_tip_router_program::id(),
        &ncn_address,
        &operator_address,
        epoch,
        PROTOCOL_FEE_BPS,
        false,
        &ledger_path.to_path_buf(),
    )
    .unwrap();

    // 3. More comprehensive validations
    assert_ne!(
        meta_merkle_tree.merkle_root, [0; 32],
        "Merkle root should not be zero"
    );

    // Verify structure
    assert!(
        meta_merkle_tree.num_nodes > 0,
        "Should have validator nodes"
    );

    // Verify each node
    for node in &meta_merkle_tree.tree_nodes {
        // Verify node has required fields
        assert_ne!(
            node.tip_distribution_account,
            Pubkey::default(),
            "Node should have valid tip distribution account"
        );
        assert!(
            node.max_total_claim > 0,
            "Node should have positive max claim"
        );
        assert!(
            node.max_num_nodes > 0,
            "Node should have positive max nodes"
        );
        assert!(node.proof.is_some(), "Node should have a proof");
    }

    // Verify the proofs are valid
    meta_merkle_tree.verify_proof().unwrap();
}

#[tokio::test]
async fn test_merkle_tree_generation() -> Result<(), Box<dyn std::error::Error>> {
    // Constants
    const PROTOCOL_FEE_BPS: u64 = 300;
    const VALIDATOR_FEE_BPS: u16 = 1000;
    const TOTAL_TIPS: u64 = 1_000_000;
    let ncn_address = Pubkey::new_unique();
    let epoch = 0u64;

    let mut test_context = TestContext::new()
        .await
        .map_err(|_e| MerkleRootGeneratorError::MerkleTreeTestError)?;

    // Get config PDA
    let (config_pda, bump) = Pubkey::find_program_address(&[b"config"], &TIP_DISTRIBUTION_ID);

    // Create config account with protocol fee
    let config = TipAccountConfig {
        authority: test_context.payer.pubkey(),
        protocol_fee_bps: PROTOCOL_FEE_BPS, // 3% protocol fee
        bump,
    };

    // Create config account
    let space = 32 + 2 + 1; // pubkey (32) + u16 (2) + u8 (1)
    let rent = test_context.context.banks_client.get_rent().await?;

    // Create account data
    let mut account =
        AccountSharedData::new(rent.minimum_balance(space), space, &TIP_DISTRIBUTION_ID);

    let mut config_data = vec![0u8; space];
    let _ = config.serialize(&mut config_data);

    // Create account with data
    account.set_data(config_data);

    // Set the account
    test_context.context.set_account(&config_pda, &account);

    let stake_meta_collection = test_context.create_test_stake_meta(TOTAL_TIPS, VALIDATOR_FEE_BPS);

    let protocol_fee_amount =
        (((TOTAL_TIPS as u128) * (PROTOCOL_FEE_BPS as u128)) / 10000u128) as u64;
    let validator_fee_amount =
        (((TOTAL_TIPS as u128) * (VALIDATOR_FEE_BPS as u128)) / 10000u128) as u64;
    let remaining_tips = TOTAL_TIPS - protocol_fee_amount - validator_fee_amount;

    // Then use it in generate_merkle_root
    let merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
        stake_meta_collection.clone(),
        &ncn_address,
        epoch,
        PROTOCOL_FEE_BPS,
        &jito_tip_router_program::id(),
    )?;

    let generated_tree = &merkle_tree_coll.generated_merkle_trees[0];

    assert_eq!(
        generated_tree.merkle_root.to_string(),
        "4X4wPZvbbKQkkJEmdot5J2nQjs2amJUbF1Be6Pb5BV3u"
    );

    let nodes = &generated_tree.tree_nodes;

    // Get the protocol fee recipient PDA - use the same derivation as in the implementation
    let (protocol_fee_recipient, _) = Pubkey::find_program_address(
        &[
            b"base_reward_receiver",
            &ncn_address.to_bytes(),
            &(epoch + 1).to_le_bytes(),
        ],
        &TIP_ROUTER_ID,
    );

    let protocol_fee_node = nodes
        .iter()
        .find(|node| node.claimant == protocol_fee_recipient)
        .expect("Protocol fee node should exist");
    assert_eq!(protocol_fee_node.amount, protocol_fee_amount);

    // Verify validator fee node
    let validator_fee_node = nodes
        .iter()
        .find(|node| node.claimant == stake_meta_collection.stake_metas[0].validator_node_pubkey)
        .expect("Validator fee node should exist");
    assert_eq!(validator_fee_node.amount, validator_fee_amount);

    // Verify delegator nodes
    for delegation in &stake_meta_collection.stake_metas[0].delegations {
        let delegator_share = (((remaining_tips as u128) * (delegation.lamports_delegated as u128))
            / (stake_meta_collection.stake_metas[0].total_delegated as u128))
            as u64;

        let delegator_node = nodes
            .iter()
            .find(|node| node.claimant == delegation.staker_pubkey)
            .expect("Delegator node should exist");
        assert_eq!(
            delegator_node.amount, delegator_share,
            "Delegator share mismatch for stake amount {}",
            delegation.lamports_delegated
        );
    }

    // Verify node structure
    for node in nodes {
        assert_ne!(
            node.claimant,
            Pubkey::default(),
            "Node claimant should not be default"
        );
        assert_ne!(
            node.claim_status_pubkey,
            Pubkey::default(),
            "Node claim status should not be default"
        );
        assert!(node.amount > 0, "Node amount should be greater than 0");
        assert!(node.proof.is_some(), "Node should have a proof");
    }

    Ok(())
}
