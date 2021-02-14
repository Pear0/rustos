#!/bin/sh
set -e

TOP=$(git rev-parse --show-toplevel)

cd "$TOP"

rust_1_50_0=2021-02-11

nightly=${1-${rust_1_50_0}}

rustup override set "nightly-$nightly"

rustup component add rust-src llvm-tools-preview clippy

cargo install cargo-xbuild
cargo install cargo-binutils

