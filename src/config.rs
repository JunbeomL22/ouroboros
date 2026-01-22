use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Supported agent CLI types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AgentCli {
    ClaudeCode,
    Codex,
}

impl Default for AgentCli {
    fn default() -> Self {
        AgentCli::ClaudeCode
    }
}

impl std::fmt::Display for AgentCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentCli::ClaudeCode => write!(f, "claude-code"),
            AgentCli::Codex => write!(f, "codex"),
        }
    }
}

impl std::str::FromStr for AgentCli {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude" => Ok(AgentCli::ClaudeCode),
            "codex" => Ok(AgentCli::Codex),
            _ => Err(format!(
                "Unknown agent CLI: {}. Supported: claude-code, codex",
                s
            )),
        }
    }
}

/// Configuration for a single role (CLI + model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub cli: AgentCli,
    pub model: String,
}

impl RoleConfig {
    pub fn new(cli: AgentCli, model: &str) -> Self {
        Self {
            cli,
            model: model.to_string(),
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
    pub fixer: RoleConfig,

    // Directory paths
    pub tasks_dir: PathBuf,
    pub results_dir: PathBuf,
    pub plans_dir: PathBuf,
    pub advises_dir: PathBuf,
    pub checks_dir: PathBuf,
    pub hows_dir: PathBuf,
    pub outlines_dir: PathBuf,
    pub fixes_dir: PathBuf,

    // Pipeline settings
    pub checks: usize,
    pub threshold: usize,
    pub max_retries: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            outliner: RoleConfig::new(AgentCli::ClaudeCode, "haiku"),
            planner: RoleConfig::new(AgentCli::ClaudeCode, "opus"),
            advisor: RoleConfig::new(AgentCli::ClaudeCode, "sonnet"),
            actor: RoleConfig::new(AgentCli::ClaudeCode, "opus"),
            checker: RoleConfig::new(AgentCli::ClaudeCode, "sonnet"),
            splitter: RoleConfig::new(AgentCli::ClaudeCode, "opus"),
            fixer: RoleConfig::new(AgentCli::ClaudeCode, "sonnet"),

            tasks_dir: PathBuf::from("./tasks"),
            results_dir: PathBuf::from("./results"),
            plans_dir: PathBuf::from("./plans"),
            advises_dir: PathBuf::from("./advises"),
            checks_dir: PathBuf::from("./checks"),
            hows_dir: PathBuf::from("./hows"),
            outlines_dir: PathBuf::from("./outlines"),
            fixes_dir: PathBuf::from("./fixes"),

            checks: 3,
            threshold: 3,
            max_retries: 3,
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
}

impl Config {
    pub fn load() -> Result<AgentConfig> {
        let cli = Config::parse();

        if cli.config.exists() {
            AgentConfig::from_file(&cli.config)
        } else {
            eprintln!("Config file {:?} not found, using defaults.", cli.config);
            Ok(AgentConfig::default())
        }
    }

}
