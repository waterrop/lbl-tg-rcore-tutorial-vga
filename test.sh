#!/bin/bash

set -euo pipefail

HOST_TARGET="${HOST_TARGET:-x86_64-unknown-linux-gnu}"
RISCV_TARGET="${RISCV_TARGET:-riscv64gc-unknown-none-elf}"

cargo test --target "$HOST_TARGET"
cargo check --target "$RISCV_TARGET"
cargo build --target "$RISCV_TARGET"
cargo clippy --target "$HOST_TARGET" --all-targets -- -D warnings
