#!/usr/bin/env bash
# Deterministic offline Rust gate; CodeRabbit review is a separate explicit step.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --release --workspace
