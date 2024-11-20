use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU128, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{constants::PRECISE_CONSENSUS, discriminators::Discriminators, error::TipRouterError};

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

    ncn_epoch: PodU64,

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
    pub fn new(ncn: Pubkey, ncn_epoch: u64, bump: u8, current_slot: u64) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(current_slot),
            slot_consensus_reached: PodU64::from(0),
            operators_voted: PodU64::from(0),
            unique_ballots: PodU64::from(0),
            winning_ballot: Ballot::default(),
            //TODO fix 32 -> MAX_OPERATORS
            operator_votes: [OperatorVote::default(); 32],
            ballot_tallies: [BallotTally::default(); 32],
            reserved: [0; 128],
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"ballot_box".to_vec(),
                ncn.to_bytes().to_vec(),
                ncn_epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, ncn_epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        epoch_snapshot: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if epoch_snapshot.owner.ne(program_id) {
            msg!("Ballot box account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if epoch_snapshot.data_is_empty() {
            msg!("Ballot box account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !epoch_snapshot.is_writable {
            msg!("Ballot box account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if epoch_snapshot.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Ballot box account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if epoch_snapshot
            .key
            .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
        {
            msg!("Ballot box account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
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
        self.slot_consensus_reached() > 0
    }

    pub fn get_winning_ballot(&self) -> Result<Ballot, TipRouterError> {
        if self.winning_ballot.is_valid() {
            Ok(self.winning_ballot)
        } else {
            Err(TipRouterError::ConsensusNotReached)
        }
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
    ) -> Result<(), TipRouterError> {
        let ballot_index = self.increment_or_create_ballot_tally(&ballot, stake_weight)?;

        for vote in self.operator_votes.iter_mut() {
            if vote.operator().eq(&operator) {
                return Err(TipRouterError::DuplicateVoteCast);
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
        if self.slot_consensus_reached() != 0 {
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

        let target_precise_percentage =
            PreciseNumber::new(PRECISE_CONSENSUS).ok_or(TipRouterError::NewPreciseNumberError)?;

        let consensus_reached =
            ballot_percentage_of_total.greater_than_or_equal(&target_precise_percentage);

        if consensus_reached {
            self.slot_consensus_reached = PodU64::from(current_slot);

            self.winning_ballot = max_tally.ballot();
        }

        Ok(())
    }
}
