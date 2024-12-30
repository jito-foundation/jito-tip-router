#! /bin/zsh
echo "Executing: cargo sort --workspace"
cargo +nightly-2024-07-25 sort --workspace

echo "Executing: cargo fmt --all"
cargo +nightly-2024-07-25 fmt --all

echo "Executing: cargo nextest run --all-features"
cargo build-sbf --sbf-out-dir integration_tests/tests/fixtures; SBF_OUT_DIR=integration_tests/tests/fixtures cargo nextest run --all-features -E 'not test(ledger_utils::tests::test_get_bank_from_ledger_success) and not test(test_meta_merkle_creation_from_ledger)'

echo "Executing: clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf"
cargo +nightly-2024-07-25 clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf

echo "Executing: cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b"
cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b

echo "Executing: cargo-build-sbf"å
cargo-build-sbf


