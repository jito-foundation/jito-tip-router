/*

Goal:
- Get a successful invoke of the set_merkle_root instruction
- Have a clean way to set this up
- Set myself up / have a good understanding of what I need to do to set up the other instructions
- Maybe add some stuff to the TestBuilder so its easier for others?


Working backwards what do I need to invoke this?

- NCN config
- NCN
- Full ballot box
- Vote account
- Tip distribution account
- tip distribution config
- tip distribution program id


1 Create GeneratedMerkleTree (with this vote account)
2 Create MetaMerkleTree (with this vote account)

- Set root of MetaMerkleTree in BallotBox

- get_proof(vote_account) from MetaMerkleTree

-

*/

mod set_merkle_root {
    use jito_tip_distribution::state::ClaimStatus;
    use jito_tip_distribution_sdk::derive_tip_distribution_account_address;
    use jito_tip_router_core::{
        ballot_box::{Ballot, BallotBox},
        ncn_config::NcnConfig,
    };
    use meta_merkle_tree::{
        generated_merkle_tree::{
            self, Delegation, GeneratedMerkleTree, GeneratedMerkleTreeCollection, StakeMeta,
            StakeMetaCollection, TipDistributionMeta,
        },
        meta_merkle_tree::MetaMerkleTree,
    };
    use solana_sdk::pubkey::Pubkey;

    use crate::{
        fixtures::{
            test_builder::TestBuilder, tip_distribution_client::TipDistributionClient,
            tip_router_client::TipRouterClient, TestError, TestResult,
        },
        helpers::ballot_box::serialized_ballot_box_account,
    };

    struct GeneratedMerkleTreeCollectionFixture {
        test_generated_merkle_tree: GeneratedMerkleTree,
        collection: GeneratedMerkleTreeCollection,
    }

    fn create_tree_node(
        claimant_staker_withdrawer: Pubkey,
        amount: u64,
        epoch: u64,
    ) -> generated_merkle_tree::TreeNode {
        let (claim_status_pubkey, claim_status_bump) = Pubkey::find_program_address(
            &[
                ClaimStatus::SEED,
                claimant_staker_withdrawer.to_bytes().as_ref(),
                epoch.to_le_bytes().as_ref(),
            ],
            &jito_tip_distribution::id(),
        );

        generated_merkle_tree::TreeNode {
            claimant: claimant_staker_withdrawer,
            claim_status_pubkey,
            claim_status_bump,
            staker_pubkey: claimant_staker_withdrawer,
            withdrawer_pubkey: claimant_staker_withdrawer,
            amount,
            proof: None,
        }
    }

