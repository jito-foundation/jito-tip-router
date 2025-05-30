use jito_bytemuck::Discriminator;
use jito_tip_router_core::{ballot_box::BallotBox, epoch_state::EpochState};
use solana_sdk::{account::Account, native_token::LAMPORTS_PER_SOL};

pub fn serialized_epoch_state_account(epoch_state: &EpochState) -> Account {
    // TODO add AccountSerialize to jito_restaking::bytemuck?
    let mut data = vec![EpochState::DISCRIMINATOR; 1];
    data.extend_from_slice(&[0; 7]);
    data.extend_from_slice(bytemuck::bytes_of(epoch_state));

    Account {
        lamports: LAMPORTS_PER_SOL * 5,
        data,
        owner: jito_tip_router_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}

pub fn serialized_ballot_box_account(ballot_box: &BallotBox) -> Account {
    // TODO add AccountSerialize to jito_restaking::bytemuck?
    let mut data = vec![BallotBox::DISCRIMINATOR; 1];
    data.extend_from_slice(&[0; 7]);
    data.extend_from_slice(bytemuck::bytes_of(ballot_box));

    Account {
        lamports: LAMPORTS_PER_SOL * 5,
        data,
        owner: jito_tip_router_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}
