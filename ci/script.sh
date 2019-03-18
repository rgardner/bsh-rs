#!/bin/sh

set -ex

cargo build --verbose
cargo fmt --all -- --check # precondition: built grammar first
# best-effort run clippy
[[ -n {CLIPPY_NIGHTLY_INSTALLED} ]] && cargo clippy --all-targets
cargo test --verbose
