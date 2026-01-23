use crate::config::{AgentCli, RoleConfig};
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Call the configured agent CLI with the given role and prompt
pub fn call_agent(role_config: &RoleConfig, role: &str, prompt: &str) -> Result<String> {
    match role_config.cli {
        AgentCli::ClaudeCode => call_claude_code(role, prompt, &role_config.model),
        AgentCli::Codex => call_codex(role, prompt, &role_config.model),
        AgentCli::OpenCode => call_opencode(role, prompt, &role_config.model),
    }
}

/// Call Claude Code CLI
fn call_claude_code(role: &str, prompt: &str, model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args([
            "/C",
            "claude",
            "-p",
            "-",
            "--model",
            model,
            "--dangerously-skip-permissions",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn claude CLI")?;

    #[cfg(not(windows))]
    let mut child = Command::new("claude")
        .args([
            "-p",
            "-",
            "--model",
            model,
            "--dangerously-skip-permissions",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn claude CLI")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(full_prompt.as_bytes())
            .context("Failed to write prompt to claude stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for claude CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI failed: {}", stderr);
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse claude output as UTF-8")?;

    Ok(response.trim().to_string())
}

/// Call OpenAI Codex CLI
fn call_codex(role: &str, prompt: &str, model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args([
            "/C",
            "codex",
            "exec",
            "--model",
            model,
            "--full-auto",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn codex CLI")?;

    #[cfg(not(windows))]
    let mut child = Command::new("codex")
        .args([
            "exec",
            "--model",
            model,
            "--full-auto",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn codex CLI")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(full_prompt.as_bytes())
            .context("Failed to write prompt to codex stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for codex CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Codex CLI failed: {}", stderr);
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse codex output as UTF-8")?;

    Ok(response.trim().to_string())
}

/// Call OpenCode CLI
fn call_opencode(role: &str, prompt: &str, model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args([
            "/C",
            "opencode",
            "--model",
            model,
            "--dangerously-skip-permissions",
            "-p",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn opencode CLI")?;

    #[cfg(not(windows))]
    let mut child = Command::new("opencode")
        .args([
            "--model",
            model,
            "--dangerously-skip-permissions",
            "-p",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn opencode CLI")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(full_prompt.as_bytes())
            .context("Failed to write prompt to opencode stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for opencode CLI")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("OpenCode CLI failed: {}", stderr);
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse opencode output as UTF-8")?;

    Ok(response.trim().to_string())
}
