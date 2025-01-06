# Jito MEV Tip Distribution NCN

## Testing Setup

### Prerequisites

1. Set up test-ledger: `./tip-router-operator-cli/scripts/setup-test-ledger.sh`

   - NOTE: This script fails on the edge version of Solana. Currently it's being ran
     with `1.18.26`. `sh -c "$(curl -sSfL https://release.anza.xyz/v1.18.26/install)"`

2. Build the tip router program: `cargo build-sbf --manifest-path program/Cargo.toml --sbf-out-dir integration_tests/tests/fixtures`

   - NOTE: Given the current state of Cargo.lock, you must use a version of cargo-build-sbf that
     has a rust toolchain higher than 1.74.0. For now, switch to the edge version to build this.
     `sh -c "$(curl -sSfL https://release.anza.xyz/v2.2.0/install)"`. Another option would be to
     manually set the Cargo.lock file version to 3 instead of 4 (<https://github.com/coral-xyz/anchor/issues/3392#issuecomment-2508412018>)

3. Run tests: `SBF_OUT_DIR=integration_tests/tests/fixtures cargo test`
   - NOTE: If you are still on the edge version of Solana CLI probably best to switch back to
     `1.18.26`

## Deploy and Upgrade

- build .so file: `cargo-build-sbf`

- create a new keypair: `solana-keygen new -o target/tmp/buffer.json`

- Deploy: `solana program deploy --use-rpc --buffer target/tmp/buffer.json --url $(solana config get | grep "RPC URL" | awk '{print $3}') --with-compute-unit-price 10000 --max-sign-attempts 10000 target/deploy/jito_tip_router_program.so --keypair $(solana config get | grep "Keypair Path" | awk '{print $3}')`

- Write to buffer: `solana program write-buffer --use-rpc --buffer target/tmp/buffer.json --url $(solana config get | grep "RPC URL" | awk '{print $3}') --with-compute-unit-price 10000 --max-sign-attempts 10000 target/deploy/jito_tip_router_program.so --keypair $(solana config get | grep "Keypair Path" | awk '{print $3}')`

- Upgrade: `solana program upgrade $(solana address --keypair target/tmp/buffer.json) $(solana address --keypair target/deploy/jito_tip_router_program-keypair.json) --keypair $(solana config get | grep "Keypair Path" | awk '{print $3}') --url $(solana config get | grep "RPC URL" | awk '{print $3}')`

- Close Buffers: `solana program close --buffers --keypair $(solana config get | grep "Keypair Path" | awk '{print $3}')`

- Upgrade Program Size: `solana program extend $(solana address --keypair target/deploy/jito_tip_router_program-keypair.json) 1000000 --keypair $(solana config get | grep "Keypair Path" | awk '{print $3}') --url $(solana config get | grep "RPC URL" | awk '{print $3}')`
