#!/bin/sh

TOP=$(git rev-parse --show-toplevel)

export PATH="$PATH:$TOP/bin"

exec qemu-system-aarch64 \
    -nographic \
    -M raspi3 \
    -serial null -serial pty \
    -kernel \
    "$@"
