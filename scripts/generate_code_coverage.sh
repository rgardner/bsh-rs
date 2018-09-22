#!/bin/sh

if ! where cargo-cov; then
  cargo install cargo-cov
fi

cargo +nightly cov clean && cargo +nightly cov test && cargo +nightly cov report --open
