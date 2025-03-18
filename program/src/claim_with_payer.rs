use jito_priority_fee_distribution_sdk::instruction::claim_ix as priority_fee_distribution_claim_ix;
use jito_restaking_core::ncn::Ncn;
use jito_tip_distribution_sdk::{instruction::claim_ix, jito_tip_distribution};
use jito_tip_router_core::{account_payer::AccountPayer, config::Config};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

pub fn _process_claim_with_payer(
    program_id: &Pubkey,
    expected_distribution_program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> ProgramResult {
    let [account_payer, config, ncn, tip_distribution_config, tip_distribution_account, claim_status, claimant, tip_distribution_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify claim status address
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Config::load(program_id, config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    if tip_distribution_program.key.ne(expected_distribution_program_id) {
        msg!("Incorrect tip distribution program");
        return Err(ProgramError::InvalidAccountData);
    }

    let (_, config_bump, mut config_seeds) = Config::find_program_address(program_id, ncn.key);
    config_seeds.push(vec![config_bump]);
    let (_, account_payer_bump, mut account_payer_seeds) =
        AccountPayer::find_program_address(program_id, ncn.key);
    account_payer_seeds.push(vec![account_payer_bump]);

    let ix = if expected_distribution_program_id.eq(&jito_tip_distribution::ID) {
        claim_ix(
            *tip_distribution_config.key,
            *tip_distribution_account.key,
            *config.key,
            *claim_status.key,
            *claimant.key,
            *account_payer.key,
            *system_program.key,
            proof,
            amount,
            bump,
        )
    } else {
        priority_fee_distribution_claim_ix(
            *tip_distribution_config.key,
            *tip_distribution_account.key,
            *config.key,
            *claim_status.key,
            *claimant.key,
            *account_payer.key,
            *system_program.key,
            proof,
            amount,
            bump,
        )
    };

    // Invoke the claim instruction with our program as the payer
    invoke_signed(
        &ix,
        &[
            tip_distribution_config.clone(),
            tip_distribution_account.clone(),
            config.clone(),
            claim_status.clone(),
            claimant.clone(),
            account_payer.clone(),
            system_program.clone(),
        ],
        &[
            account_payer_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice(),
            config_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice(),
        ],
    )?;

    Ok(())
}

pub fn process_claim_with_payer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> ProgramResult {
    _process_claim_with_payer(
        program_id,
        &jito_tip_distribution::ID,
        accounts,
        proof,
        amount,
        bump,
    )
}
