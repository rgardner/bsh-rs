#!/bin/sh

repo_root="$(git rev-parse --show-toplevel)"

rustup toolchain install "$(cat "$repo_root"/rustnightly.txt)"
cp scripts/git-hooks/* .git/hooks/
