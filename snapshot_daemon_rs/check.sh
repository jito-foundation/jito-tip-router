#!/usr/bin/env bash
set -euo pipefail

export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib
export DYLD_LIBRARY_PATH=/Library/Developer/CommandLineTools/usr/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}

cargo clippy \
  --all-features \
  --workspace \
  --fix --allow-dirty \
  -- \
  -D warnings \
  -D clippy::all \
  -D clippy::nursery \
  -D clippy::integer_division \
  -D clippy::arithmetic_side_effects \
  -D clippy::style \
  -D clippy::perf

cargo check
cargo clippy --fix --allow-dirty
cargo +nightly fmt
