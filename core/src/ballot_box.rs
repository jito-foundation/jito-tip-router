use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU128, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    constants::MAX_OPERATORS, discriminators::Discriminators, error::TipRouterError, fees::Fees,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct MerkleRoot {
    root: [u8; 32],
    max_total_claim: PodU64,
    max_num_nodes: PodU64,
    reserved: [u8; 64],
}

impl Default for MerkleRoot {
    fn default() -> Self {
        Self {
            root: [0; 32],
            max_total_claim: PodU64::from(0),
            max_num_nodes: PodU64::from(0),
            reserved: [0; 64],
        }
    }
}

impl MerkleRoot {
    pub fn new(root: [u8; 32], max_total_claim: u64, max_num_nodes: u64) -> Self {
        Self {
            root,
            max_total_claim: PodU64::from(max_total_claim),
            max_num_nodes: PodU64::from(max_num_nodes),
            reserved: [0; 64],
        }
    }

    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    pub fn max_total_claim(&self) -> u64 {
        self.max_total_claim.into()
    }

    pub fn max_num_nodes(&self) -> u64 {
        self.max_num_nodes.into()
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct MerkleRootTally {
    merkle_root: MerkleRoot,
    stake_weight: PodU128,
    vote_count: PodU64,
    reserved: [u8; 64],
}

impl Default for MerkleRootTally {
    fn default() -> Self {
        Self {
            merkle_root: MerkleRoot::default(),
            stake_weight: PodU128::from(0),
            vote_count: PodU64::from(0),
            reserved: [0; 64],
        }
    }
}

impl MerkleRootTally {
    pub fn new(
        root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
        stake_weight: u128,
    ) -> Self {
        Self {
            merkle_root: MerkleRoot::new(root, max_total_claim, max_num_nodes),
            stake_weight: PodU128::from(stake_weight),
            vote_count: PodU64::from(1),
            reserved: [0; 64],
        }
    }

    pub fn merkle_root(&self) -> MerkleRoot {
        self.merkle_root
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn vote_count(&self) -> u64 {
        self.vote_count.into()
    }

    pub fn increment_tally(&mut self, stake_weight: u128) -> Result<(), TipRouterError> {
        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        self.vote_count = PodU64::from(
            self.vote_count()
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
    merkle_root: MerkleRoot,
    reserved: [u8; 64],
}

impl Default for OperatorVote {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            slot_voted: PodU64::from(0),
            stake_weight: PodU128::from(0),
            merkle_root: MerkleRoot::default(),
            reserved: [0; 64],
        }
    }
}

impl OperatorVote {
    pub fn new(
        root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
        operator: Pubkey,
        current_slot: u64,
        stake_weight: u128,
    ) -> Self {
        Self {
            operator,
            merkle_root: MerkleRoot::new(root, max_total_claim, max_num_nodes),
            slot_voted: PodU64::from(current_slot),
            stake_weight: PodU128::from(stake_weight),
            reserved: [0; 64],
        }
    }

    pub fn operator(&self) -> Pubkey {
        self.operator
    }

    pub fn slot_voted(&self) -> u64 {
        self.slot_voted.into()
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn merkle_root(&self) -> MerkleRoot {
        self.merkle_root
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
    unique_merkle_roots: PodU64,

    operator_votes: [OperatorVote; 256],
    merkle_root_tallies: [MerkleRootTally; 256],
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
            unique_merkle_roots: PodU64::from(0),
            operator_votes: [OperatorVote::default(); MAX_OPERATORS],
            merkle_root_tallies: [MerkleRootTally::default(); MAX_OPERATORS],
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

    fn insert_or_create_merkle_root_tally(
        &mut self,
        merkle_root: &MerkleRoot,
    ) -> Result<(), TipRouterError> {
        Ok(())
    }
}
