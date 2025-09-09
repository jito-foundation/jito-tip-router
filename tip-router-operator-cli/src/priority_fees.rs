use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;

pub fn configure_instruction(
    instruction: Instruction,
    compute_unit_price: u64,
    maybe_compute_unit_limit: Option<u32>,
) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    if let Some(limit) = maybe_compute_unit_limit {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
        compute_unit_price,
    ));
    instructions.push(instruction);
    instructions
}
