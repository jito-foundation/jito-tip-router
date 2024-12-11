#! /bin/zsh
echo "Executing: cargo sort --workspace"
cargo sort --workspace

echo "Executing: cargo fmt --all"
cargo fmt --all

echo "Executing: cargo nextest run --all-features -E 'not test(bpf)'"
cargo nextest run --all-features -E 'not test(bpf)'

echo "Executing: cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf"
cargo clippy --all-features -- -D warnings -D clippy::all -D clippy::nursery -D clippy::integer_division -D clippy::arithmetic_side_effects -D clippy::style -D clippy::perf

echo "Executing: cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b"
cargo b && ./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b

echo "Executing: cargo-build-sbf"
cargo-build-sbf


