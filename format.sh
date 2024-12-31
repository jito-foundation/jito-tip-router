#! /bin/zsh

# Function to print command being executed
print_executing() {
    echo "Executing: $1"
}

# Basic commands that always run
print_executing "cargo sort --workspace"
cargo sort --workspace

print_executing "cargo fmt --all"
cargo fmt --all

print_executing "cargo nextest run --all-features"
cargo build-sbf --sbf-out-dir integration_tests/tests/fixtures
SBF_OUT_DIR=integration_tests/tests/fixtures cargo nextest run --all-features -E 'not test(ledger_utils::tests::test_get_bank_from_ledger_success) and not test(test_meta_merkle_creation_from_ledger)'

# Code coverage only runs with flag
if [[ "$*" == *"--code-coverage"* ]]; then
    print_executing "cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info"
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info -- --skip "tip_router::bpf::set_merkle_root"
fi

print_executing "cargo clippy --all-features"
cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf

print_executing "cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b"
cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b

print_executing "cargo-build-sbf"
cargo-build-sbf