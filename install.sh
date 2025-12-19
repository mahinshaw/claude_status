#!/usr/bin/env bash
set -e

cargo build --release

mkdir -p ~/.claude
cp target/release/claude_status ~/.claude/claude_status

echo "Installed to ~/.claude/claude_status"
