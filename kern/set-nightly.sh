#!/bin/sh
set -e

TOP=$(git rev-parse --show-toplevel)

cd "$TOP"

nightly=${1-2020-08-15}

rustup override set "nightly-$nightly"

rustup component add rust-src llvm-tools-preview clippy

cargo install cargo-xbuild
cargo install cargo-binutils

