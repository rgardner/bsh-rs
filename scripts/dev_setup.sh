#!/bin/sh

rustup component add cargo-clippy
rustup component add rustfmt-preview

ln -sf ../../scripts/git-hooks/pre-commit .git/hooks/
