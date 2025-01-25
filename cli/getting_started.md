# Tip Router CLI

## Official Accounts

| Account                    | Address                                      |
| -------------------------- | -------------------------------------------- |
| Test Tip Router Program ID | Ap2AH3VcZGuuauEDq87uhgjNoUKcCAafc4DTyTByLMFf |
| Test NCN                   | rYQFkFYXuDqJPoH2FvFtZTC8oC3CntgRjtNatx6q1z1  |

## Setup CLIs

Install the Tip Router CLI

```bash
cargo build --release
cargo install --path ./cli --bin jito-tip-router-cli --locked
```

Ensure it has been installed

```bash
jito-tip-router-cli -- help
```

Clone and Install the Restaking and Vault CLI in a different directory

```bash
cd ..
git clone https://github.com/jito-foundation/restaking.git
cd restaking
cargo build --release
cargo install --path ./cli --bin jito-restaking-cli
```

Ensure it works

```bash
jito-restaking-cli --help
```

## Registering Network

### [VAULT] Create

Create vault:

- <TOKEN_MINT>: The Supported Token (ST) mint (e.g. JitoSOL, JTO)
- <DEPOSIT_FEE_BPS>: Deposit fee in basis points
- <WITHDRAWAL_FEE_BPS>: Withdrawal fee in basis points
- <REWARD_FEE_BPS>: Reward fee in basis points
- <DECIMALS>: Decimals of the VRT to be created (standard is 9)
- <INITIALIZE_TOKEN_AMOUNT>: Initial amount of tokens to be deposited and "burned" this is in the smallest unit of the token.

```bash
jito-restaking-cli vault vault initialize <TOKEN_MINT> <DEPOSIT_FEE_BPS> <WITHDRAWAL_FEE_BPS> <REWARD_FEE_BPS> <DECIMALS> <INITIALIZE_TOKEN_AMOUNT>
```

### [VAULT] Register to Tip Router

Create vault-ncn-ticket:

```bash
jito-restaking-cli vault vault initialize <TOKEN_MINT> <DEPOSIT_FEE_BPS> <WITHDRAWAL_FEE_BPS> <REWARD_FEE_BPS> <DECIMALS> <INITIALIZE_TOKEN_AMOUNT>
```

- Warmup vault-ncn-ticket

### [Vault] Delegate Operator

( after the operator_vault_ticket has been created )

- Create vault-operator-delegation
- Delegate to vault

### [Operator] Create

- Create operator

### [Operator] Register to Tip Router

- operator_warmup_ncn
- operator_cooldown_ncn
- operator_set_admin ( payer )

### [Operator] Register Vault

- Create vault_operator_ticket
- Warmup vault_operator_ticket

### [NCN] Create

- Create ncn

### [NCN] Register Operator

- Create ncn_operator_state
- ncn_warmup_operator

### [NCN] Register Vault

- Create ncn_vault_ticket
- Warmup ncn_vault_ticket

## For Operator

- initialize_operator ( operator_fee_bps )
- set_operator_admin ( voter )
( Give Jito the Operator Account Address )

- initialize_operator_vault_ticket ( for all vaults )
- warmup_operator_vault_ticket ( for all vaults )

( Wait for NCN )

- operator_warmup_ncn

## For Vault

( Wait for Operator )

- initialize_vault_operator_delegation
- add_delegation

- initialize_vault_ncn_ticket
- warmup_vault_ncn_ticket

## For NCN

- initialize_ncn

- initialize_ncn_vault_ticket
- warmup_ncn_vault_ticket

( Wait for Operators to be created )

- initialize_ncn_operator_state
- ncn_warmup_operator
