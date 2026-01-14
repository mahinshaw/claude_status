#!/usr/bin/env bash
set -e

cargo build --release

mkdir -p ~/bin
cp -f target/release/claude_status ~/bin/claude_status

echo "Installed to ~/bin/claude_status"
