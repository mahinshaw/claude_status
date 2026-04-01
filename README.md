# claude_status

A custom statusline binary for [Claude Code](https://claude.ai/code) that displays workspace, git, model, and context window information in the prompt.

**Output format:**
```
 project-name on  main [!+?] v1.0.80 [Opus] [in/out: 19k context: 200k used percentage: 9.8%] ➜
```

Shows: project name, git branch + dirty status (`!` unstaged, `+` staged, `?` untracked), Claude version, model, and token usage.

## Requirements

- Rust toolchain (`cargo`)
- A [Nerd Font](https://www.nerdfonts.com/) for icons

## Installation

```bash
./install.sh
```

This builds the release binary and copies it to `~/bin/claude_status`.

## Claude Code Configuration

Add the following to your Claude Code settings (`~/.claude/settings.json`):

```json
{
  "statusline": {
    "type": "custom",
    "command": "~/bin/claude_status"
  }
}
```

## Local Testing

```bash
./test.sh
```

Runs the binary with sample JSON input so you can preview the output without Claude Code.
