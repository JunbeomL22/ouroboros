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

/// Subagent definition for --agents flag
#[derive(Debug, Clone)]
pub struct SubagentDef {
    pub name: String,
    pub model: String,
    pub description: String,
    pub prompt: Option<String>,
    pub tools: Vec<String>,
}

impl SubagentDef {
    /// Create a web-searcher subagent from a RoleConfig
    pub fn web_searcher_from_config(config: &RoleConfig) -> Self {
        Self {
            name: "web-searcher".to_string(),
            model: config.model.clone(),
            description: "Web search specialist. Delegates web searches to a cheaper model to save tokens.".to_string(),
            prompt: None,
            tools: vec![
                "WebSearch".to_string(),
                "WebFetch".to_string(),
            ],
        }
    }

    /// Convert to JSON string for --agents flag
    /// Format: {"name": {"description": "...", "prompt": "...", "model": "...", "tools": [...]}}
    pub fn to_json(&self) -> String {
        let prompt = "You are a web search specialist. Search the web for the requested information and return a concise summary of relevant findings. Focus only on answering the specific question asked.";
        let tools_json = self.tools.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ");
        format!(
            r#"{{"{}": {{"description": "{}", "prompt": "{}", "model": "{}", "tools": [{}]}}}}"#,
            self.name,
            self.description,
            prompt,
            self.model,
            tools_json
        )
    }
}

/// Call the configured agent CLI with the given role and prompt
pub fn call_agent(role_config: &RoleConfig, role: &str, prompt: &str) -> Result<String> {
    call_agent_with_subagents(role_config, role, prompt, None)
}

/// Call the configured agent CLI with optional subagents
pub fn call_agent_with_subagents(
    role_config: &RoleConfig,
    role: &str,
    prompt: &str,
    subagents: Option<Vec<SubagentDef>>,
) -> Result<String> {
    match role_config.cli {
        AgentCli::ClaudeCode => call_claude_code(role, prompt, &role_config.model, &role_config.provider, subagents),
        AgentCli::Codex => call_codex(role, prompt, &role_config.model),
        AgentCli::OpenCode => call_opencode(role, prompt, &role_config.model),
        AgentCli::Gemini => call_gemini(role, prompt, &role_config.model),
        AgentCli::Kimi => call_kimi(role, prompt, &role_config.model),
    }
}

/// Call Claude Code CLI with provider-specific environment variables
fn call_claude_code(
    role: &str,
    prompt: &str,
    model: &str,
    provider: &Provider,
    subagents: Option<Vec<SubagentDef>>,
) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    // Build base args
    let mut args: Vec<String> = vec![
        "-p".to_string(),
        "-".to_string(),
        "--model".to_string(),
        model.to_string(),
        "--dangerously-skip-permissions".to_string(),
    ];

    // Add subagents if provided
    if let Some(agents) = &subagents {
        if !agents.is_empty() {
            // Combine all subagent definitions into one JSON object
            let agents_json = if agents.len() == 1 {
                agents[0].to_json()
            } else {
                // Merge multiple agents into one JSON object
                let inner: Vec<String> = agents.iter().map(|a| {
                    let tools_json = a.tools.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ");
                    match &a.prompt {
                        Some(prompt) => format!(
                            r#""{}": {{"model": "{}", "description": "{}", "prompt": "{}", "tools": [{}]}}"#,
                            a.name,
                            a.model,
                            a.description,
                            prompt.replace("\"", "\\\""),
                            tools_json
                        ),
                        None => format!(
                            r#""{}": {{"model": "{}", "description": "{}", "tools": [{}]}}"#,
                            a.name,
                            a.model,
                            a.description,
                            tools_json
                        ),
                    }
                }).collect();
                format!("{{{}}}", inner.join(", "))
            };
            args.push("--agents".to_string());
            args.push(agents_json);
        }
    }

    // Build command with provider-specific environment
    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    {
        let mut cmd_args = vec!["/C".to_string(), "claude".to_string()];
        cmd_args.extend(args);
        cmd.args(&cmd_args);
    }

    #[cfg(not(windows))]
    let mut cmd = Command::new("claude");
    #[cfg(not(windows))]
    cmd.args(&args);

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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
        anyhow::bail!(
            "Claude CLI failed (exit code {}):\nstdout: {}\nstderr: {}",
            exit_code,
            stdout,
            stderr
        );
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

/// Check if a role requires agentic mode (file/command execution)
fn is_agentic_role(role: &str) -> bool {
    matches!(
        role.to_lowercase().as_str(),
        "actor" | "minorfixer" | "majorfixer" | "minor_fixer" | "major_fixer" | "fixer"
    )
}

/// Call Gemini CLI
/// - Agentic roles (Actor, Fixer): use --yolo for file/command execution
/// - Text-generation roles (Outliner, Planner, etc.): no --yolo, just generate text
fn call_gemini(role: &str, prompt: &str, model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    let use_yolo = is_agentic_role(role);

    // Build args based on whether this is an agentic role
    let mut args: Vec<&str> = vec!["--model", model];
    if use_yolo {
        args.push("--yolo");
    }
    args.extend_from_slice(&["--output-format", "text"]);

    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    {
        let mut cmd_args: Vec<String> = vec!["/C".to_string(), "gemini".to_string()];
        cmd_args.extend(args.iter().map(|s| s.to_string()));
        cmd_args.push(full_prompt.clone());
        cmd.args(&cmd_args);
    }

    #[cfg(not(windows))]
    let mut cmd = Command::new("gemini");
    #[cfg(not(windows))]
    {
        cmd.args(&args);
        cmd.arg(&full_prompt);
    }

    let child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn gemini CLI")?;

    let output = child
        .wait_with_output()
        .context("Failed to wait for gemini CLI")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
        anyhow::bail!(
            "Gemini CLI failed (exit code {}):\nstdout: {}\nstderr: {}",
            exit_code,
            stdout,
            stderr
        );
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse gemini output as UTF-8")?;

    Ok(response.trim().to_string())
}

/// Call Kimi CLI
/// Uses --print mode for non-interactive output (implicitly adds --yolo for agentic roles)
/// For text-generation roles: uses --print --output-format text --final-message-only
fn call_kimi(role: &str, prompt: &str, model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Respond only with your output, no explanations.\n\n{}",
        role, prompt
    );

    // Build args: --model, --print for non-interactive, -p for prompt
    // Note: --print implicitly adds --yolo for auto-approval
    let args: Vec<String> = vec![
        "--model".to_string(),
        model.to_string(),
        "--print".to_string(),
        "--output-format".to_string(),
        "text".to_string(),
        "--final-message-only".to_string(),
        "-p".to_string(),
        full_prompt,
    ];

    #[cfg(windows)]
    let mut cmd = Command::new("cmd");
    #[cfg(windows)]
    {
        let mut cmd_args: Vec<String> = vec!["/C".to_string(), "kimi".to_string()];
        cmd_args.extend(args);
        cmd.args(&cmd_args);
    }

    #[cfg(not(windows))]
    let mut cmd = Command::new("kimi");
    #[cfg(not(windows))]
    cmd.args(&args);

    let child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn kimi CLI")?;

    let output = child
        .wait_with_output()
        .context("Failed to wait for kimi CLI")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
        anyhow::bail!(
            "Kimi CLI failed (exit code {}):\nstdout: {}\nstderr: {}",
            exit_code,
            stdout,
            stderr
        );
    }

    let response =
        String::from_utf8(output.stdout).context("Failed to parse kimi output as UTF-8")?;

    Ok(response.trim().to_string())
}

