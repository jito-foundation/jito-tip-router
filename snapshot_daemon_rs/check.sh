#!/usr/bin/env bash
set -euo pipefail

export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib
export DYLD_LIBRARY_PATH=/Library/Developer/CommandLineTools/usr/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}

cargo check
cargo clippy --fix --allow-dirty
cargo +nightly fmt
