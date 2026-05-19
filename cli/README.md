# Jito Tip Router CLI

## Overview

The Jito Tip Router CLI is a command-line tool that provides access to Jito Tip Router Program.

## Features

### Transaction Inspection

You can preview transactions before sending them using the --print-tx flag.

```bash
jito-tip-router-cli -- --print-tx admin-create-config --keypair-path <KEYPAIR_PATH> --ncn <NCN_ADDRESS>
```

Example output:

```bash
------ IX ------

RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb

4mgyW8EGjLgq2gfPapQUYE3PGDtsUpYyuHDUj3tT3K6i  W
RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb
2V6Abua9BY6Ga8HUeLWSLXh4Gm6oKsn3GpTzP4eYMFqT
2V6Abua9BY6Ga8HUeLWSLXh4Gm6oKsn3GpTzP4eYMFqT    S
2V6Abua9BY6Ga8HUeLWSLXh4Gm6oKsn3GpTzP4eYMFqT
JAAgQEBRyA5Jx6UGWsLNGicohE57cDFsZ58vT9MMDpd9  W
11111111111111111111111111111111


1M3AwPW4zJMasH4186d3fXfvLwMoZYHroPZVyZnhZR
```

When using this flag, the transaction will not be processed - only printed for inspection.
Note that instruction data shown in the output is **base58** encoded, which provides a compact text representation of binary data.

### Admin Set Tie Breaker

When voting reaches a stall (no consensus after the configured epoch window), the tie-breaker admin can manually set the winning merkle root.

The `--meta-merkle-root` accepts either a bracketed byte array or a 64-character hex string:

```bash
# byte array format
jito-tip-router-cli --ncn <NCN_ADDRESS> --keypair-path <KEYPAIR_PATH> \
  admin-set-tie-breaker \
  --meta-merkle-root "[172, 209, 53, 243, 16, 133, 81, 178, 15, 61, 0, 1, 80, 230, 29, 46, 236, 162, 155, 4, 183, 213, 241, 201, 14, 128, 161, 188, 128, 137, 0, 132]"

# hex format
jito-tip-router-cli --ncn <NCN_ADDRESS> --keypair-path <KEYPAIR_PATH> \
  admin-set-tie-breaker \
  --meta-merkle-root "acd135f3108551b20f3d00015...0e80a1bc808900..."
```

#### Using with a Squads Multisig

When the tie-breaker admin is a Squads vault rather than a local keypair, pass the vault address via `--tie-breaker-admin` and use `--print-tx` to get the base58-encoded transaction to submit through Squads:

```bash
jito-tip-router-cli --ncn <NCN_ADDRESS> --keypair-path <KEYPAIR_PATH> \
  --print-tx \
  admin-set-tie-breaker \
  --meta-merkle-root "[172, 209, ...]" \
  --tie-breaker-admin <SQUADS_VAULT_PUBKEY>
```

The accounts in the printed instruction are:

| # | Account | Flags | Description |
|---|---------|-------|-------------|
| 1 | Program ID | | Tip Router program being invoked |
| 2 | `epoch_state` | W | PDA tracking epoch progress; updated when tie-breaker fires |
| 3 | `config` | | `TipRouterConfig` PDA; used to validate the tie-breaker admin |
| 4 | `ballot_box` | W | PDA storing operator votes; winning ballot is forced here |
| 5 | `ncn` | | NCN address used to derive the other PDAs |
| 6 | `tie_breaker_admin` | S | Must match the admin stored in `config`; signs the transaction |

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
jito-tip-router-cli --help
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
