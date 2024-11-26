#! /bin/zsh
echo "Executing: ANCHOR_IDL_BUILD_SKIP_LINT=true cargo sort --workspace"
ANCHOR_IDL_BUILD_SKIP_LINT=true cargo sort --workspace

echo "Executing: ANCHOR_IDL_BUILD_SKIP_LINT=true cargo fmt --all"
ANCHOR_IDL_BUILD_SKIP_LINT=true cargo fmt --all

echo "Executing: ANCHOR_IDL_BUILD_SKIP_LINT=true cargo nextest run --all-features"
ANCHOR_IDL_BUILD_SKIP_LINT=true cargo nextest run --all-features

echo "Executing: ANCHOR_IDL_BUILD_SKIP_LINT=true cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf"
ANCHOR_IDL_BUILD_SKIP_LINT=true cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf

echo "Executing: ANCHOR_IDL_BUILD_SKIP_LINT=true cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b"
ANCHOR_IDL_BUILD_SKIP_LINT=true cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b

echo "Executing: cargo-build-sbf"
cargo-build-sbf


