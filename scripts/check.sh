#!/usr/bin/env bash
set -euo pipefail
export CARGO_INCREMENTAL=0

step() {
  printf '\n== %s ==\n' "$1"
  shift
  "$@"
}

step format cargo fmt --all -- --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step tests cargo test --workspace
step build cargo build --workspace

printf '\nAll checks passed.\n'
