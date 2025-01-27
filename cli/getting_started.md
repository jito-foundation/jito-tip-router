# Tip Router CLI

## Official Accounts

| Account                    | Address                                      |
| -------------------------- | -------------------------------------------- |
| Test Tip Router Program ID | RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb |
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

### For Operator

- initialize_operator ( operator_fee_bps )
- set_operator_admin ( voter )
( Give Jito the Operator Account Address )

- initialize_operator_vault_ticket ( for all vaults )
- warmup_operator_vault_ticket ( for all vaults )

( Wait for NCN )

- operator_warmup_ncn

### For Vault

( Wait for Operator )

- initialize_vault_operator_delegation
- add_delegation

- initialize_vault_ncn_ticket
- warmup_vault_ncn_ticket

### For NCN

- initialize_ncn

- initialize_ncn_vault_ticket
- warmup_ncn_vault_ticket

( Wait for Operators to be created )

- initialize_ncn_operator_state
- ncn_warmup_operator
