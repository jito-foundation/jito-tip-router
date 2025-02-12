use jito_restaking_core::ncn::Ncn;
use jito_tip_distribution_sdk::{instruction::claim_ix, jito_tip_distribution};
use jito_tip_router_core::{account_payer::AccountPayer, config::Config};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

pub fn process_claim_with_payer(
    program_id: &Pubkey,
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

    if tip_distribution_program.key.ne(&jito_tip_distribution::ID) {
        msg!("Incorrect tip distribution program");
        return Err(ProgramError::InvalidAccountData);
    }

    //NOTE: The Config will be signing as the merkle_root_upload_authority when the tip distribution program is upgraded

    // let (_, config_bump, mut config_seeds) =
    //     AccountPayer::find_program_address(program_id, &jito_tip_distribution::ID);
    // config_seeds.push(vec![config_bump]);
    let (_, account_payer_bump, mut account_payer_seeds) =
        AccountPayer::find_program_address(program_id, ncn.key);
    account_payer_seeds.push(vec![account_payer_bump]);

    // Invoke the claim instruction with our program as the payer
    invoke_signed(
        &claim_ix(
            *tip_distribution_config.key,
            *tip_distribution_account.key,
            *claim_status.key,
            *claimant.key,
            *account_payer.key,
            *system_program.key,
            proof,
            amount,
            bump,
        ),
        &[
            tip_distribution_config.clone(),
            tip_distribution_account.clone(),
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
            //NOTE: The Config will be signing as the merkle_root_upload_authority when the tip distribution program is upgraded

            // config_seeds
            //     .iter()
            //     .map(|s| s.as_slice())
            //     .collect::<Vec<&[u8]>>()
            //     .as_slice(),
        ],
    )?;

    Ok(())
}
