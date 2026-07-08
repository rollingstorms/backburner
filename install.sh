#!/bin/sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required to install Backburner." >&2
  echo "Install Rust from https://rustup.rs/, then run this installer again." >&2
  exit 1
fi

cargo install --locked backburner
