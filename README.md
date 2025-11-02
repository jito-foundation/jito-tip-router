# Jito Tip Router

## Overview

Jito Tip Router NCN will assume programmatic operations related to the distribution of MEV tips generated from the Jito Tip Distribution protocol every epoch, and, as part of these programmatic operations, will route some portion of all MEV tips collected to the DAO treasury and the NCN network participants.

## Development Setup

### Docker Services Management

This project includes a Makefile for easy management of Docker services. Use `make help` to see all available commands.

#### NCN Keeper Services

```bash
# Start services
make ncn-keeper-start          # Main keeper service
make ncn-keeper-start-metrics  # Metrics-only service  
make ncn-keeper-start-migrate  # Migration-only service
make ncn-keeper-start-all      # Start all NCN keeper services

# View logs
make ncn-keeper-logs           # Main keeper logs
make ncn-keeper-logs-metrics   # Metrics service logs
make ncn-keeper-logs-migrate   # Migration service logs

# Stop services
make ncn-keeper-stop           # Stop main keeper
make ncn-keeper-stop-metrics   # Stop metrics service
```

#### Operator Services

```bash
# Operator management
make start-operator # Start tip router operator
make logs-operator  # View operator logs
make stop-operator  # Stop operator service
```

#### Utility Commands

```bash
make help           # Show all available commands
```

### Container Images

The project uses GitHub Actions to build container images for two main services:

- **NCN Keeper**: `jito-tip-router-ncn-keeper` (built from `./cli/`)
- **Operator**: `tip-router-operator-cli` (built from `./tip-router-operator-cli/`)

## Testing Setup

### Prerequisites

1. Set up test-ledger: `./tip-router-operator-cli/scripts/setup-test-ledger.sh`
2. Build the tip router program: `cargo build-sbf --manifest-path program/Cargo.toml --sbf-out-dir integration_tests/tests/fixtures`
3. Run tests: `SBPF_OUT_DIR=integration_tests/tests/fixtures cargo nextest run --all-features -E 'not test(ledger_utils::tests)'`

To see more info on the Tip Router CLI check out the [CLI documentation](./cli/README.md)

---

## 📖 Documentation

The comprehensive documentation for Tip Router has moved to [jito.network/docs/tiprouter](https://jito.network/docs/tiprouter). The source files are maintained in the [Jito Omnidocs repository](https://github.com/jito-foundation/jito-omnidocs/tree/master/tiprouter).

To see more info on the Tip Router CLI check out the [CLI documentation](./cli/README.md)

---

## Deploy and Upgrade

- build .so file: `cargo-build-sbf`

- create a new keypair: `solana-keygen new -o target/tmp/buffer.json`

- Deploy: `solana program deploy --use-rpc --buffer target/tmp/buffer.json --with-compute-unit-price 10000 --max-sign-attempts 10000 target/deploy/jito_tip_router_program.so`

- (Pre Upgrade) Write to buffer: `solana program write-buffer --use-rpc --buffer target/tmp/buffer.json --with-compute-unit-price 10000 --max-sign-attempts 10000 target/deploy/jito_tip_router_program.so`

- Upgrade: `solana program upgrade $(solana address --keypair target/tmp/buffer.json) $(solana address --keypair target/deploy/jito_tip_router_program-keypair.json)`

- Close Buffers: `solana program close --buffers`

- Upgrade Program Size: `solana program extend $(solana address --keypair target/deploy/jito_tip_router_program-keypair.json) 100000`

## Security Audits

| Group    | Date       | Commit                                                                 |
|----------|------------|------------------------------------------------------------------------|
| Certora  | 2025-01-05 | [ac76352](security_audits/certora.pdf)                                 |
| Offside  | 2024-10-25 | [443368a](security_audits/offside.pdf)                                 |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
