use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use meta_merkle_tree::{meta_merkle_tree::LEAF_PREFIX, tree_node::TreeNode};
use shank::{ShankAccount, ShankType};
use solana_program::{
    account_info::AccountInfo, hash::hashv, msg, program_error::ProgramError, pubkey::Pubkey,
};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::{precise_consensus, DEFAULT_CONSENSUS_REACHED_SLOT, MAX_OPERATORS},
    discriminators::Discriminators,
    error::TipRouterError,
    loaders::check_load,
    ncn_fee_group::NcnFeeGroup,
    stake_weight::StakeWeights,
};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct Ballot {
    /// The merkle root of the meta merkle tree
    meta_merkle_root: [u8; 32],
    /// Whether the ballot is valid
    is_valid: PodBool,
    /// Reserved space
    reserved: [u8; 63],
}

impl PartialEq for Ballot {
    fn eq(&self, other: &Self) -> bool {
        if !self.is_valid() || !other.is_valid() {
            return false;
        }
        self.meta_merkle_root == other.meta_merkle_root
    }
}

impl Eq for Ballot {}

impl Default for Ballot {
    fn default() -> Self {
        Self {
            meta_merkle_root: [0; 32],
            is_valid: PodBool::from(false),
            reserved: [0; 63],
        }
    }
}

impl std::fmt::Display for Ballot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.meta_merkle_root)
    }
}

impl Ballot {
    pub fn new(merkle_root: &[u8; 32]) -> Self {
        let mut ballot = Self {
            meta_merkle_root: *merkle_root,
            is_valid: PodBool::from(false),
            reserved: [0; 63],
        };

        for byte in ballot.meta_merkle_root.iter() {
            if *byte != 0 {
                ballot.is_valid = PodBool::from(true);
                break;
            }
        }

        ballot
    }

    pub const fn root(&self) -> [u8; 32] {
        self.meta_merkle_root
    }

