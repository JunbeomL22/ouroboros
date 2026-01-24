use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Supported agent CLI types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AgentCli {
    #[serde(rename = "claude", alias = "claude-code")]
    ClaudeCode,
    Codex,
    #[serde(alias = "opencode")]
    OpenCode,
    #[serde(rename = "browser-use", alias = "browser_use")]
    BrowserUse,
}

/// Supported API providers for Claude Code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    #[default]
    Anthropic,
    Minimax,
}

impl Default for AgentCli {
    fn default() -> Self {
        AgentCli::ClaudeCode
    }
}

impl std::fmt::Display for AgentCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentCli::ClaudeCode => write!(f, "claude"),
            AgentCli::Codex => write!(f, "codex"),
            AgentCli::OpenCode => write!(f, "opencode"),
            AgentCli::BrowserUse => write!(f, "browser-use"),
        }
    }
}

impl std::str::FromStr for AgentCli {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude" => Ok(AgentCli::ClaudeCode),
            "codex" => Ok(AgentCli::Codex),
            "opencode" | "open-code" => Ok(AgentCli::OpenCode),
            "browser-use" | "browser_use" => Ok(AgentCli::BrowserUse),
            _ => Err(format!(
                "Unknown agent CLI: {}. Supported: claude, codex, opencode, browser-use",
                s
            )),
        }
    }
}

/// Configuration for a single role (CLI + model + provider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub cli: AgentCli,
    pub model: String,
    #[serde(default)]
    pub provider: Provider,
}

impl RoleConfig {
    pub fn new(cli: AgentCli, model: &str) -> Self {
        Self {
            cli,
            model: model.to_string(),
            provider: Provider::default(),
        }
    }
}

/// Agent configuration with per-role CLI and model settings, plus pipeline options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    // Role configurations
    pub outliner: RoleConfig,
    pub planner: RoleConfig,
    pub advisor: RoleConfig,
    pub actor: RoleConfig,
    pub checker: RoleConfig,
    pub splitter: RoleConfig,
    pub minor_fixer: RoleConfig,
    pub major_fixer: RoleConfig,

    // Directory paths
    pub tasks_dir: PathBuf,
    pub results_dir: PathBuf,
    pub plans_dir: PathBuf,
    pub advises_dir: PathBuf,
    pub checks_dir: PathBuf,
    pub hows_dir: PathBuf,
    pub outlines_dir: PathBuf,
    pub fixes_dir: PathBuf,
    pub rechecks_dir: PathBuf,

    // Pipeline settings
    pub checks: usize,
    pub threshold: usize,
    pub recheck_threshold: usize,
    pub max_tries: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            outliner: RoleConfig::new(AgentCli::ClaudeCode, "haiku"),
            planner: RoleConfig::new(AgentCli::ClaudeCode, "opus"),
            advisor: RoleConfig::new(AgentCli::ClaudeCode, "sonnet"),
            actor: RoleConfig::new(AgentCli::ClaudeCode, "opus"),
            checker: RoleConfig::new(AgentCli::OpenCode, "minimax/MiniMax-M2.1"),
            splitter: RoleConfig::new(AgentCli::ClaudeCode, "opus"),
            minor_fixer: RoleConfig::new(AgentCli::ClaudeCode, "haiku"),
            major_fixer: RoleConfig::new(AgentCli::ClaudeCode, "opus"),

            tasks_dir: PathBuf::from("./tasks"),
            results_dir: PathBuf::from("./results"),
            plans_dir: PathBuf::from("./plans"),
            advises_dir: PathBuf::from("./advises"),
            checks_dir: PathBuf::from("./checks"),
            hows_dir: PathBuf::from("./hows"),
            outlines_dir: PathBuf::from("./outlines"),
            fixes_dir: PathBuf::from("./fixes"),
            rechecks_dir: PathBuf::from("./rechecks"),

            checks: 3,
            threshold: 3,
            recheck_threshold: 3,
            max_tries: 3,
        }
    }
}

impl AgentConfig {
    /// Load agent configuration from a JSON file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read agent config file: {:?}", path))?;
        let config: AgentConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse agent config file: {:?}", path))?;
        Ok(config)
    }

    /// Save agent configuration to a JSON file
    #[allow(dead_code)]
    pub fn to_file(&self, path: &PathBuf) -> Result<()> {
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize agent config")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write agent config file: {:?}", path))?;
        Ok(())
    }

    /// Load from JSON string
    #[allow(dead_code)]
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse agent config JSON")
    }

    /// Serialize to JSON string
    #[allow(dead_code)]
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize agent config to JSON")
    }
}

#[derive(Parser, Debug, Clone)]
#[command(name = "ouroboros")]
#[command(about = "Recursive agent pipeline for sequential task processing")]
pub struct Config {
    /// Path to configuration file (JSON format)
    #[arg(long, short = 'c', default_value = "./config.json")]
    pub config: PathBuf,

    /// Path to secrets file (JSON format with API keys)
    #[arg(long, short = 's')]
    pub secrets: Option<PathBuf>,
}

impl Config {
    /// Load configuration and determine secrets path
    /// Returns (AgentConfig, secrets_path)
    /// Secrets path priority: CLI option > config.json directory > current directory
    pub fn load() -> Result<(AgentConfig, PathBuf)> {
        let cli = Config::parse();

        let agent_config = if cli.config.exists() {
            AgentConfig::from_file(&cli.config)?
        } else {
            eprintln!("Config file {:?} not found, using defaults.", cli.config);
            AgentConfig::default()
        };

        // Determine secrets path
        let secrets_path = if let Some(path) = cli.secrets {
            // CLI option takes priority
            path
        } else if let Some(config_dir) = cli.config.parent() {
            // Try config.json directory
            let dir_secrets = config_dir.join("secrets.json");
            if dir_secrets.exists() {
                dir_secrets
            } else {
                // Fall back to current directory
                PathBuf::from("./secrets.json")
            }
        } else {
            PathBuf::from("./secrets.json")
        };

        Ok((agent_config, secrets_path))
    }
}