    fn create_generated_merkle_tree_collection(
        vote_account: Pubkey,
        merkle_root_upload_authority: Pubkey,
        epoch: u64,
    ) -> TestResult<GeneratedMerkleTreeCollectionFixture> {
        let claimant_staker_withdrawer = Pubkey::new_unique();

        let test_delegation = Delegation {
            stake_account_pubkey: claimant_staker_withdrawer,
            staker_pubkey: claimant_staker_withdrawer,
            withdrawer_pubkey: claimant_staker_withdrawer,
            lamports_delegated: 50,
        };

        let vote_account_stake_meta = StakeMeta {
            validator_vote_account: vote_account,
            validator_node_pubkey: Pubkey::new_unique(),
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                merkle_root_upload_authority,
                tip_distribution_pubkey: derive_tip_distribution_account_address(
                    &jito_tip_distribution::id(),
                    &vote_account,
                    epoch,
                )
                .0,
                total_tips: 50,
                validator_fee_bps: 0,
            }),
            delegations: vec![test_delegation.clone()],
            total_delegated: 50,
            commission: 0,
        };

        let other_validator = Pubkey::new_unique();
        let other_stake_meta = StakeMeta {
            validator_vote_account: other_validator,
            validator_node_pubkey: Pubkey::new_unique(),
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                merkle_root_upload_authority: other_validator,
                tip_distribution_pubkey: derive_tip_distribution_account_address(
                    &jito_tip_distribution::id(),
                    &other_validator,
                    epoch,
                )
                .0,
                total_tips: 50,
                validator_fee_bps: 0,
            }),
            delegations: vec![test_delegation],
            total_delegated: 50,
            commission: 0,
        };

        let stake_meta_collection = StakeMetaCollection {
            stake_metas: vec![vote_account_stake_meta, other_stake_meta],
            tip_distribution_program_id: Pubkey::new_unique(),
            bank_hash: String::default(),
            epoch,
            slot: 0,
        };

        let collection =
            GeneratedMerkleTreeCollection::new_from_stake_meta_collection(stake_meta_collection)
                .map_err(TestError::from)?;

        let test_tip_distribution_account = derive_tip_distribution_account_address(
            &jito_tip_distribution::id(),
            &vote_account,
            epoch,
        )
        .0;
        let test_generated_merkle_tree = collection
            .generated_merkle_trees
            .iter()
            .find(|tree| tree.tip_distribution_account == test_tip_distribution_account)
            .unwrap();

        Ok(GeneratedMerkleTreeCollectionFixture {
            test_generated_merkle_tree: test_generated_merkle_tree.clone(),
            collection,
        })
    }

    struct MetaMerkleTreeFixture {
        // Contains the individual validator's merkle trees, with the TreeNode idata needed to invoke the set_merkle_root instruction (root, max_num_nodes, max_total_claim)
        pub generated_merkle_tree_fixture: GeneratedMerkleTreeCollectionFixture,
        // Contains meta merkle tree with the root that all validators vote on, and proofs needed to verify the input data
        pub meta_merkle_tree: MetaMerkleTree,
    }

    fn create_meta_merkle_tree(
        vote_account: Pubkey,
        merkle_root_upload_authority: Pubkey,
        epoch: u64,
    ) -> TestResult<MetaMerkleTreeFixture> {
        let generated_merkle_tree_fixture = create_generated_merkle_tree_collection(
            vote_account,
            merkle_root_upload_authority,
            epoch,
        )
        .map_err(TestError::from)?;

        let meta_merkle_tree = MetaMerkleTree::new_from_generated_merkle_tree_collection(
            generated_merkle_tree_fixture.collection.clone(),
        )?;

        Ok(MetaMerkleTreeFixture {
            generated_merkle_tree_fixture,
            meta_merkle_tree,
        })
    }

    #[tokio::test]
    async fn test_set_merkle_root_ok() -> TestResult<()> {
        let mut fixture = TestBuilder::new().await;
        let mut tip_router_client = fixture.tip_router_client();
        let mut tip_distribution_client = fixture.tip_distribution_client();

        let test_ncn = fixture.create_test_ncn().await?;
        let ncn_address = test_ncn.ncn_root.ncn_pubkey;
        let ncn_config_address =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_address).0;

        let epoch = 0;

        tip_distribution_client
            .do_initialize(ncn_config_address)
            .await?;
        let vote_account = tip_distribution_client.setup_vote_account().await?;

        tip_distribution_client
            .do_initialize_tip_distribution_account(ncn_config_address, vote_account, epoch, 100)
            .await?;

        let meta_merkle_tree_fixture =
            create_meta_merkle_tree(vote_account, ncn_config_address, epoch)?;
        let winning_root = meta_merkle_tree_fixture.meta_merkle_tree.merkle_root;

        let (ballot_box_address, bump, _) =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn_address, epoch);

        let ballot_box_fixture = {
            let mut ballot_box = BallotBox::new(ncn_address, epoch, bump, 0);
            let winning_ballot = Ballot::new(winning_root);
            ballot_box.set_winning_ballot(winning_ballot);
            // TODO set other fields to make this ballot box realistic
            ballot_box
        };

        fixture
            .set_account(
                ballot_box_address,
                serialized_ballot_box_account(&ballot_box_fixture),
            )
            .await;

        Ok(())
    }

    // Failure cases:
    // - wrong TDA
    // - ballot box not finalized
    // - proof is incorrect
    // - Merkle root already uploaded?
}
