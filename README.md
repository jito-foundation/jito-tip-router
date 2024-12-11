# Jito MEV Tip Distribution NCN

## Testing Setup

### Prerequisites
Before running the tests, set up your environment:

Set Anchor IDL build program path:
`export ANCHOR_IDL_BUILD_PROGRAM_PATH=$PWD/tip-router-operator-cli`

### Running Tests
1. Set up local validators:
`cd tip-router-operator-cli/scripts`
`./setup-local-validators.sh 3 validators.txt 200`

2. Run integration tests with full backtrace:
`cd tip-router-operator-cli`
`RUST_BACKTRACE=full SBF_OUT_DIR=target/deploy/ cargo test --test integration_tests`