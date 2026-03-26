#!/bin/bash

set -euo pipefail

TARGET="${TARGET:-riscv64gc-unknown-none-elf}"
PROFILE="${PROFILE:-debug}"
QEMU_DISPLAY="${QEMU_DISPLAY:-gtk}"
QEMU_MEMORY="${QEMU_MEMORY:-128M}"

CARGO_ARGS=(build --target "$TARGET")
ARTIFACT_DIR="debug"

if [[ "$PROFILE" == "release" ]]; then
    CARGO_ARGS+=(--release)
    ARTIFACT_DIR="release"
fi

cargo "${CARGO_ARGS[@]}"

KERNEL="target/$TARGET/$ARTIFACT_DIR/lbl-tg-rcore-tutorial-vga"

exec qemu-system-riscv64 \
    -machine virt \
    -m "$QEMU_MEMORY" \
    -serial stdio \
    -monitor none \
    -display "$QEMU_DISPLAY" \
    -device ramfb \
    -bios none \
    -kernel "$KERNEL"
