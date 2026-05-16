use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::BittimeError;

/// Default Bittime API host.
pub const DEFAULT_HOST: &str = "https://openapi.bittime.com";

/// Default WebSocket host for market data.
pub const DEFAULT_WS_MARKET_HOST: &str = "wss://ws.bittime.com/market/ws";

/// Default WebSocket host for user data streams.
pub const DEFAULT_WS_USER_HOST: &str = "wss://wsapi.bittime.com";

/// Environment variable names for credential override.
pub const ENV_API_KEY: &str = "BITTIME_API_KEY";
pub const ENV_API_SECRET: &str = "BITTIME_API_SECRET";

/// Configuration file schema.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub auth: AuthConfig,

    #[serde(default)]
    pub settings: SettingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsConfig {
    #[serde(default = "default_output")]
    pub output: String,

    #[serde(default = "default_host")]
    pub host: String,

    pub default_pair: Option<String>,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            output: default_output(),
            host: default_host(),
            default_pair: None,
        }
    }
}

fn default_output() -> String {
    "table".to_string()
}

fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

impl Config {
    /// Returns the config file path: `~/.config/bittime/config.toml`
    pub fn config_path() -> Result<PathBuf, BittimeError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| BittimeError::Config("Cannot determine config directory".to_string()))?;
        Ok(config_dir.join("bittime").join("config.toml"))
    }

    /// Load config from disk. Returns default config if file doesn't exist.
    pub fn load() -> Result<Self, BittimeError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| {
            BittimeError::Config(format!("Failed to read config at {}: {}", path.display(), e))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            BittimeError::Config(format!(
                "Failed to parse config at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    /// Save config to disk with 0600 permissions.
    pub fn save(&self) -> Result<(), BittimeError> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                BittimeError::Config(format!(
                    "Failed to create config directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| {
            BittimeError::Config(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&path, &content).map_err(|e| {
            BittimeError::Config(format!(
                "Failed to write config at {}: {}",
                path.display(),
                e
            ))
        })?;

        // Set 0600 permissions (owner read/write only)
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms).map_err(|e| {
            BittimeError::Config(format!(
                "Failed to set permissions on {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Delete the config file.
    pub fn delete() -> Result<(), BittimeError> {
        let path = Self::config_path()?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                BittimeError::Config(format!(
                    "Failed to delete config at {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }
}

/// Resolved credentials from multiple sources.
/// Priority: CLI flags → environment variables → config file.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
}

impl Credentials {
    /// Resolve credentials from available sources.
    pub fn resolve(
        cli_key: Option<&str>,
        cli_secret: Option<&str>,
    ) -> Result<Self, BittimeError> {
        // 1. CLI flags
        if let (Some(key), Some(secret)) = (cli_key, cli_secret) {
            return Ok(Self {
                api_key: key.to_string(),
                api_secret: secret.to_string(),
            });
        }

        // 2. Environment variables
        let env_key = std::env::var(ENV_API_KEY).ok();
        let env_secret = std::env::var(ENV_API_SECRET).ok();
        if let (Some(key), Some(secret)) = (env_key, env_secret) {
            return Ok(Self {
                api_key: key,
                api_secret: secret,
            });
        }

        // 3. Config file
        let config = Config::load()?;
        if let (Some(key), Some(secret)) = (config.auth.api_key, config.auth.api_secret) {
            return Ok(Self {
                api_key: key,
                api_secret: secret,
            });
        }

        Err(BittimeError::Auth(
            "No API credentials found. Set via:\n  \
             1. CLI flags: --api-key, --api-secret\n  \
             2. Environment: BITTIME_API_KEY, BITTIME_API_SECRET\n  \
             3. Config: bittime auth set --api-key KEY --api-secret SECRET"
                .to_string(),
        ))
    }

    /// Check if credentials are available without error.
    pub fn available(cli_key: Option<&str>, cli_secret: Option<&str>) -> bool {
        Self::resolve(cli_key, cli_secret).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.settings.host, DEFAULT_HOST);
        assert_eq!(config.settings.output, "table");
        assert!(config.auth.api_key.is_none());
    }

    #[test]
    fn test_config_path() {
        let path = Config::config_path().unwrap();
        assert!(path.to_string_lossy().contains("bittime"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }
}
