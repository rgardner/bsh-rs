#!/bin/bash
#
# Builds and run tests for bsh.

# Exit immediately if a command fails
# Print commands and their arguments
set -ex

cargo build --verbose

# precondition: built grammar first via cargo build
cargo fmt --all -- --check

# best-effort run clippy
if [ -n {CLIPPY_NIGHTLY_INSTALLED} ]; then
  cargo clippy --all-targets
fi

cargo test --verbose
