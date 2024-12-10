# Jito MEV Tip Distribution NCN

## Testing Setup

### Prerequisites
Before running the tests, set up your environment:

Set Anchor IDL build program path:
`export ANCHOR_IDL_BUILD_PROGRAM_PATH=PATH_TO/jito-tip-router/tip-router-operator-cli`

### Running Tests
1. Set up local validators:
`./setup-local-validators.sh 3 validators.txt 200`

2. Run integration tests with full backtrace:
`RUST_BACKTRACE=full cargo test --test integration_tests`