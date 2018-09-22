#!/bin/sh

set -ex

cargo build --verbose
cargo fmt --all -- --check # precondition: built grammar first
cargo clippy --all-targets
cargo test --verbose
