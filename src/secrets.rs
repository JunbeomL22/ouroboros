use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Provider-specific credentials
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCredentials {
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Secrets configuration containing API keys for different providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Secrets {
    #[serde(flatten)]
    pub providers: HashMap<String, ProviderCredentials>,
}

impl Secrets {
    /// Load secrets from a JSON file
    pub fn from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!(
                "Secrets file not found: {:?}\n\
                Please create a secrets.json file with your API keys.\n\
                See secrets.json.example for the format.",
                path
            );
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read secrets file: {:?}", path))?;
        let secrets: Secrets = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse secrets file: {:?}", path))?;
        Ok(secrets)
    }

    /// Get credentials for a specific provider
    pub fn get(&self, provider: &str) -> Option<&ProviderCredentials> {
        self.providers.get(provider)
    }

    /// Get MiniMax credentials with default base URL
    pub fn get_minimax(&self) -> Option<ProviderCredentials> {
        self.providers.get("minimax").map(|creds| ProviderCredentials {
            api_key: creds.api_key.clone(),
            base_url: Some(
                creds
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.minimax.io/anthropic".to_string()),
            ),
        })
    }
}
