# Jito MEV Tip Distribution NCN

## Testing Setup

### Prerequisites
1. Set up test-ledger: `./tip-router-operator-cli/scripts/setup-test-ledger.sh `

2. Build the tip router program: `cargo build-sbf -- -p jito-tip-router-program`

3. Copy the program to fixtures: `cp target/deploy/jito_tip_router_program.so integration_tests/tests/fixtures`

3. Run tests: `SBF_OUT_DIR=integration_tests/fixtures cargo test`