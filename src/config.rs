//! Configuration loading for Guardian CLI.
//!
//! Loads configuration from TOML files, supporting:
//! - Ollama host definitions with fallback support
//! - Default timeout and model settings
//! - Policy and script configurations (future)

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// An Ollama host configuration.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct OllamaHost {
    /// Human-readable name for this host (e.g., "big72", "local").
    pub name: String,

    /// Base URL for the Ollama API (e.g., "http://big72:11434").
    pub base_url: String,

    /// Whether this host is enabled for use.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whether this is a fallback host (used only when primaries are unavailable).
    #[serde(default)]
    pub fallback: bool,

    /// Optional description of this host.
    #[serde(default)]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Ollama-related configuration.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct OllamaSection {
    /// Default timeout in milliseconds for HTTP requests.
    #[serde(default)]
    pub default_timeout_ms: Option<u64>,

    /// Default host name to use when not specified.
    #[serde(default)]
    pub default_host: Option<String>,

    /// Default model to use for LLM operations.
    #[serde(default)]
    pub default_model: Option<String>,

    /// List of configured Ollama hosts.
    #[serde(default)]
    pub hosts: Vec<OllamaHost>,
}

/// Root configuration structure for Guardian CLI.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct GuardianConfig {
    /// Ollama-related configuration.
    #[serde(default)]
    pub ollama: OllamaSection,
}

impl GuardianConfig {
    /// Load configuration from a file path.
    ///
    /// If `config_path` is `None`, attempts to load from the default location.
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let path = match config_path {
            Some(p) => p.to_path_buf(),
            None => default_config_path().context("Could not determine default config path")?,
        };

        if !path.exists() {
            tracing::warn!(
                "Config file not found at {}, using defaults",
                path.display()
            );
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file at {}", path.display()))?;

        let cfg: Self = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse TOML config at {}", path.display()))?;

        Ok(cfg)
    }

    /// Get the default timeout in milliseconds.
    pub fn default_timeout_ms(&self) -> u64 {
        self.ollama.default_timeout_ms.unwrap_or(2500)
    }

    /// Get primary (non-fallback) hosts that are enabled.
    pub fn primary_hosts(&self) -> Vec<&OllamaHost> {
        self.ollama
            .hosts
            .iter()
            .filter(|h| h.enabled && !h.fallback)
            .collect()
    }

    /// Get fallback hosts that are enabled.
    pub fn fallback_hosts(&self) -> Vec<&OllamaHost> {
        self.ollama
            .hosts
            .iter()
            .filter(|h| h.enabled && h.fallback)
            .collect()
    }

    /// Get all enabled hosts (primary first, then fallback).
    pub fn enabled_hosts(&self) -> Vec<&OllamaHost> {
        let mut hosts = self.primary_hosts();
        hosts.extend(self.fallback_hosts());
        hosts
    }
}

/// Get the default configuration file path.
///
/// Returns `~/.config/guardian-cli/guardian.toml` on Unix systems.
pub fn default_config_path() -> Option<PathBuf> {
    let proj = ProjectDirs::from("com", "softwarewrighter", "guardian-cli")?;
    Some(proj.config_dir().join("guardian.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[ollama]
"#;
        let cfg: GuardianConfig = toml::from_str(toml).unwrap();
        assert!(cfg.ollama.hosts.is_empty());
        assert_eq!(cfg.default_timeout_ms(), 2500);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[ollama]
default_timeout_ms = 3000
default_host = "big72"
default_model = "qwen2.5-coder:7b"

[[ollama.hosts]]
name = "big72"
base_url = "http://big72:11434"
enabled = true
fallback = false
description = "Main server"

[[ollama.hosts]]
name = "local"
base_url = "http://localhost:11434"
enabled = true
fallback = true
"#;
        let cfg: GuardianConfig = toml::from_str(toml).unwrap();

        assert_eq!(cfg.default_timeout_ms(), 3000);
        assert_eq!(cfg.ollama.default_host, Some("big72".to_string()));
        assert_eq!(cfg.ollama.hosts.len(), 2);

        let primaries = cfg.primary_hosts();
        assert_eq!(primaries.len(), 1);
        assert_eq!(primaries[0].name, "big72");

        let fallbacks = cfg.fallback_hosts();
        assert_eq!(fallbacks.len(), 1);
        assert_eq!(fallbacks[0].name, "local");
    }

    #[test]
    fn test_host_enabled_defaults_to_true() {
        let toml = r#"
[[ollama.hosts]]
name = "test"
base_url = "http://test:11434"
"#;
        let cfg: GuardianConfig = toml::from_str(toml).unwrap();
        assert!(cfg.ollama.hosts[0].enabled);
        assert!(!cfg.ollama.hosts[0].fallback);
    }

    #[test]
    fn test_disabled_host_not_in_enabled_list() {
        let toml = r#"
[[ollama.hosts]]
name = "disabled"
base_url = "http://disabled:11434"
enabled = false

[[ollama.hosts]]
name = "enabled"
base_url = "http://enabled:11434"
enabled = true
"#;
        let cfg: GuardianConfig = toml::from_str(toml).unwrap();
        let enabled = cfg.enabled_hosts();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "enabled");
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("guardian.toml");

        let toml = r#"
[ollama]
default_timeout_ms = 5000

[[ollama.hosts]]
name = "test"
base_url = "http://test:11434"
"#;
        fs::write(&config_path, toml).unwrap();

        let cfg = GuardianConfig::load(Some(&config_path)).unwrap();
        assert_eq!(cfg.default_timeout_ms(), 5000);
        assert_eq!(cfg.ollama.hosts.len(), 1);
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        let cfg = GuardianConfig::load(Some(&config_path)).unwrap();
        assert!(cfg.ollama.hosts.is_empty());
        assert_eq!(cfg.default_timeout_ms(), 2500);
    }

    #[test]
    fn test_enabled_hosts_order() {
        let toml = r#"
[[ollama.hosts]]
name = "fallback1"
base_url = "http://fallback1:11434"
fallback = true

[[ollama.hosts]]
name = "primary1"
base_url = "http://primary1:11434"
fallback = false

[[ollama.hosts]]
name = "primary2"
base_url = "http://primary2:11434"
fallback = false
"#;
        let cfg: GuardianConfig = toml::from_str(toml).unwrap();
        let enabled = cfg.enabled_hosts();

        // Primaries should come before fallbacks
        assert_eq!(enabled.len(), 3);
        assert!(!enabled[0].fallback);
        assert!(!enabled[1].fallback);
        assert!(enabled[2].fallback);
    }
}