    pub fn is_valid(&self) -> bool {
        self.is_valid.into()
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct BallotTally {
    /// Index of the tally within the ballot_tallies
    index: PodU16,
    /// The ballot being tallied
    ballot: Ballot,
    /// Breakdown of all of the stake weights that contribute to the vote
    stake_weights: StakeWeights,
    /// The number of votes for this ballot
    tally: PodU64,
    // reserved: [u8; 64],
}

impl Default for BallotTally {
    fn default() -> Self {
        Self {
            index: PodU16::from(u16::MAX),
            ballot: Ballot::default(),
            stake_weights: StakeWeights::default(),
            tally: PodU64::from(0),
            // reserved: [0; 64],
        }
    }
}

impl BallotTally {
    pub fn new(index: u16, ballot: &Ballot, stake_weights: &StakeWeights) -> Self {
        Self {
            index: PodU16::from(index),
            ballot: *ballot,
            stake_weights: *stake_weights,
            tally: PodU64::from(1),
            // reserved: [0; 64],
        }
    }

    pub const fn ballot(&self) -> &Ballot {
        &self.ballot
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weights
    }

    pub fn tally(&self) -> u64 {
        self.tally.into()
    }

    pub fn index(&self) -> u16 {
        self.index.into()
    }

    pub fn is_valid(&self) -> bool {
        self.ballot.is_valid()
    }

    pub fn increment_tally(&mut self, stake_weights: &StakeWeights) -> Result<(), TipRouterError> {
        self.stake_weights.increment(stake_weights)?;
        self.tally = PodU64::from(
            self.tally()
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    pub fn decrement_tally(&mut self, stake_weights: &StakeWeights) -> Result<(), TipRouterError> {
        self.stake_weights.decrement(stake_weights)?;
        self.tally = PodU64::from(
            self.tally()
                .checked_sub(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct OperatorVote {
    /// The operator that cast the vote
    operator: Pubkey,
    /// The slot the operator voted
    slot_voted: PodU64,
    /// The stake weights of the operator
    stake_weights: StakeWeights,
    /// The index of the ballot in the ballot_tallies
    ballot_index: PodU16,
    /// Reserved space
    reserved: [u8; 64],
}

impl Default for OperatorVote {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            slot_voted: PodU64::from(0),
            stake_weights: StakeWeights::default(),
            ballot_index: PodU16::from(u16::MAX),
            reserved: [0; 64],
        }
    }
}

impl OperatorVote {
    pub fn new(
        ballot_index: usize,
        operator: &Pubkey,
        current_slot: u64,
        stake_weights: &StakeWeights,
    ) -> Self {
        Self {
            operator: *operator,
            ballot_index: PodU16::from(ballot_index as u16),
            slot_voted: PodU64::from(current_slot),
            stake_weights: *stake_weights,
            reserved: [0; 64],
        }
    }

    pub const fn operator(&self) -> &Pubkey {
        &self.operator
    }

    pub fn slot_voted(&self) -> u64 {
        self.slot_voted.into()
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weights
    }

    pub fn ballot_index(&self) -> u16 {
        self.ballot_index.into()
    }

    pub fn is_empty(&self) -> bool {
        self.ballot_index() == u16::MAX
    }
}

// PDA'd ["epoch_snapshot", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct BallotBox {
    /// The NCN account this ballot box is for
    ncn: Pubkey,
    /// The epoch this ballot box is for
    epoch: PodU64,
    /// Bump seed for the PDA
    bump: u8,
    /// Slot when this ballot box was created
    slot_created: PodU64,
    /// Slot when consensus was reached
    slot_consensus_reached: PodU64,
    /// Reserved space
    reserved: [u8; 128],
    /// Number of operators that have voted
    operators_voted: PodU64,
    /// Number of unique ballots
    unique_ballots: PodU64,
    /// The ballot that got at least 66% of votes
    winning_ballot: Ballot,
    /// Operator votes
    operator_votes: [OperatorVote; 256],
    /// Mapping of ballots votes to stake weight
    ballot_tallies: [BallotTally; 256],
}

impl Discriminator for BallotBox {
    const DISCRIMINATOR: u8 = Discriminators::BallotBox as u8;
}

impl BallotBox {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: &Pubkey, epoch: u64, bump: u8, current_slot: u64) -> Self {
        Self {
            ncn: *ncn,
            epoch: PodU64::from(epoch),
            bump,
            slot_created: PodU64::from(current_slot),
            slot_consensus_reached: PodU64::from(DEFAULT_CONSENSUS_REACHED_SLOT),
            operators_voted: PodU64::from(0),
            unique_ballots: PodU64::from(0),
            winning_ballot: Ballot::default(),
            operator_votes: [OperatorVote::default(); MAX_OPERATORS],
            ballot_tallies: [BallotTally::default(); MAX_OPERATORS],
            reserved: [0; 128],
        }
    }

    pub fn initialize(&mut self, ncn: &Pubkey, epoch: u64, bump: u8, current_slot: u64) {
        // Avoids overflowing stack
        self.ncn = *ncn;
        self.epoch = PodU64::from(epoch);
        self.bump = bump;
        self.slot_created = PodU64::from(current_slot);
        self.slot_consensus_reached = PodU64::from(DEFAULT_CONSENSUS_REACHED_SLOT);
        self.operators_voted = PodU64::from(0);
        self.unique_ballots = PodU64::from(0);
        self.winning_ballot = Ballot::default();
        self.operator_votes = [OperatorVote::default(); MAX_OPERATORS];
        self.ballot_tallies = [BallotTally::default(); MAX_OPERATORS];
        self.reserved = [0; 128];
    }

    pub fn seeds(ncn: &Pubkey, epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"ballot_box".to_vec(),
                ncn.to_bytes().to_vec(),
                epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn, epoch).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn load_to_close(
        program_id: &Pubkey,
        account_to_close: &AccountInfo,
        ncn: &Pubkey,
        epoch: u64,
    ) -> Result<(), ProgramError> {
        Self::load(program_id, account_to_close, ncn, epoch, true)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn slot_consensus_reached(&self) -> u64 {
        self.slot_consensus_reached.into()
    }

    pub fn unique_ballots(&self) -> u64 {
        self.unique_ballots.into()
    }

    pub fn operators_voted(&self) -> u64 {
        self.operators_voted.into()
    }

    pub fn has_ballot(&self, ballot: &Ballot) -> bool {
        self.ballot_tallies.iter().any(|t| t.ballot.eq(ballot))
    }

    pub const fn ballot_tallies(&self) -> &[BallotTally; MAX_OPERATORS] {
        &self.ballot_tallies
    }

    pub fn is_consensus_reached(&self) -> bool {
        self.slot_consensus_reached() != DEFAULT_CONSENSUS_REACHED_SLOT
            || self.winning_ballot.is_valid()
    }

    pub fn tie_breaker_set(&self) -> bool {
        self.slot_consensus_reached() == DEFAULT_CONSENSUS_REACHED_SLOT
            && self.winning_ballot.is_valid()
    }

    pub fn get_winning_ballot(&self) -> Result<&Ballot, TipRouterError> {
        if !self.winning_ballot.is_valid() {
            Err(TipRouterError::ConsensusNotReached)
        } else {
            Ok(&self.winning_ballot)
        }
    }

    pub fn get_winning_ballot_tally(&self) -> Result<&BallotTally, TipRouterError> {
        if !self.winning_ballot.is_valid() {
            Err(TipRouterError::ConsensusNotReached)
        } else {
            let winning_ballot_tally = self
                .ballot_tallies
                .iter()
                .find(|t| t.ballot.eq(&self.winning_ballot))
                .ok_or(TipRouterError::BallotTallyNotFoundFull)?;

            Ok(winning_ballot_tally)
        }
    }

    pub fn has_winning_ballot(&self) -> bool {
        self.winning_ballot.is_valid()
    }

    pub const fn operator_votes(&self) -> &[OperatorVote; MAX_OPERATORS] {
        &self.operator_votes
    }

    pub fn set_winning_ballot(&mut self, ballot: &Ballot) {
        self.winning_ballot = *ballot;
    }

    fn increment_or_create_ballot_tally(
        &mut self,
        ballot: &Ballot,
        stake_weights: &StakeWeights,
    ) -> Result<usize, TipRouterError> {
        let result = self
            .ballot_tallies
            .iter()
            .enumerate()
            .find(|(_, t)| t.is_valid() && t.ballot.eq(ballot));

        if let Some((tally_index, _)) = result {
            self.ballot_tallies[tally_index].increment_tally(stake_weights)?;
            return Ok(tally_index);
        }

        for (tally_index, tally) in self.ballot_tallies.iter_mut().enumerate() {
            if !tally.is_valid() {
                *tally = BallotTally::new(tally_index as u16, ballot, stake_weights);

                self.unique_ballots = PodU64::from(
                    self.unique_ballots()
                        .checked_add(1)
                        .ok_or(TipRouterError::ArithmeticOverflow)?,
                );

                return Ok(tally_index);
            }
        }

        Err(TipRouterError::BallotTallyFull)
    }

    pub fn cast_vote(
        &mut self,
        operator: &Pubkey,
        ballot: &Ballot,
        stake_weights: &StakeWeights,
        current_slot: u64,
        valid_slots_after_consensus: u64,
    ) -> Result<(), TipRouterError> {
        if !self.is_voting_valid(current_slot, valid_slots_after_consensus)? {
            return Err(TipRouterError::VotingNotValid);
        }

        if !ballot.is_valid() {
            return Err(TipRouterError::BadBallot);
        }

        let ballot_index = self.increment_or_create_ballot_tally(ballot, stake_weights)?;

        let unique_ballots = self.unique_ballots();
        let consensus_reached = self.is_consensus_reached();

        for vote in self.operator_votes.iter_mut() {
            if vote.operator().eq(operator) {
                if consensus_reached {
                    return Err(TipRouterError::ConsensusAlreadyReached);
                }

                // If the operator has already voted, we need to decrement their vote from the previous ballot
                let prev_ballot_index = vote.ballot_index();
                if let Some(prev_tally) = self.ballot_tallies.get_mut(prev_ballot_index as usize) {
                    prev_tally.decrement_tally(vote.stake_weights())?;

                    // If no more operators voting for the previous ballot, wipe and decrement the unique ballots
                    if prev_tally.tally() == 0 {
                        *prev_tally = BallotTally::default();
                        self.unique_ballots = PodU64::from(
                            unique_ballots
                                .checked_sub(1)
                                .ok_or(TipRouterError::ArithmeticOverflow)?,
                        );
                    }
                }

                let operator_vote =
                    OperatorVote::new(ballot_index, operator, current_slot, stake_weights);
                *vote = operator_vote;
                return Ok(());
            }

            if vote.is_empty() {
                let operator_vote =
                    OperatorVote::new(ballot_index, operator, current_slot, stake_weights);
                *vote = operator_vote;

                self.operators_voted = PodU64::from(
                    self.operators_voted()
                        .checked_add(1)
                        .ok_or(TipRouterError::ArithmeticOverflow)?,
                );
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorVotesFull)
    }

    // Should be called anytime a new vote is cast
    pub fn tally_votes(
        &mut self,
        total_stake_weight: u128,
        current_slot: u64,
    ) -> Result<(), TipRouterError> {
        if self.slot_consensus_reached() != DEFAULT_CONSENSUS_REACHED_SLOT {
            return Ok(());
        }

        let max_tally = self
            .ballot_tallies
            .iter()
            .max_by_key(|t| t.stake_weights().stake_weight())
            .unwrap();

        let ballot_stake_weight = max_tally.stake_weights().stake_weight();
        let precise_ballot_stake_weight =
            PreciseNumber::new(ballot_stake_weight).ok_or(TipRouterError::NewPreciseNumberError)?;
        let precise_total_stake_weight =
            PreciseNumber::new(total_stake_weight).ok_or(TipRouterError::NewPreciseNumberError)?;

        let ballot_percentage_of_total = precise_ballot_stake_weight
            .checked_div(&precise_total_stake_weight)
            .ok_or(TipRouterError::DenominatorIsZero)?;

        let target_precise_percentage = precise_consensus()?;

        let consensus_reached =
            ballot_percentage_of_total.greater_than_or_equal(&target_precise_percentage);

        if consensus_reached && !self.winning_ballot.is_valid() {
            self.slot_consensus_reached = PodU64::from(current_slot);
            let winning_ballot = *max_tally.ballot();

            self.set_winning_ballot(&winning_ballot);
        }

        Ok(())
    }

    pub fn set_tie_breaker_ballot(
        &mut self,
        meta_merkle_root: &[u8; 32],
        current_epoch: u64,
        epochs_before_stall: u64,
    ) -> Result<(), TipRouterError> {
        // Check that consensus has not been reached
        if self.is_consensus_reached() {
            msg!("Consensus already reached");
            return Err(TipRouterError::ConsensusAlreadyReached);
        }

        // Check if voting is stalled and setting the tie breaker is eligible
        if current_epoch
            < self
                .epoch()
                .checked_add(epochs_before_stall)
                .ok_or(TipRouterError::ArithmeticOverflow)?
        {
            return Err(TipRouterError::VotingNotFinalized);
        }

        let finalized_ballot = Ballot::new(meta_merkle_root);

        // // Check that the merkle root is one of the existing options
        if !self.has_ballot(&finalized_ballot) {
            return Err(TipRouterError::TieBreakerNotInPriorVotes);
        }

        self.set_winning_ballot(&finalized_ballot);
        Ok(())
    }

    /// Determines if an operator can still cast their vote.
    /// Returns true when:
    /// Consensus is not reached OR the voting window is still valid, assuming set_tie_breaker was not invoked
    pub fn is_voting_valid(
        &self,
        current_slot: u64,
        valid_slots_after_consensus: u64,
    ) -> Result<bool, TipRouterError> {
        if self.tie_breaker_set() {
            return Ok(false);
        }

        if self.is_consensus_reached() {
            let vote_window_valid = current_slot
                <= self
                    .slot_consensus_reached()
                    .checked_add(valid_slots_after_consensus)
                    .ok_or(TipRouterError::ArithmeticOverflow)?;

            return Ok(vote_window_valid);
        }

        Ok(true)
    }

    pub fn verify_merkle_root(
        &self,
        tip_distribution_account: &Pubkey,
        proof: Vec<[u8; 32]>,
        merkle_root: &[u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
    ) -> Result<(), TipRouterError> {
        let tree_node = TreeNode::new(
            tip_distribution_account,
            merkle_root,
            max_total_claim,
            max_num_nodes,
        );

        let node_hash = hashv(&[LEAF_PREFIX, &tree_node.hash().to_bytes()]);

        if !meta_merkle_tree::verify::verify(
            proof,
            self.winning_ballot.root(),
            node_hash.to_bytes(),
        ) {
            return Err(TipRouterError::InvalidMerkleProof);
        }

        Ok(())
    }
}

#[rustfmt::skip]
impl fmt::Display for BallotBox {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       writeln!(f, "\n\n----------- Ballot Box -------------")?;
       writeln!(f, "  NCN:                          {}", self.ncn)?;
       writeln!(f, "  Epoch:                        {}", self.epoch())?;
       writeln!(f, "  Bump:                         {}", self.bump)?;
       writeln!(f, "  Slot Consensus Reached:       {}", self.slot_consensus_reached())?;
       writeln!(f, "  Operators Voted:              {}", self.operators_voted())?;
       writeln!(f, "  Unique Ballots:               {}", self.unique_ballots())?;
       writeln!(f, "  IS Consensus Reached:         {}", self.is_consensus_reached())?;
       if self.is_consensus_reached() {
           writeln!(f, "  Tie Breaker Set:              {}", self.tie_breaker_set())?;
           if let Ok(winning_ballot) = self.get_winning_ballot() {
               writeln!(f, "  Winning Ballot:               {}", winning_ballot)?;
           }
       }

       writeln!(f, "\nOperator Votes:")?;
       for vote in self.operator_votes().iter() {
           if !vote.is_empty() {
               writeln!(f, "  Operator:                     {}", vote.operator())?;
               writeln!(f, "    Slot Voted:                 {}", vote.slot_voted())?;
               writeln!(f, "    Ballot Index:               {}", vote.ballot_index())?;
               writeln!(f, "    Stake Weights:")?;
               let weights = vote.stake_weights();
               for group in NcnFeeGroup::all_groups() {
                   if let Ok(weight) = weights.ncn_fee_group_stake_weight(group) {
                       if weight > 0 {
                           writeln!(f, "      Group {}:                  {}", group.group, weight)?;
                       }
                   }
               }
           }
       }

       writeln!(f, "\nBallot Tallies:")?;
       for tally in self.ballot_tallies().iter() {
           if tally.is_valid() {
               writeln!(f, "  Index {}:", tally.index())?;
               writeln!(f, "    Ballot:                     {}", tally.ballot())?;
               writeln!(f, "    Tally:                      {}", tally.tally())?;
               writeln!(f, "    Stake Weights:")?;
               let weights = tally.stake_weights();
               for group in NcnFeeGroup::all_groups() {
                   if let Ok(weight) = weights.ncn_fee_group_stake_weight(group) {
                       if weight > 0 {
                           writeln!(f, "      Group {}:                  {}", group.group, weight)?;
                       }
                   }
               }
           }
       }

       writeln!(f, "\n")?;
       Ok(())
   }
}

#[cfg(test)]
mod tests {
    use crate::utils::assert_tip_router_error;

    use super::*;

    #[test]
    fn test_len() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // epoch
            + 1 // bump
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // slot_consensus_reached
            + 128 // reserved
            + size_of::<PodU64>() // operators_voted
            + size_of::<PodU64>() // unique_ballots
            + size_of::<Ballot>() // winning_ballot
            + size_of::<OperatorVote>() * MAX_OPERATORS // operator_votes
            + size_of::<BallotTally>() * MAX_OPERATORS; // ballot_tallies

        assert_eq!(size_of::<BallotBox>(), expected_total);

        let ballot_box = BallotBox::new(&Pubkey::default(), 0, 0, 0);
        assert_eq!(ballot_box.operator_votes.len(), MAX_OPERATORS);
        assert_eq!(ballot_box.ballot_tallies.len(), MAX_OPERATORS);
    }

    #[test]
    fn test_verify_merkle_root() {
        use meta_merkle_tree::meta_merkle_tree::MetaMerkleTree;

        // Create test data with unique pubkeys
        let tip_distribution1 = Pubkey::new_unique();
        let tip_distribution2 = Pubkey::new_unique();
        let tip_distribution3 = Pubkey::new_unique();
        let max_total_claim = 1000;
        let max_num_nodes = 10;

        // Create tree nodes with unique tip_distribution_accounts
        let mut tree_nodes = vec![
            TreeNode::new(&tip_distribution1, &[1; 32], max_total_claim, max_num_nodes),
            TreeNode::new(&tip_distribution2, &[2; 32], max_total_claim, max_num_nodes),
            TreeNode::new(&tip_distribution3, &[3; 32], max_total_claim, max_num_nodes),
        ];

        // Sort nodes by hash (required for consistent tree creation)
        tree_nodes.sort_by_key(|node| node.hash());

        // Build the merkle tree
        let meta_merkle_tree = MetaMerkleTree::new(tree_nodes).unwrap();

        // Initialize ballot box and set the winning ballot with the merkle root
        let mut ballot_box = BallotBox::new(&Pubkey::default(), 0, 0, 0);
        let winning_ballot = Ballot::new(&meta_merkle_tree.merkle_root);
        ballot_box.set_winning_ballot(&winning_ballot);

        // Get the first node and its proof from the merkle tree
        let test_node = &meta_merkle_tree.tree_nodes[0];
        let valid_proof = test_node.proof.clone().unwrap();

        // Test with valid proof - use the specific tip_distribution_account from the test node
        let result = ballot_box.verify_merkle_root(
            &test_node.tip_distribution_account,
            valid_proof.clone(),
            &test_node.validator_merkle_root,
            max_total_claim,
            max_num_nodes,
        );
        assert!(result.is_ok(), "Valid proof should succeed");

        // Test with invalid proof (modify one hash in the proof)
        let mut invalid_proof = valid_proof;
        if let Some(first_hash) = invalid_proof.first_mut() {
            first_hash[0] ^= 0xFF; // Flip some bits to make it invalid
        }

        let result = ballot_box.verify_merkle_root(
            &test_node.tip_distribution_account,
            invalid_proof,
            &test_node.validator_merkle_root,
            max_total_claim,
            max_num_nodes,
        );
        assert_eq!(
            result,
            Err(TipRouterError::InvalidMerkleProof),
            "Invalid proof should fail"
        );
    }

    #[test]
    fn test_cast_vote() {
        let ncn = Pubkey::new_unique();
        let operator = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let stake_weights = StakeWeights::new(1000);
        let valid_slots_after_consensus = 10;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);
        let ballot = Ballot::new(&[1; 32]);

        // Test initial cast vote
        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify vote was recorded correctly
        let operator_vote = ballot_box
            .operator_votes
            .iter()
            .find(|v| v.operator().eq(&operator))
            .unwrap();
        assert_eq!(
            operator_vote.stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );
        assert_eq!(operator_vote.slot_voted(), current_slot);

        // Verify ballot tally
        let tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .unwrap();
        assert_eq!(
            tally.stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );

        // Test re-vote with different ballot
        let new_ballot = Ballot::new(&[2u8; 32]);
        let new_slot = current_slot + 1;
        ballot_box
            .cast_vote(
                &operator,
                &new_ballot,
                &stake_weights,
                new_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify new ballot tally increased
        let new_tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot().eq(&new_ballot))
            .unwrap();
        assert_eq!(
            new_tally.stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );

        // Test error on changing vote after consensus
        let winning_ballot = *new_tally.ballot();
        ballot_box.set_winning_ballot(&winning_ballot);
        ballot_box.slot_consensus_reached = PodU64::from(new_slot);
        let result = ballot_box.cast_vote(
            &operator,
            &ballot,
            &stake_weights,
            new_slot + 1,
            valid_slots_after_consensus,
        );
        assert!(matches!(
            result,
            Err(TipRouterError::ConsensusAlreadyReached)
        ));

        // Test voting window expired after consensus
        let result = ballot_box.cast_vote(
            &operator,
            &ballot,
            &stake_weights,
            new_slot + valid_slots_after_consensus + 1,
            valid_slots_after_consensus,
        );
        assert!(matches!(result, Err(TipRouterError::VotingNotValid)));
    }

    #[test]
    fn test_get_winning_ballot() {
        // Create a new ballot box (should have no winning ballot)
        let ballot_box = BallotBox::new(&Pubkey::default(), 0, 0, 0);

        // Test with no winning ballot initialized
        let result = ballot_box.get_winning_ballot();
        assert_eq!(
            result,
            Err(TipRouterError::ConsensusNotReached),
            "Should return ConsensusNotReached when no winning ballot is set"
        );

        // Create a new ballot box and set a winning ballot
        let mut ballot_box = BallotBox::new(&Pubkey::default(), 0, 0, 0);
        let expected_ballot = Ballot::new(&[1; 32]);
        ballot_box.set_winning_ballot(&expected_ballot);

        // Test with winning ballot set
        let result = ballot_box.get_winning_ballot();
        assert!(result.is_ok(), "Should succeed when winning ballot is set");
        assert_eq!(
            result.unwrap(),
            &expected_ballot,
            "Should return the correct winning ballot"
        );
    }

    #[test]
    fn test_operator_votes_full() {
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 10;
        let mut ballot_box = BallotBox::new(&Pubkey::default(), epoch, 0, current_slot);
        let ballot = Ballot::new(&[1; 32]);
        let stake_weights = StakeWeights::new(1000);

        // Fill up all operator vote slots (MAX_OPERATORS = 256)
        for _ in 0..MAX_OPERATORS {
            let operator = Pubkey::new_unique();
            let result = ballot_box.cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                current_slot,
                valid_slots_after_consensus,
            );
            assert!(result.is_ok(), "Vote should succeed when slots available");
        }

        // Try to add one more vote, which should fail
        let extra_operator = Pubkey::new_unique();
        let result = ballot_box.cast_vote(
            &extra_operator,
            &ballot,
            &stake_weights,
            current_slot,
            valid_slots_after_consensus,
        );
        assert_eq!(
            result,
            Err(TipRouterError::OperatorVotesFull),
            "Should return OperatorVotesFull when no slots available"
        );
    }

    #[test]
    fn test_increment_or_create_ballot_tally() {
        let mut ballot_box = BallotBox::new(&Pubkey::new_unique(), 1, 1, 1);
        let ballot = Ballot::new(&[1u8; 32]);
        let stake_weights = StakeWeights::new(100);

        // Test creating new ballot tally
        let tally_index = ballot_box
            .increment_or_create_ballot_tally(&ballot, &stake_weights)
            .unwrap();
        assert_eq!(tally_index, 0);
        assert_eq!(ballot_box.unique_ballots(), 1);
        assert_eq!(
            ballot_box.ballot_tallies[0].stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );
        assert_eq!(*ballot_box.ballot_tallies[0].ballot(), ballot);

        // Test incrementing existing ballot tally
        let tally_index = ballot_box
            .increment_or_create_ballot_tally(&ballot, &stake_weights)
            .unwrap();
        assert_eq!(tally_index, 0);
        assert_eq!(ballot_box.unique_ballots(), 1);
        assert_eq!(
            ballot_box.ballot_tallies[0].stake_weights().stake_weight(),
            stake_weights.stake_weight() * 2
        );
        assert_eq!(*ballot_box.ballot_tallies[0].ballot(), ballot);

        // Test creating second ballot tally
        let ballot2 = Ballot::new(&[2u8; 32]);
        let tally_index = ballot_box
            .increment_or_create_ballot_tally(&ballot2, &stake_weights)
            .unwrap();
        assert_eq!(tally_index, 1);
        assert_eq!(ballot_box.unique_ballots(), 2);
        assert_eq!(
            ballot_box.ballot_tallies[1].stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );
        assert_eq!(*ballot_box.ballot_tallies[1].ballot(), ballot2);

        // Test error when ballot tallies are full
        for _ in 3..=ballot_box.ballot_tallies.len() {
            let ballot = Ballot::new(&Pubkey::new_unique().to_bytes());
            ballot_box
                .increment_or_create_ballot_tally(&ballot, &stake_weights)
                .unwrap();
        }
        let ballot_full = Ballot::new(&Pubkey::new_unique().to_bytes());
        let result = ballot_box.increment_or_create_ballot_tally(&ballot_full, &stake_weights);
        assert!(matches!(result, Err(TipRouterError::BallotTallyFull)));
    }

    #[test]
    fn test_tally_votes() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let quarter_stake_weights = StakeWeights::new(500);
        let half_stake_weights = StakeWeights::new(500);
        let full_stake_weights = StakeWeights::new(1000);
        let total_stake_weight: u128 = 1000;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);
        let ballot = Ballot::new(&[1; 32]);

        // Test no consensus when below threshold
        ballot_box
            .increment_or_create_ballot_tally(&ballot, &half_stake_weights)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(!ballot_box.is_consensus_reached());
        assert_eq!(
            ballot_box.slot_consensus_reached(),
            DEFAULT_CONSENSUS_REACHED_SLOT
        );
        assert!(matches!(
            ballot_box.get_winning_ballot_tally(),
            Err(TipRouterError::ConsensusNotReached)
        ));

        // Test consensus reached when above threshold
        ballot_box
            .increment_or_create_ballot_tally(&ballot, &half_stake_weights)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.slot_consensus_reached(), current_slot);
        assert_eq!(
            *ballot_box.get_winning_ballot_tally().unwrap().ballot(),
            ballot
        );

        // Consensus remains after additional votes
        let ballot2 = Ballot::new(&[2; 32]);
        ballot_box
            .increment_or_create_ballot_tally(&ballot2, &full_stake_weights)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot + 1)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.slot_consensus_reached(), current_slot);
        assert_eq!(
            *ballot_box.get_winning_ballot_tally().unwrap().ballot(),
            ballot
        );

        // Test with multiple competing ballots
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);
        let ballot1 = Ballot::new(&[1; 32]);
        let ballot2 = Ballot::new(&[2; 32]);
        let ballot3 = Ballot::new(&[3; 32]);

        ballot_box
            .increment_or_create_ballot_tally(&ballot1, &quarter_stake_weights)
            .unwrap();
        ballot_box
            .increment_or_create_ballot_tally(&ballot2, &quarter_stake_weights)
            .unwrap();
        ballot_box
            .increment_or_create_ballot_tally(&ballot3, &half_stake_weights)
            .unwrap();

        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(!ballot_box.is_consensus_reached());

        // Add more votes to reach consensus
        ballot_box
            .increment_or_create_ballot_tally(&ballot3, &half_stake_weights)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(
            *ballot_box.get_winning_ballot_tally().unwrap().ballot(),
            ballot3
        );
    }

    #[test]
    fn test_cast_bad_ballot() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 10;
        let stake_weight_per_operator = 1000;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        let operator1 = Pubkey::new_unique();

        let stake_weights = StakeWeights::new(stake_weight_per_operator);
        let ballot1 = Ballot::new(&[0; 32]);

        // Operator 1 votes for ballot1 initially
        let result = ballot_box.cast_vote(
            &operator1,
            &ballot1,
            &stake_weights,
            current_slot,
            valid_slots_after_consensus,
        );

        assert_tip_router_error(result, TipRouterError::BadBallot);
    }

    #[test]
    fn test_multiple_operators_converging_votes() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 10;
        let stake_weight_per_operator = 1000;
        let total_stake_weight = stake_weight_per_operator * 3;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        let operator1 = Pubkey::new_unique();
        let operator2 = Pubkey::new_unique();
        let operator3 = Pubkey::new_unique();

        let stake_weights = StakeWeights::new(stake_weight_per_operator);
        let ballot1 = Ballot::new(&[1; 32]);
        let ballot2 = Ballot::new(&[2; 32]);

        // Operator 1 votes for ballot1 initially
        ballot_box
            .cast_vote(
                &operator1,
                &ballot1,
                &stake_weights,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        assert_eq!(ballot_box.unique_ballots(), 1);

        // Operator 1 changes vote to ballot2
        ballot_box
            .cast_vote(
                &operator1,
                &ballot2,
                &stake_weights,
                current_slot + 1,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Ballot1 should be removed and ballot2 should be the only ballot
        assert_eq!(ballot_box.unique_ballots(), 1);

        // Operator 2 votes for ballot2
        ballot_box
            .cast_vote(
                &operator2,
                &ballot2,
                &stake_weights,
                current_slot + 2,
                valid_slots_after_consensus,
            )
            .unwrap();
        assert_eq!(ballot_box.unique_ballots(), 1);

        // Operator 3 votes for ballot2
        ballot_box
            .cast_vote(
                &operator3,
                &ballot2,
                &stake_weights,
                current_slot + 3,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Still only one unique ballot
        assert_eq!(ballot_box.unique_ballots(), 1);

        // Check total stake weight
        let winning_tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot().eq(&ballot2))
            .unwrap();

        assert_eq!(
            winning_tally.stake_weights().stake_weight(),
            total_stake_weight as u128
        );
        assert_eq!(winning_tally.tally(), 3);

        // Verify ballot2 wins consensus with all votes
        ballot_box
            .tally_votes(total_stake_weight as u128, current_slot + 4)
            .unwrap();
        assert!(ballot_box.has_winning_ballot());
        assert_eq!(*ballot_box.get_winning_ballot().unwrap(), ballot2);
    }

    #[test]
    fn test_set_tie_breaker_ballot() {
        let ncn = Pubkey::new_unique();
        let epoch = 0;
        let current_slot = 1000;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Create some initial ballots
        let ballot1 = Ballot::new(&[1; 32]);
        let ballot2 = Ballot::new(&[2; 32]);
        let stake_weights = StakeWeights::new(100);
        let double_stake_weights = StakeWeights::new(200);

        ballot_box
            .increment_or_create_ballot_tally(&ballot1, &stake_weights)
            .unwrap();
        ballot_box
            .increment_or_create_ballot_tally(&ballot2, &stake_weights)
            .unwrap();

        // Test setting tie breaker before voting is stalled
        let current_epoch = epoch + 1;
        let epochs_before_stall = 3;

        assert_eq!(
            ballot_box.set_tie_breaker_ballot(&ballot1.root(), current_epoch, epochs_before_stall),
            Err(TipRouterError::VotingNotFinalized)
        );

        // Test setting tie breaker after voting is stalled (current_epoch >= epoch + epochs_before_stall)
        let current_epoch = epoch + epochs_before_stall;
        ballot_box
            .set_tie_breaker_ballot(&ballot1.root(), current_epoch, epochs_before_stall)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(
            *ballot_box.get_winning_ballot_tally().unwrap().ballot(),
            ballot1
        );

        // Test setting tie breaker with invalid merkle root
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);
        ballot_box
            .increment_or_create_ballot_tally(&ballot1, &stake_weights)
            .unwrap();
        assert_eq!(
            ballot_box.set_tie_breaker_ballot(&[99; 32], current_epoch, epochs_before_stall),
            Err(TipRouterError::TieBreakerNotInPriorVotes)
        );

        // Test setting tie breaker when consensus already reached
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);
        ballot_box
            .increment_or_create_ballot_tally(&ballot1, &double_stake_weights)
            .unwrap();
        ballot_box
            .tally_votes(double_stake_weights.stake_weight(), current_slot)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(
            ballot_box.set_tie_breaker_ballot(&ballot1.root(), current_epoch, epochs_before_stall),
            Err(TipRouterError::ConsensusAlreadyReached)
        );
    }

    #[test]
    fn test_cast_vote_stake_weight_accounting() {
        let ncn = Pubkey::new_unique();
        let operator = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let stake_weights = StakeWeights::new(1000);
        let valid_slots_after_consensus = 10;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Initial ballot
        let ballot1 = Ballot::new(&[1; 32]);
        ballot_box
            .cast_vote(
                &operator,
                &ballot1,
                &stake_weights,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify initial stake weight
        let initial_tally = *ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot().eq(&ballot1))
            .unwrap();
        assert_eq!(
            initial_tally.stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );

        // Change vote to new ballot
        let ballot2 = Ballot::new(&[2; 32]);
        ballot_box
            .cast_vote(
                &operator,
                &ballot2,
                &stake_weights,
                current_slot + 1,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify old tally is gone
        let old_tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot().eq(&ballot1));
        assert!(old_tally.is_none());
        assert_eq!(
            ballot_box.ballot_tallies[initial_tally.index() as usize]
                .ballot()
                .root(),
            Ballot::default().root()
        );

        // Verify stake weight moved from ballot1 to ballot2
        let new_tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot().eq(&ballot2))
            .unwrap();
        assert_eq!(
            new_tally.stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );

        // Verify total operators voted hasn't changed
        assert_eq!(ballot_box.operators_voted(), 1);
    }
}

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use std::collections::HashMap;

    // Generate pseudo-random ballot roots using Pubkey's random bytes
    fn generate_ballot_root() -> [u8; 32] {
        Pubkey::new_unique().to_bytes()
    }

    // Generate random stake weight for initial operator assignment
    fn generate_stake_weights() -> StakeWeights {
        let random_bytes = Pubkey::new_unique().to_bytes();
        let stake = u64::from_le_bytes(random_bytes[0..8].try_into().unwrap());
        let stake = (stake % 1_000_000 + 1) as u128;
        StakeWeights::new(stake)
    }

    #[test]
    fn test_fuzz_ballot_box_vote_changes() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Set total stake for the entire test
        let total_stake: u128 = 10_000_000;
        let mut remaining_stake = total_stake;

        // Generate a pool of operators with fixed stake weights
        let num_operators = 50;
        let mut operator_stakes: HashMap<Pubkey, StakeWeights> = HashMap::new();
        let operators: Vec<Pubkey> = (0..num_operators)
            .map(|_| {
                let operator = Pubkey::new_unique();
                let stake_weight = if remaining_stake > 0 {
                    let stake = generate_stake_weights();
                    remaining_stake = remaining_stake.saturating_sub(stake.stake_weight());
                    stake
                } else {
                    StakeWeights::new(0)
                };
                operator_stakes.insert(operator, stake_weight);
                operator
            })
            .collect();

        // Generate a pool of ballots
        let num_ballots = 20;
        let ballots: Vec<Ballot> = (0..num_ballots)
            .map(|_| Ballot::new(&generate_ballot_root()))
            .collect();

        // Perform random operations
        let num_operations = 1000;
        for i in 0..num_operations {
            let random_bytes = Pubkey::new_unique().to_bytes();
            let operator_idx = u64::from_le_bytes(random_bytes[0..8].try_into().unwrap()) as usize
                % operators.len();
            let ballot_idx = u64::from_le_bytes(random_bytes[8..16].try_into().unwrap()) as usize
                % ballots.len();

            let operator = operators[operator_idx];
            let ballot = ballots[ballot_idx];
            let stake_weights = operator_stakes[&operator];
            let slot = current_slot + i as u64;

            // Cast vote and verify BallotBox state
            match ballot_box.cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                slot,
                valid_slots_after_consensus,
            ) {
                Ok(_) => {
                    // Verify operator vote was recorded
                    let operator_vote = ballot_box
                        .operator_votes()
                        .iter()
                        .find(|v| v.operator().eq(&operator))
                        .expect("Operator vote should be recorded");

                    // Verify stake weight hasn't changed
                    assert_eq!(
                        operator_vote.stake_weights().stake_weight(),
                        stake_weights.stake_weight(),
                        "Operator stake weight should never change"
                    );
                    assert_eq!(operator_vote.slot_voted(), slot);

                    // Verify ballot tally exists
                    let _ballot_tally = ballot_box
                        .ballot_tallies()
                        .iter()
                        .find(|t| t.ballot().eq(&ballot))
                        .expect("Ballot tally should exist");

                    // Calculate total stake weight for verification
                    let current_total_stake: u128 = ballot_box
                        .operator_votes()
                        .iter()
                        .filter(|v| !v.is_empty())
                        .map(|v| v.stake_weights().stake_weight())
                        .sum();

                    // Since each operator's stake is fixed, total stake should never exceed initial total
                    assert!(current_total_stake <= total_stake);

                    // Periodically check consensus
                    if i % 10 == 0 {
                        ballot_box.tally_votes(total_stake, slot).unwrap();

                        if ballot_box.is_consensus_reached() {
                            let winning_tally = ballot_box.get_winning_ballot_tally().unwrap();

                            // Verify winning ballot has highest stake
                            for tally in ballot_box.ballot_tallies().iter().filter(|t| t.is_valid())
                            {
                                assert!(
                                    tally.stake_weights().stake_weight()
                                        <= winning_tally.stake_weights().stake_weight()
                                );
                            }

                            // Verify consensus state is consistent
                            assert!(
                                ballot_box.slot_consensus_reached()
                                    != DEFAULT_CONSENSUS_REACHED_SLOT
                            );
                            assert!(ballot_box.has_winning_ballot());
                            assert!(ballot_box.is_consensus_reached());
                        }
                    }
                }
                Err(e) => match e {
                    TipRouterError::OperatorVotesFull => {
                        assert_eq!(
                            ballot_box.operators_voted() as usize,
                            MAX_OPERATORS,
                            "OperatorVotesFull error but max operators not reached"
                        );
                    }
                    TipRouterError::BallotTallyFull => {
                        assert_eq!(
                            ballot_box.unique_ballots() as usize,
                            MAX_OPERATORS,
                            "BallotTallyFull error but max tallies not reached"
                        );
                    }
                    TipRouterError::VotingNotValid => {
                        assert!(!ballot_box
                            .is_voting_valid(slot, valid_slots_after_consensus)
                            .unwrap());
                    }
                    TipRouterError::ConsensusAlreadyReached => {
                        assert!(ballot_box.is_consensus_reached());
                    }
                    _ => panic!("Unexpected error: {:?}", e),
                },
            }

            // Verify invariants
            assert!(ballot_box.operators_voted() <= MAX_OPERATORS as u64);
            assert!(ballot_box.unique_ballots() <= MAX_OPERATORS as u64);

            // Verify each ballot tally matches its vote count
            for tally in ballot_box.ballot_tallies().iter().filter(|t| t.is_valid()) {
                let vote_count = ballot_box
                    .operator_votes()
                    .iter()
                    .filter(|v| !v.is_empty() && v.ballot_index() == tally.index())
                    .count();

                assert_eq!(vote_count as u64, tally.tally());
            }
        }
    }
}

