#! /bin/zsh

cargo b
./target/debug/jito-tip-router-shank-cli && yarn install && yarn generate-clients && cargo b
cargo-build-sbf
cargo fmt
