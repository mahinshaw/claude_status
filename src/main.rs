use colored::{Colorize, control};
use serde::Deserialize;
use std::env;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;

///
/// See https://code.claude.com/docs/en/statusline#json-input-structure
#[derive(Deserialize)]
struct Input {
    workspace: Workspace,
    model: Model,
    context_window: ContextWindow,
    version: String,
}

#[derive(Deserialize)]
struct Workspace {
    current_dir: String,
    #[allow(dead_code)]
    project_dir: String,
}

#[derive(Deserialize)]
struct Model {
    display_name: String,
}

#[derive(Deserialize)]
struct ContextWindow {
    total_input_tokens: u64,
    total_output_tokens: u64,
    context_window_size: u64,
}

fn get_git_info(dir: &Path) -> Option<String> {
    let check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(dir)
        .output()
        .ok()?;

    if !check.status.success() {
        return None;
    }

    let branch_output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .ok()?;

    let branch = if branch_output.status.success() {
        let b = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();
        if b.is_empty() {
            let sha_output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .current_dir(dir)
                .output()
                .ok()?;
            String::from_utf8_lossy(&sha_output.stdout)
                .trim()
                .to_string()
        } else {
            b
        }
    } else {
        return None;
    };

    let diff_unstaged = Command::new("git")
        .args(["--no-optional-locks", "diff", "--quiet"])
        .current_dir(dir)
        .output()
        .ok()?;
    let unstaged = if !diff_unstaged.status.success() {
        Some("!")
    } else {
        None
    };

    let diff_staged = Command::new("git")
        .args(["--no-optional-locks", "diff", "--cached", "--quiet"])
        .current_dir(dir)
        .output()
        .ok()?;

    let staged = if !diff_staged.status.success() {
        Some("+")
    } else {
        None
    };

    let untracked_output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(dir)
        .output()
        .ok()?;

    let untracked = if untracked_output.status.success() && !untracked_output.stdout.is_empty() {
        Some("?")
    } else {
        None
    };

    let status = if staged.is_some() || unstaged.is_some() || untracked.is_some() {
        format!(
            " [{}{}{}]",
            unstaged.unwrap_or(""),
            staged.unwrap_or(""),
            untracked.unwrap_or("")
        )
    } else {
        String::new()
    };
    Some(format!(
        " {} {}",
        "on".bold().magenta(),
        format!("{}{}", branch, status).bold().red()
    ))
}

fn current_dir(input: &Input) -> String {
    if let Some(home) = env::var_os("HOME") {
        let home_str = home.to_string_lossy();
        if input.workspace.current_dir.starts_with(home_str.as_ref()) {
            input
                .workspace
                .current_dir
                .replacen(home_str.as_ref(), "~", 1)
        } else {
            input.workspace.current_dir.clone()
        }
    } else {
        input.workspace.current_dir.clone()
    }
}

fn format_token_info(context_window: &ContextWindow) -> (u64, u64) {
    let total_tokens = context_window.total_input_tokens + context_window.total_output_tokens;
    let ctx_pct = total_tokens * 100 / context_window.context_window_size;
    (total_tokens, ctx_pct)
}

fn main() {
    control::set_override(true);

    let mut input_str = String::new();
    if io::stdin().read_to_string(&mut input_str).is_err() {
        return;
    }

    let input: Input = match serde_json::from_str(&input_str) {
        Ok(i) => i,
        Err(_) => return,
    };

    let dir = Path::new(&input.workspace.current_dir);
    let _ = env::set_current_dir(dir);

    let current_dir = current_dir(&input);

    let git_info = get_git_info(dir).unwrap_or_default();

    let (total_tokens, ctx_pct) = format_token_info(&input.context_window);

    let version_info = format!("v{}", input.version).bold().blue();
    let model_info = format!("[{}]", input.model.display_name).bold().yellow();
    let ctx_info = format!(
        "[{}k/{}k {}%]",
        total_tokens / 1000,
        input.context_window.context_window_size / 1000,
        ctx_pct
    )
    .bold()
    .blue();

    print!(
        "{}{} {} {} {} {}",
        current_dir.bold().cyan(),
        git_info,
        version_info,
        model_info,
        ctx_info,
        "➜".bold().green()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(current_dir: &str) -> Input {
        Input {
            version: "1.0.0".to_string(),
            workspace: Workspace {
                current_dir: current_dir.to_string(),
                project_dir: current_dir.to_string(),
            },
            model: Model {
                display_name: "TestModel".to_string(),
            },
            context_window: make_context_window(0, 0, 100000),
        }
    }

    fn make_context_window(input: u64, output: u64, size: u64) -> ContextWindow {
        ContextWindow {
            total_input_tokens: input,
            total_output_tokens: output,
            context_window_size: size,
        }
    }

    #[test]
    fn test_current_dir_with_home_substitution() {
        if let Some(home) = env::var_os("HOME") {
            let home_str = home.to_string_lossy();
            let input = make_input(&format!("{}/projects/test", home_str));
            let result = current_dir(&input);
            assert_eq!(result, "~/projects/test");
        }
    }

    #[test]
    fn test_current_dir_without_home_prefix() {
        let input = make_input("/tmp/some/path");
        let result = current_dir(&input);
        assert_eq!(result, "/tmp/some/path");
    }

    #[test]
    fn test_current_dir_exact_home() {
        if let Some(home) = env::var_os("HOME") {
            let home_str = home.to_string_lossy();
            let input = make_input(&home_str);
            let result = current_dir(&input);
            assert_eq!(result, "~");
        }
    }

    #[test]
    fn test_format_token_info_basic() {
        let ctx = make_context_window(5000, 3000, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 8000);
        assert_eq!(pct, 8);
    }

    #[test]
    fn test_format_token_info_zero_tokens() {
        let ctx = make_context_window(0, 0, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 0);
        assert_eq!(pct, 0);
    }

    #[test]
    fn test_format_token_info_half_context() {
        let ctx = make_context_window(25000, 25000, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 50000);
        assert_eq!(pct, 50);
    }

    #[test]
    fn test_format_token_info_full_context() {
        let ctx = make_context_window(60000, 40000, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 100000);
        assert_eq!(pct, 100);
    }

    #[test]
    fn test_input_deserialization() {
        let json = r#"{
            "workspace": {"current_dir": "/home/user/project", "project_dir": "/home/user/project"},
            "model": {"display_name": "Claude Opus"},
            "version": "1.0.0",
            "context_window": {
                "total_input_tokens": 1000,
                "total_output_tokens": 500,
                "context_window_size": 200000
            }
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.workspace.current_dir, "/home/user/project");
        assert_eq!(input.workspace.project_dir, "/home/user/project");
        assert_eq!(input.model.display_name, "Claude Opus");
        assert_eq!(input.version, "1.0.0");
        assert_eq!(input.context_window.total_input_tokens, 1000);
        assert_eq!(input.context_window.total_output_tokens, 500);
        assert_eq!(input.context_window.context_window_size, 200000);
    }
}
