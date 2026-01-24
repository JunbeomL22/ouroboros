use crate::config::{AgentCli, Provider, RoleConfig};
use crate::secrets::Secrets;
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::RwLock;

/// Global secrets cache
static SECRETS: Lazy<RwLock<Option<Secrets>>> = Lazy::new(|| RwLock::new(None));

/// Initialize secrets from a file path (call once at startup)
pub fn init_secrets(path: &Path) -> Result<()> {
    let secrets = Secrets::from_file(path)?;
    let mut cache = SECRETS.write().unwrap();
    *cache = Some(secrets);
    Ok(())
}

/// Get cached secrets
fn get_secrets() -> Option<Secrets> {
    SECRETS.read().unwrap().clone()
}

/// Call the configured agent CLI with the given role and prompt
pub fn call_agent(role_config: &RoleConfig, role: &str, prompt: &str) -> Result<String> {
    match role_config.cli {
        AgentCli::ClaudeCode => call_claude_code(role, prompt, &role_config.model, &role_config.provider),
        AgentCli::Codex => call_codex(role, prompt, &role_config.model),
        AgentCli::OpenCode => call_opencode(role, prompt, &role_config.model),
        AgentCli::BrowserUse => call_browser_use(role, prompt, &role_config.model),
    }
}

/// Call Claude Code CLI with provider-specific environment variables
fn call_claude_code(role: &str, prompt: &str, model: &str, provider: &Provider) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    // Build command with provider-specific environment
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    cmd.args([
        "/C",
        "claude",
        "-p",
        "-",
        "--model",
        model,
        "--dangerously-skip-permissions",
    ]);

    #[cfg(not(windows))]
    let mut cmd = Command::new("claude");
    #[cfg(not(windows))]
    cmd.args([
        "-p",
        "-",
        "--model",
        model,
        "--dangerously-skip-permissions",
    ]);

    // Set provider-specific environment variables
    match provider {
        Provider::Minimax => {
            let secrets = get_secrets()
                .ok_or_else(|| anyhow::anyhow!("Secrets not initialized. Please create secrets.json"))?;
            let creds = secrets.get_minimax()
                .ok_or_else(|| anyhow::anyhow!("MiniMax credentials not found in secrets.json"))?;

            cmd.env("ANTHROPIC_BASE_URL", creds.base_url.unwrap_or_else(|| "https://api.minimax.io/anthropic".to_string()));
            cmd.env("ANTHROPIC_AUTH_TOKEN", &creds.api_key);
            cmd.env("DISABLE_PROMPT_CACHING", "1");
        }
        Provider::Anthropic => {
            // Use default Anthropic settings (no env override needed)
            // Optionally use API key from secrets if provided
            if let Some(secrets) = get_secrets() {
                if let Some(creds) = secrets.get("anthropic") {
                    if !creds.api_key.is_empty() {
                        cmd.env("ANTHROPIC_API_KEY", &creds.api_key);
                    }
                }
            }
        }
    }

    let mut child = cmd
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
        .args(["/C", "opencode", "run", "--model", model, "--agent", "build", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn opencode CLI")?;

    #[cfg(not(windows))]
    let mut child = Command::new("opencode")
        .args(["run", "--model", model, "--agent", "build", "-"])
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
        anyhow::bail!(
            "OpenCode CLI failed (exit code {}):\nstdout: {}\nstderr: {}",
            exit_code,
            stdout,
            stderr
        );
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse opencode output as UTF-8")?;

    Ok(response.trim().to_string())
}

/// Call browser_use Python wrapper for web automation
pub fn call_browser_use(role: &str, prompt: &str, _model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    // Get the path to the wrapper script (relative to current working directory)
    let script_path = "scripts/browser_use_wrapper.py";

    #[cfg(windows)]
    let mut child = Command::new("python")
        .args([script_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn browser_use wrapper (python)")?;

    #[cfg(not(windows))]
    let mut child = Command::new("python3")
        .args([script_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn browser_use wrapper (python3)")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(full_prompt.as_bytes())
            .context("Failed to write prompt to browser_use stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for browser_use wrapper")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
        anyhow::bail!(
            "browser_use wrapper failed (exit code {}):\nstdout: {}\nstderr: {}",
            exit_code,
            stdout,
            stderr
        );
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse browser_use output as UTF-8")?;

    Ok(response.trim().to_string())
}
