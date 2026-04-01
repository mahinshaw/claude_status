use colored::{ColoredString, Colorize, control};
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;

/// Input model defines the data passed to the claude status line executable
/// See [available data](https://code.claude.com/docs/en/statusline#available-data)
/// See [json scehema](https://code.claude.com/docs/en/statusline#full-json-schema)
#[derive(Deserialize)]
struct Input {
    workspace: Workspace,
    model: Model,
    context_window: ContextWindow,
    version: String,
    #[allow(dead_code)]
    cost: Cost,
    #[allow(dead_code)]
    vim: Option<Vim>,
    #[allow(dead_code)]
    agent: Option<Agent>,
    rate_limits: Option<RateLimits>,
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
    #[allow(dead_code)]
    id: String,
}

#[derive(Deserialize)]
struct ContextWindow {
    total_input_tokens: u64,
    total_output_tokens: u64,
    context_window_size: u64,
    used_percentage: Option<f64>,
    #[allow(dead_code)]
    remaining_percentage: Option<f64>,
    #[allow(dead_code)]
    current_usage: Option<CurrentUsage>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CurrentUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Cost {
    total_cost_usd: f64,
    total_duration_ms: u64,
    total_api_duration_ms: u64,
    total_lines_added: u64,
    total_lines_removed: u64,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Vim {
    mode: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Agent {
    name: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct RateLimits {
    five_hour: Option<Limits>,
    seven_day: Option<Limits>,
}

#[derive(Deserialize)]
struct Limits {
    /// Percentage of the 5-hour or 7-day rate limit consumed, from 0 to 100
    used_percentage: f64,
    /// Unix epoch seconds when the 5-hour or 7-day rate limit window resets
    resets_at: u64,
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
        format!("\u{e725} {}{}", branch, status).bold().red()
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

fn project(input: &Input) -> String {
    let current = current_dir(input);
    let project = current
        .split('/')
        .next_back()
        .unwrap_or(&current)
        .to_string();
    format!("\u{f4d4} {}", project)
}

fn format_token_info(context_window: &ContextWindow) -> (u64, f64) {
    let total_tokens = context_window.total_input_tokens + context_window.total_output_tokens;
    let ctx_pct = match context_window.used_percentage {
        Some(pct) => pct,
        None => (total_tokens * 100 / context_window.context_window_size) as f64,
    };
    (total_tokens, ctx_pct)
}

fn format_rate_limits(rate_limits: &Option<RateLimits>) -> ColoredString {
    let five_hour = rate_limits.as_ref().and_then(|rl| rl.five_hour.as_ref());
    let five_hour_pct = five_hour.map(|l| l.used_percentage).unwrap_or(0.0);
    let five_hour_resets = five_hour
        .map(|l| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let secs = l.resets_at.saturating_sub(now);
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            format!("{}h{}m", hours, mins)
        })
        .unwrap_or_else(|| "-".to_string());
    let rate_limits_str = format!("[5h: {}% resets: {}]", five_hour_pct.round() as u64, five_hour_resets);
    if five_hour_pct < 70.0 {
        rate_limits_str.bold().green()
    } else if five_hour_pct < 90.0 {
        rate_limits_str.bold().yellow()
    } else {
        rate_limits_str.bold().red()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    control::set_override(true);

    let mut input_str = String::new();

    io::stdin().read_to_string(&mut input_str)?;

    let input: Input = serde_json::from_str(&input_str)?;

    let dir = Path::new(&input.workspace.current_dir);
    let _ = env::set_current_dir(dir);

    let project = project(&input);

    let git_info = get_git_info(dir).unwrap_or_default();

    let (total_tokens, ctx_pct) = format_token_info(&input.context_window);

    let version_info = format!("v{}", input.version).bold().blue();
    let model_info = format!("[{}]", input.model.display_name).bold().yellow();
    let ctx_info = format!(
        "[in/out: {}k context: {}k used percentage: {}%]",
        total_tokens / 1000,
        input.context_window.context_window_size / 1000,
        ctx_pct,
    )
    .bold()
    .blue();
    let rate_limits = format_rate_limits(&input.rate_limits);

    print!(
        "{}{} {} {} {} {} {}",
        project.bold().cyan(),
        git_info,
        version_info,
        model_info,
        ctx_info,
        rate_limits,
        "➜".bold().green()
    );
    Ok(())
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
                id: "id".to_string(),
            },
            context_window: make_context_window(0, 0, 100000),
            cost: Cost {
                total_cost_usd: 0.0,
                total_duration_ms: 0,
                total_api_duration_ms: 0,
                total_lines_added: 0,
                total_lines_removed: 0,
            },
            vim: None,
            agent: None,
            rate_limits: None,
        }
    }

    fn make_context_window(input: u64, output: u64, size: u64) -> ContextWindow {
        let used = (((input + output) * 100) / size) as f64;
        let remaining = 100.0 - used;
        ContextWindow {
            total_input_tokens: input,
            total_output_tokens: output,
            context_window_size: size,
            current_usage: None,
            used_percentage: Some(used),
            remaining_percentage: Some(remaining),
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
        assert_eq!(pct, 8.0);
    }

    #[test]
    fn test_format_token_info_zero_tokens() {
        let ctx = make_context_window(0, 0, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 0);
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn test_format_token_info_half_context() {
        let ctx = make_context_window(25000, 25000, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 50000);
        assert_eq!(pct, 50.0);
    }

    #[test]
    fn test_format_token_info_full_context() {
        let ctx = make_context_window(60000, 40000, 100000);
        let (total, pct) = format_token_info(&ctx);
        assert_eq!(total, 100000);
        assert_eq!(pct, 100.0);
    }

    #[test]
    fn test_input_deserialization() {
        let json = r#"{
            "workspace": {"current_dir": "/home/user/project", "project_dir": "/home/user/project"},
            "model": {
                "display_name": "Opus",
                "id": "super opus"
            },
            "version": "1.0.0",
            "context_window": {
                "total_input_tokens": 1000,
                "total_output_tokens": 500,
                "context_window_size": 200000,
                "used_percentage": 0.75,
                "remaining_percentage": 25.0
            },
            "cost": {
                "total_cost_usd": 0.0,
                "total_duration_ms": 0,
                "total_api_duration_ms": 0,
                "total_lines_added": 0,
                "total_lines_removed": 0
            }
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.workspace.current_dir, "/home/user/project");
        assert_eq!(input.workspace.project_dir, "/home/user/project");
        assert_eq!(input.model.display_name, "Opus");
        assert_eq!(input.version, "1.0.0");
        assert_eq!(input.context_window.total_input_tokens, 1000);
        assert_eq!(input.context_window.total_output_tokens, 500);
        assert_eq!(input.context_window.context_window_size, 200000);
        assert_eq!(input.context_window.remaining_percentage, Some(25.0));
        assert_eq!(input.context_window.used_percentage, Some(0.75));
    }
}