#[cfg(test)]
mod vote_change_tests {
    use super::*;

    #[test]
    fn test_ballot_box_vote_change() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Create two distinct ballots
        let ballot1 = Ballot::new(&Pubkey::new_unique().to_bytes());
        let ballot2 = Ballot::new(&Pubkey::new_unique().to_bytes());

        // Create a single operator
        let operator = Pubkey::new_unique();
        let stake_weights = StakeWeights::new(1000);

        // Cast initial vote for ballot1
        ballot_box
            .cast_vote(
                &operator,
                &ballot1,
                &stake_weights,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify initial state
        let ballot1_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot1))
            .expect("Ballot1 tally should exist");

        assert_eq!(
            ballot1_tally.tally(),
            1,
            "Initial ballot should have tally of 1"
        );

        // Change vote to ballot2
        ballot_box
            .cast_vote(
                &operator,
                &ballot2,
                &stake_weights,
                current_slot + 1,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify ballot1 tally decreased
        let ballot1_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot1));

        assert!(
            ballot1_tally.is_none() || !ballot1_tally.unwrap().is_valid(),
            "Ballot1 tally should be removed or invalid"
        );

        // Verify ballot2 tally
        let ballot2_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot2))
            .expect("Ballot2 tally should exist");

        assert_eq!(
            ballot2_tally.tally(),
            1,
            "New ballot should have tally of 1"
        );

        // Verify operator vote record
        let operator_vote = ballot_box
            .operator_votes()
            .iter()
            .find(|v| v.operator().eq(&operator))
            .expect("Operator vote should exist");

        assert_eq!(
            operator_vote.ballot_index(),
            ballot2_tally.index(),
            "Operator should be recorded as voting for ballot2"
        );

        // Verify total counts
        let total_votes: u64 = ballot_box
            .ballot_tallies()
            .iter()
            .filter(|t| t.is_valid())
            .map(|t| t.tally())
            .sum();

        assert_eq!(
            total_votes,
            ballot_box.operators_voted(),
            "Total votes should match number of operators"
        );
    }

    #[test]
    fn test_multiple_vote_changes() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        let num_operators = 5;
        let operators: Vec<Pubkey> = (0..num_operators).map(|_| Pubkey::new_unique()).collect();

        let num_ballots = 3;
        let ballots: Vec<Ballot> = (0..num_ballots)
            .map(|_| Ballot::new(&Pubkey::new_unique().to_bytes()))
            .collect();

        let stake_weights = StakeWeights::new(1000);
        let mut slot = current_slot;

        // Have each operator vote for each ballot in sequence
        for ballot in &ballots {
            for operator in &operators {
                ballot_box
                    .cast_vote(
                        operator,
                        ballot,
                        &stake_weights,
                        slot,
                        valid_slots_after_consensus,
                    )
                    .unwrap();
                slot += 1;

                // Verify counts after each vote
                for tally in ballot_box.ballot_tallies().iter().filter(|t| t.is_valid()) {
                    let vote_count = ballot_box
                        .operator_votes()
                        .iter()
                        .filter(|v| !v.is_empty() && v.ballot_index() == tally.index())
                        .count();

                    assert_eq!(
                        vote_count as u64,
                        tally.tally(),
                        "Ballot tally count mismatch. Expected {} but got {}",
                        vote_count,
                        tally.tally()
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod same_ballot_tests {
    use super::*;

    #[test]
    fn test_revote_same_ballot_different_stake() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Create ballot and operator
        let ballot = Ballot::new(&[1; 32]);
        let operator = Pubkey::new_unique();

        // Initial vote with stake 1000
        let stake_weights1 = StakeWeights::new(1000);
        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights1,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify initial state
        let ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist");
        assert_eq!(ballot_tally.tally(), 1);
        assert_eq!(ballot_tally.stake_weights().stake_weight(), 1000);

        // Vote again for same ballot with different stake
        let stake_weights2 = StakeWeights::new(2000);
        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights2,
                current_slot + 1,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify state after revote
        let ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist");

        // Should still be only 1 vote, just with updated stake
        assert_eq!(
            ballot_tally.tally(),
            1,
            "Vote count should not change when revoting for same ballot"
        );
        assert_eq!(
            ballot_tally.stake_weights().stake_weight(),
            2000,
            "Stake weight should be updated"
        );

        // Verify operator vote record
        let operator_vote = ballot_box
            .operator_votes()
            .iter()
            .find(|v| v.operator().eq(&operator))
            .expect("Operator vote should exist");
        assert_eq!(operator_vote.stake_weights().stake_weight(), 2000);

        // Verify total counts
        assert_eq!(ballot_box.operators_voted(), 1);
        assert_eq!(ballot_box.unique_ballots(), 1);
    }
}

#[cfg(test)]
mod revote_same_tests {
    use super::*;

    #[test]
    fn test_revote_same_ballot() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Create ballot and operator
        let ballot = Ballot::new(&Pubkey::new_unique().to_bytes());
        let operator = Pubkey::new_unique();
        let stake_weights = StakeWeights::new(1000);

        // Cast initial vote
        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Record state after first vote
        let initial_operators_voted = ballot_box.operators_voted();
        let initial_unique_ballots = ballot_box.unique_ballots();
        let initial_ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist")
            .clone();

        // Vote again for the same ballot
        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                current_slot + 1,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify nothing changed except the slot voted
        assert_eq!(ballot_box.operators_voted(), initial_operators_voted);
        assert_eq!(ballot_box.unique_ballots(), initial_unique_ballots);

        let final_ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist");

        assert_eq!(final_ballot_tally.tally(), initial_ballot_tally.tally());
        assert_eq!(
            final_ballot_tally.stake_weights().stake_weight(),
            initial_ballot_tally.stake_weights().stake_weight()
        );
        assert_eq!(final_ballot_tally.index(), initial_ballot_tally.index());

        // Verify operator vote record
        let operator_vote = ballot_box
            .operator_votes()
            .iter()
            .find(|v| v.operator().eq(&operator))
            .expect("Operator vote should exist");

        assert_eq!(operator_vote.ballot_index(), final_ballot_tally.index());
        assert_eq!(
            operator_vote.stake_weights().stake_weight(),
            stake_weights.stake_weight()
        );
        assert_eq!(operator_vote.slot_voted(), current_slot + 1);

        // Try voting one more time with same ballot
        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                current_slot + 2,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify still nothing changed except slot
        assert_eq!(ballot_box.operators_voted(), initial_operators_voted);
        assert_eq!(ballot_box.unique_ballots(), initial_unique_ballots);

        let final_ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist");

        assert_eq!(final_ballot_tally.tally(), initial_ballot_tally.tally());
        assert_eq!(
            final_ballot_tally.stake_weights().stake_weight(),
            initial_ballot_tally.stake_weights().stake_weight()
        );
        assert_eq!(final_ballot_tally.index(), initial_ballot_tally.index());

        let operator_vote = ballot_box
            .operator_votes()
            .iter()
            .find(|v| v.operator().eq(&operator))
            .expect("Operator vote should exist");

        assert_eq!(operator_vote.slot_voted(), current_slot + 2);
    }
}

#[cfg(test)]
mod zero_stake_tests {
    use super::*;

    #[test]
    fn test_zero_stake_operator_basic_voting() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        // Create ballots and operators
        let ballot1 = Ballot::new(&Pubkey::new_unique().to_bytes());
        let ballot2 = Ballot::new(&Pubkey::new_unique().to_bytes());

        let zero_stake_operator = Pubkey::new_unique();
        let zero_stake = StakeWeights::new(0);

        // Zero stake operator can cast a vote
        ballot_box
            .cast_vote(
                &zero_stake_operator,
                &ballot1,
                &zero_stake,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify vote was recorded
        let operator_vote = ballot_box
            .operator_votes()
            .iter()
            .find(|v| v.operator().eq(&zero_stake_operator))
            .expect("Zero stake operator vote should be recorded");

        assert_eq!(operator_vote.stake_weights().stake_weight(), 0);

        // Verify ballot tally
        let ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot1))
            .expect("Ballot tally should exist");

        assert_eq!(ballot_tally.stake_weights().stake_weight(), 0);
        assert_eq!(ballot_tally.tally(), 1);

        // Zero stake operator can change their vote
        ballot_box
            .cast_vote(
                &zero_stake_operator,
                &ballot2,
                &zero_stake,
                current_slot + 1,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify both ballot tallies
        let ballot1_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot1));
        assert!(
            ballot1_tally.is_none() || !ballot1_tally.unwrap().is_valid(),
            "First ballot tally should be removed since zero stake operator was only voter"
        );

        let ballot2_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot2))
            .expect("Second ballot tally should exist");

        assert_eq!(ballot2_tally.stake_weights().stake_weight(), 0);
        assert_eq!(ballot2_tally.tally(), 1);
    }

    #[test]
    fn test_zero_stake_operator_consensus() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        let ballot = Ballot::new(&Pubkey::new_unique().to_bytes());

        // Create multiple zero stake operators
        let num_zero_stake = 5;
        let zero_stake_operators: Vec<Pubkey> =
            (0..num_zero_stake).map(|_| Pubkey::new_unique()).collect();
        let zero_stake = StakeWeights::new(0);

        // Have all zero stake operators vote for the same ballot
        for (i, operator) in zero_stake_operators.iter().enumerate() {
            ballot_box
                .cast_vote(
                    operator,
                    &ballot,
                    &zero_stake,
                    current_slot + i as u64,
                    valid_slots_after_consensus,
                )
                .unwrap();
        }

        // Check ballot state after zero stake votes
        let ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist");

        assert_eq!(ballot_tally.stake_weights().stake_weight(), 0);
        assert_eq!(ballot_tally.tally(), num_zero_stake as u64);

        // Calculate consensus with only zero stake votes
        let total_stake = 1000u128;
        ballot_box
            .tally_votes(total_stake, current_slot + num_zero_stake as u64)
            .unwrap();
        assert!(
            !ballot_box.is_consensus_reached(),
            "Zero stake votes alone should not reach consensus"
        );

        // Add one normal stake vote
        let normal_operator = Pubkey::new_unique();
        let normal_stake = StakeWeights::new(700); // 70% of total stake

        ballot_box
            .cast_vote(
                &normal_operator,
                &ballot,
                &normal_stake,
                current_slot + num_zero_stake as u64,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify ballot tally includes both zero and normal stakes
        let ballot_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot))
            .expect("Ballot tally should exist");

        assert_eq!(
            ballot_tally.stake_weights().stake_weight(),
            normal_stake.stake_weight()
        );
        assert_eq!(ballot_tally.tally(), (num_zero_stake + 1) as u64);

        // Check consensus again
        ballot_box
            .tally_votes(total_stake, current_slot + num_zero_stake as u64 + 1)
            .unwrap();
        assert!(
            ballot_box.is_consensus_reached(),
            "Consensus should be reached with normal stake vote despite zero stake votes"
        );
    }

    #[test]
    fn test_zero_stake_operator_mixed_voting() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let valid_slots_after_consensus = 100;
        let mut ballot_box = BallotBox::new(&ncn, epoch, 0, current_slot);

        let ballot1 = Ballot::new(&Pubkey::new_unique().to_bytes());
        let ballot2 = Ballot::new(&Pubkey::new_unique().to_bytes());

        // Create mix of zero and normal stake operators
        let zero_stake_operator = Pubkey::new_unique();
        let zero_stake = StakeWeights::new(0);

        let normal_operator1 = Pubkey::new_unique();
        let normal_stake1 = StakeWeights::new(300);

        let normal_operator2 = Pubkey::new_unique();
        let normal_stake2 = StakeWeights::new(400);

        // Cast votes for ballot1
        ballot_box
            .cast_vote(
                &zero_stake_operator,
                &ballot1,
                &zero_stake,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();
        ballot_box
            .cast_vote(
                &normal_operator1,
                &ballot1,
                &normal_stake1,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Cast vote for ballot2
        ballot_box
            .cast_vote(
                &normal_operator2,
                &ballot2,
                &normal_stake2,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify ballot tallies
        let ballot1_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot1))
            .expect("Ballot1 tally should exist");

        assert_eq!(
            ballot1_tally.stake_weights().stake_weight(),
            normal_stake1.stake_weight()
        );
        assert_eq!(ballot1_tally.tally(), 2); // Counts both zero and normal stake votes

        let ballot2_tally = ballot_box
            .ballot_tallies()
            .iter()
            .find(|t| t.ballot().eq(&ballot2))
            .expect("Ballot2 tally should exist");

        assert_eq!(
            ballot2_tally.stake_weights().stake_weight(),
            normal_stake2.stake_weight()
        );
        assert_eq!(ballot2_tally.tally(), 1);

        // Check consensus
        let total_stake = 1000u128;
        ballot_box.tally_votes(total_stake, current_slot).unwrap();

        // Neither ballot should have consensus yet
        assert!(!ballot_box.is_consensus_reached());

        // Add another normal stake vote to ballot2 to reach consensus
        let normal_operator3 = Pubkey::new_unique();
        let normal_stake3 = StakeWeights::new(300);
        ballot_box
            .cast_vote(
                &normal_operator3,
                &ballot2,
                &normal_stake3,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        ballot_box.tally_votes(total_stake, current_slot).unwrap();

        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.get_winning_ballot().unwrap(), &ballot2);
    }
}
