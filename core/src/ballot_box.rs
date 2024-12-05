use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU128, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use meta_merkle_tree::{meta_merkle_tree::LEAF_PREFIX, tree_node::TreeNode};
use shank::{ShankAccount, ShankType};
use solana_program::{
    account_info::AccountInfo, hash::hashv, msg, program_error::ProgramError, pubkey::Pubkey,
};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::{precise_consensus, DEFAULT_CONSENSUS_REACHED_SLOT},
    discriminators::Discriminators,
    error::TipRouterError,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct Ballot {
    merkle_root: [u8; 32],
    reserved: [u8; 64],
}

impl Default for Ballot {
    fn default() -> Self {
        Self {
            merkle_root: [0; 32],
            reserved: [0; 64],
        }
    }
}

impl std::fmt::Display for Ballot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.merkle_root)
    }
}

impl Ballot {
    pub const fn new(root: [u8; 32]) -> Self {
        Self {
            merkle_root: root,
            reserved: [0; 64],
        }
    }

    pub const fn root(&self) -> [u8; 32] {
        self.merkle_root
    }

    pub fn is_valid(&self) -> bool {
        self.merkle_root.iter().any(|&b| b != 0)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct BallotTally {
    ballot: Ballot,
    stake_weight: PodU128,
    tally: PodU64,
    reserved: [u8; 64],
}

impl Default for BallotTally {
    fn default() -> Self {
        Self {
            ballot: Ballot::default(),
            stake_weight: PodU128::from(0),
            tally: PodU64::from(0),
            reserved: [0; 64],
        }
    }
}

impl BallotTally {
    pub fn new(ballot: Ballot, stake_weight: u128) -> Self {
        Self {
            ballot,
            stake_weight: PodU128::from(stake_weight),
            tally: PodU64::from(1),
            reserved: [0; 64],
        }
    }

    pub const fn ballot(&self) -> Ballot {
        self.ballot
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn tally(&self) -> u64 {
        self.tally.into()
    }

    pub fn is_empty(&self) -> bool {
        self.stake_weight() == 0
    }

    pub fn increment_tally(&mut self, stake_weight: u128) -> Result<(), TipRouterError> {
        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        self.tally = PodU64::from(
            self.tally()
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct OperatorVote {
    operator: Pubkey,
    slot_voted: PodU64,
    stake_weight: PodU128,
    ballot_index: PodU16,
    reserved: [u8; 64],
}

impl Default for OperatorVote {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            slot_voted: PodU64::from(0),
            stake_weight: PodU128::from(0),
            ballot_index: PodU16::from(0),
            reserved: [0; 64],
        }
    }
}

impl OperatorVote {
    pub fn new(
        ballot_index: usize,
        operator: Pubkey,
        current_slot: u64,
        stake_weight: u128,
    ) -> Self {
        Self {
            operator,
            ballot_index: PodU16::from(ballot_index as u16),
            slot_voted: PodU64::from(current_slot),
            stake_weight: PodU128::from(stake_weight),
            reserved: [0; 64],
        }
    }

    pub const fn operator(&self) -> Pubkey {
        self.operator
    }

    pub fn slot_voted(&self) -> u64 {
        self.slot_voted.into()
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn ballot_index(&self) -> u16 {
        self.ballot_index.into()
    }

    pub fn is_empty(&self) -> bool {
        self.stake_weight() == 0
    }
}

// PDA'd ["epoch_snapshot", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct BallotBox {
    ncn: Pubkey,

    epoch: PodU64,

    bump: u8,

    slot_created: PodU64,
    slot_consensus_reached: PodU64,

    reserved: [u8; 128],

    operators_voted: PodU64,
    unique_ballots: PodU64,

    winning_ballot: Ballot,

    //TODO fix 32 -> MAX_OPERATORS
    operator_votes: [OperatorVote; 32],
    ballot_tallies: [BallotTally; 32],
}

impl Discriminator for BallotBox {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl BallotBox {
    pub fn new(ncn: Pubkey, epoch: u64, bump: u8, current_slot: u64) -> Self {
        Self {
            ncn,
            epoch: PodU64::from(epoch),
            bump,
            slot_created: PodU64::from(current_slot),
            slot_consensus_reached: PodU64::from(DEFAULT_CONSENSUS_REACHED_SLOT),
            operators_voted: PodU64::from(0),
            unique_ballots: PodU64::from(0),
            winning_ballot: Ballot::default(),
            //TODO fix 32 -> MAX_OPERATORS
            operator_votes: [OperatorVote::default(); 32],
            ballot_tallies: [BallotTally::default(); 32],
            reserved: [0; 128],
        }
    }

    pub fn initialize(&mut self, ncn: Pubkey, epoch: u64, bump: u8, current_slot: u64) {
        *self = Self::new(ncn, epoch, bump, current_slot);
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
        ncn: &Pubkey,
        epoch: u64,
        ballot_box_account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if ballot_box_account.owner.ne(program_id) {
            msg!("Ballot box account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if ballot_box_account.data_is_empty() {
            msg!("Ballot box account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !ballot_box_account.is_writable {
            msg!("Ballot box account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if ballot_box_account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Ballot box account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if ballot_box_account
            .key
            .ne(&Self::find_program_address(program_id, ncn, epoch).0)
        {
            msg!("Ballot box account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
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

    pub fn is_consensus_reached(&self) -> bool {
        self.slot_consensus_reached() != DEFAULT_CONSENSUS_REACHED_SLOT
            || self.winning_ballot.is_valid()
    }

    pub fn tie_breaker_set(&self) -> bool {
        self.slot_consensus_reached() == DEFAULT_CONSENSUS_REACHED_SLOT
            && self.winning_ballot.is_valid()
    }

    pub fn get_winning_ballot(&self) -> Result<Ballot, TipRouterError> {
        if self.winning_ballot.is_valid() {
            Ok(self.winning_ballot)
        } else {
            Err(TipRouterError::ConsensusNotReached)
        }
    }

    pub fn set_winning_ballot(&mut self, ballot: Ballot) {
        self.winning_ballot = ballot;
    }

    fn increment_or_create_ballot_tally(
        &mut self,
        ballot: &Ballot,
        stake_weight: u128,
    ) -> Result<usize, TipRouterError> {
        let mut tally_index: usize = 0;
        for tally in self.ballot_tallies.iter_mut() {
            if tally.ballot.eq(ballot) {
                tally.increment_tally(stake_weight)?;
                return Ok(tally_index);
            }

            if tally.is_empty() {
                *tally = BallotTally::new(*ballot, stake_weight);

                self.unique_ballots = PodU64::from(
                    self.unique_ballots()
                        .checked_add(1)
                        .ok_or(TipRouterError::ArithmeticOverflow)?,
                );

                return Ok(tally_index);
            }

            tally_index = tally_index
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?;
        }

        Err(TipRouterError::BallotTallyFull)
    }

    pub fn cast_vote(
        &mut self,
        operator: Pubkey,
        ballot: Ballot,
        stake_weight: u128,
        current_slot: u64,
        valid_slots_after_consensus: u64,
    ) -> Result<(), TipRouterError> {
        if !self.is_voting_valid(current_slot, valid_slots_after_consensus)? {
            return Err(TipRouterError::VotingNotValid);
        }

        let ballot_index = self.increment_or_create_ballot_tally(&ballot, stake_weight)?;

        let consensus_reached = self.is_consensus_reached();

        for vote in self.operator_votes.iter_mut() {
            if vote.operator().eq(&operator) {
                if consensus_reached {
                    return Err(TipRouterError::ConsensusAlreadyReached);
                }

                let operator_vote =
                    OperatorVote::new(ballot_index, operator, current_slot, stake_weight);
                *vote = operator_vote;
                return Ok(());
            }

            if vote.is_empty() {
                let operator_vote =
                    OperatorVote::new(ballot_index, operator, current_slot, stake_weight);
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
            .max_by_key(|t| t.stake_weight())
            .unwrap();

        let ballot_stake_weight = max_tally.stake_weight();
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

            self.set_winning_ballot(max_tally.ballot());
        }

        Ok(())
    }

    pub fn set_tie_breaker_ballot(
        &mut self,
        meta_merkle_root: [u8; 32],
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

        // Check that the merkle root is one of the existing options
        if !self.has_ballot(&finalized_ballot) {
            return Err(TipRouterError::TieBreakerNotInPriorVotes);
        }

        self.set_winning_ballot(finalized_ballot);
        Ok(())
    }

    pub fn has_ballot(&self, ballot: &Ballot) -> bool {
        self.ballot_tallies.iter().any(|t| t.ballot.eq(ballot))
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
        tip_distribution_account: Pubkey,
        proof: Vec<[u8; 32]>,
        merkle_root: [u8; 32],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // TODO?
    fn test_verify_merkle_root() {
        // Create merkle tree of merkle trees

        // Intialize ballot box
        let ballot_box = BallotBox::new(Pubkey::default(), 0, 0, 0);

        // Set winning merkle root, don't care about anything else
        ballot_box
            .verify_merkle_root(Pubkey::default(), vec![], [0u8; 32], 0, 0)
            .unwrap();
    }

    #[test]
    fn test_cast_vote() {
        let ncn = Pubkey::new_unique();
        let operator = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let stake_weight: u128 = 1000;
        let valid_slots_after_consensus = 10;
        let mut ballot_box = BallotBox::new(ncn, epoch, 0, current_slot);
        let ballot = Ballot::new([1; 32]);

        // Test initial cast vote
        ballot_box
            .cast_vote(
                operator,
                ballot,
                stake_weight,
                current_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify vote was recorded correctly
        let operator_vote = ballot_box
            .operator_votes
            .iter()
            .find(|v| v.operator() == operator)
            .unwrap();
        assert_eq!(operator_vote.stake_weight(), stake_weight);
        assert_eq!(operator_vote.slot_voted(), current_slot);

        // Verify ballot tally
        let tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot() == ballot)
            .unwrap();
        assert_eq!(tally.stake_weight(), stake_weight);

        // Test re-vote with different ballot
        let new_ballot = Ballot::new([2u8; 32]);
        let new_slot = current_slot + 1;
        ballot_box
            .cast_vote(
                operator,
                new_ballot,
                stake_weight,
                new_slot,
                valid_slots_after_consensus,
            )
            .unwrap();

        // Verify new ballot tally increased
        let new_tally = ballot_box
            .ballot_tallies
            .iter()
            .find(|t| t.ballot() == new_ballot)
            .unwrap();
        assert_eq!(new_tally.stake_weight(), stake_weight);

        // Test error on changing vote after consensus
        ballot_box.set_winning_ballot(new_ballot);
        ballot_box.slot_consensus_reached = PodU64::from(new_slot);
        let result = ballot_box.cast_vote(
            operator,
            ballot,
            stake_weight,
            new_slot + 1,
            valid_slots_after_consensus,
        );
        assert!(matches!(
            result,
            Err(TipRouterError::ConsensusAlreadyReached)
        ));

        // Test voting window expired after consensus
        let result = ballot_box.cast_vote(
            operator,
            ballot,
            stake_weight,
            new_slot + valid_slots_after_consensus + 1,
            valid_slots_after_consensus,
        );
        assert!(matches!(result, Err(TipRouterError::VotingNotValid)));
    }

    #[test]
    fn test_increment_or_create_ballot_tally() {
        let mut ballot_box = BallotBox::new(Pubkey::new_unique(), 1, 1, 1);
        let ballot = Ballot::new([1u8; 32]);
        let stake_weight = 100;

        // Test creating new ballot tally
        let tally_index = ballot_box
            .increment_or_create_ballot_tally(&ballot, stake_weight)
            .unwrap();
        assert_eq!(tally_index, 0);
        assert_eq!(ballot_box.unique_ballots(), 1);
        assert_eq!(ballot_box.ballot_tallies[0].stake_weight(), stake_weight);
        assert_eq!(ballot_box.ballot_tallies[0].ballot(), ballot);

        // Test incrementing existing ballot tally
        let tally_index = ballot_box
            .increment_or_create_ballot_tally(&ballot, stake_weight)
            .unwrap();
        assert_eq!(tally_index, 0);
        assert_eq!(ballot_box.unique_ballots(), 1);
        assert_eq!(
            ballot_box.ballot_tallies[0].stake_weight(),
            stake_weight * 2
        );
        assert_eq!(ballot_box.ballot_tallies[0].ballot(), ballot);

        // Test creating second ballot tally
        let ballot2 = Ballot::new([2u8; 32]);
        let tally_index = ballot_box
            .increment_or_create_ballot_tally(&ballot2, stake_weight)
            .unwrap();
        assert_eq!(tally_index, 1);
        assert_eq!(ballot_box.unique_ballots(), 2);
        assert_eq!(ballot_box.ballot_tallies[1].stake_weight(), stake_weight);
        assert_eq!(ballot_box.ballot_tallies[1].ballot(), ballot2);

        // Test error when ballot tallies are full
        for i in 3..=32 {
            let ballot = Ballot::new([i as u8; 32]);
            ballot_box
                .increment_or_create_ballot_tally(&ballot, stake_weight)
                .unwrap();
        }
        let ballot_full = Ballot::new([33u8; 32]);
        let result = ballot_box.increment_or_create_ballot_tally(&ballot_full, stake_weight);
        assert!(matches!(result, Err(TipRouterError::BallotTallyFull)));
    }

    #[test]
    fn test_tally_votes() {
        let ncn = Pubkey::new_unique();
        let current_slot = 100;
        let epoch = 1;
        let stake_weight: u128 = 1000;
        let total_stake_weight: u128 = 1000;
        let mut ballot_box = BallotBox::new(ncn, epoch, 0, current_slot);
        let ballot = Ballot::new([1; 32]);

        // Test no consensus when below threshold
        ballot_box
            .increment_or_create_ballot_tally(&ballot, stake_weight / 2)
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
            ballot_box.get_winning_ballot(),
            Err(TipRouterError::ConsensusNotReached)
        ));

        // Test consensus reached when above threshold
        ballot_box
            .increment_or_create_ballot_tally(&ballot, stake_weight / 2)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.slot_consensus_reached(), current_slot);
        assert_eq!(ballot_box.get_winning_ballot().unwrap(), ballot);

        // Consensus remains after additional votes
        let ballot2 = Ballot::new([2; 32]);
        ballot_box
            .increment_or_create_ballot_tally(&ballot2, stake_weight)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot + 1)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.slot_consensus_reached(), current_slot);
        assert_eq!(ballot_box.get_winning_ballot().unwrap(), ballot);

        // Test with multiple competing ballots
        let mut ballot_box = BallotBox::new(ncn, epoch, 0, current_slot);
        let ballot1 = Ballot::new([1; 32]);
        let ballot2 = Ballot::new([2; 32]);
        let ballot3 = Ballot::new([3; 32]);

        ballot_box
            .increment_or_create_ballot_tally(&ballot1, stake_weight / 4)
            .unwrap();
        ballot_box
            .increment_or_create_ballot_tally(&ballot2, stake_weight / 4)
            .unwrap();
        ballot_box
            .increment_or_create_ballot_tally(&ballot3, stake_weight / 2)
            .unwrap();

        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(!ballot_box.is_consensus_reached());

        // Add more votes to reach consensus
        ballot_box
            .increment_or_create_ballot_tally(&ballot3, stake_weight / 2)
            .unwrap();
        ballot_box
            .tally_votes(total_stake_weight, current_slot)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.get_winning_ballot().unwrap(), ballot3);
    }

    #[test]
    fn test_set_tie_breaker_ballot() {
        let ncn = Pubkey::new_unique();
        let epoch = 0;
        let current_slot = 1000;
        let mut ballot_box = BallotBox::new(ncn, epoch, 0, current_slot);

        // Create some initial ballots
        let ballot1 = Ballot::new([1; 32]);
        let ballot2 = Ballot::new([2; 32]);
        let stake_weight = 100;

        ballot_box
            .increment_or_create_ballot_tally(&ballot1, stake_weight)
            .unwrap();
        ballot_box
            .increment_or_create_ballot_tally(&ballot2, stake_weight)
            .unwrap();

        // Test setting tie breaker before voting is stalled
        let current_epoch = epoch + 1;
        let epochs_before_stall = 3;
        assert_eq!(
            ballot_box.set_tie_breaker_ballot(ballot1.root(), current_epoch, epochs_before_stall),
            Err(TipRouterError::VotingNotFinalized)
        );

        // Test setting tie breaker after voting is stalled (current_epoch >= epoch + epochs_before_stall)
        let current_epoch = epoch + epochs_before_stall;
        ballot_box
            .set_tie_breaker_ballot(ballot1.root(), current_epoch, epochs_before_stall)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(ballot_box.get_winning_ballot().unwrap(), ballot1);

        // Test setting tie breaker with invalid merkle root
        let mut ballot_box = BallotBox::new(ncn, epoch, 0, current_slot);
        ballot_box
            .increment_or_create_ballot_tally(&ballot1, stake_weight)
            .unwrap();
        assert_eq!(
            ballot_box.set_tie_breaker_ballot([99; 32], current_epoch, epochs_before_stall),
            Err(TipRouterError::TieBreakerNotInPriorVotes)
        );

        // Test setting tie breaker when consensus already reached
        let mut ballot_box = BallotBox::new(ncn, epoch, 0, current_slot);
        ballot_box
            .increment_or_create_ballot_tally(&ballot1, stake_weight * 2)
            .unwrap();
        ballot_box
            .tally_votes(stake_weight * 2, current_slot)
            .unwrap();
        assert!(ballot_box.is_consensus_reached());
        assert_eq!(
            ballot_box.set_tie_breaker_ballot(ballot1.root(), current_epoch, epochs_before_stall),
            Err(TipRouterError::ConsensusAlreadyReached)
        );
    }
}
