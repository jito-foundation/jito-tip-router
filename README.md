# Jito MEV Tip Distribution NCN

## Testing Setup

### Prerequisites

1. Set up test-ledger: `./tip-router-operator-cli/scripts/setup-test-ledger.sh`
2. Build the tip router program: `cargo build-sbf --manifest-path program/Cargo.toml --sbf-out-dir integration_tests/tests/fixtures`
3. Run tests: `SBF_OUT_DIR=integration_tests/tests/fixtures cargo test`

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
