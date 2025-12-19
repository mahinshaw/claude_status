# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build --release    # Build optimized binary to target/release/claude_status
cargo build              # Build debug binary
cargo check              # Type-check without building
./install.sh             # Build and install to ~/.claude/claude_status
```

## Project Overview

This is a Rust binary that generates a formatted statusline for Claude Code. It reads JSON from stdin containing workspace, model, and context window information, then outputs a colored statusline showing:

- Current directory (with ~ substitution for home)
- Git branch and dirty status (if in a git repo)
- Model name
- Token usage (input+output / context window size)

The binary forces color output via `colored::control::set_override(true)` since it's typically used in a pipe context where stdout is not a TTY.
