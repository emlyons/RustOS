#!/bin/sh

TOP=$(git rev-parse --show-toplevel)
$TOP/bin/qemu-system-aarch64 \
    -nographic \
    -s \
    -M raspi3 \
    -serial null -serial pty \
    -kernel \
    "$@"
