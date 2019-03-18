#!/bin/sh

set -ex

cargo build --verbose
cargo fmt --all -- --check # precondition: built grammar first
# best-effort run clippy
command -V cargo-clippy && cargo clippy --all-targets
cargo test --verbose
