use solana_client::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, transaction::Transaction};

// Vector of transactions, each containing as many instructions as possible while staying under the size limit
pub fn pack_transactions(
    instructions: Vec<Instruction>,
    payer: Pubkey,
    max_transaction_size: usize,
) -> Vec<Transaction> {
    let mut transactions = vec![];
    let mut current_instructions = vec![];

    for instruction in instructions {
        // Create a temporary transaction with the new instruction to measure size
        let mut temp_instructions = current_instructions.clone();
        temp_instructions.push(instruction.clone());
        let temp_transaction = Transaction::new_with_payer(&temp_instructions, Some(&payer));
        let transaction_size = temp_transaction.message.serialize().len();

        let estimated_base64_size = (transaction_size * 4 + 2) / 3; // Ceiling division for base64

        if estimated_base64_size > max_transaction_size {
            if !current_instructions.is_empty() {
                let transaction = Transaction::new_with_payer(&current_instructions, Some(&payer));
                transactions.push(transaction);
            }

            current_instructions = vec![instruction];
        } else {
            current_instructions.push(instruction);
        }
    }

    if !current_instructions.is_empty() {
        let transaction = Transaction::new_with_payer(&current_instructions, Some(&payer));
        transactions.push(transaction);
    }

    transactions
}