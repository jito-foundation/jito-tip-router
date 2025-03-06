use solana_sdk::{bs58, instruction::Instruction};

pub fn print_base58_tx(ixs: &[Instruction]) {
    ixs.iter().for_each(|ix| {
        println!("\n------ IX ------\n");

        println!("{}\n", ix.program_id);

        ix.accounts.iter().for_each(|account| {
            let pubkey = format!("{}", account.pubkey);
            let writable = if account.is_writable { "W" } else { "" };
            let signer = if account.is_signer { "S" } else { "" };

            println!("{:<44} {:>2} {:>1}", pubkey, writable, signer);
        });

        println!("\n");

        let base58_string = bs58::encode(&ix.data).into_string();
        println!("{}\n", base58_string);
    });
}
