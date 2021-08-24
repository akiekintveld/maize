#!/bin/sh
set -e
cargo objcopy \
    --package root \
    --release \
    -- \
    --output-target=binary kernel/usermode_image
cargo objcopy \
    --package kernel \
    --release \
    -- \
    --output-target=binary system_image
qemu-system-riscv64 \
    -machine virt \
    -m 6G \
    -smp 1 \
    -bios default \
    -kernel system_image \
    -d guest_errors \
    -nographic
