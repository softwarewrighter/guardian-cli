//! Guardian CLI - Local LLM governor for development process enforcement.
//!
//! This tool acts as a "local governor" that enforces development process
//! and architecture rules while reducing token usage for cloud-based AI
//! coding agents.

mod checks;
mod commands;
mod config;
mod ollama;

use crate::config::GuardianConfig;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Guardian CLI - Local LLM governor for development process enforcement.
#[derive(Debug, Parser)]
#[command(name = "guardian-cli")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to configuration file (default: ~/.config/guardian-cli/guardian.toml)
    #[arg(long, short, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    /// Enable verbose output
    #[arg(long, short, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Ping all configured Ollama hosts to check availability
    PingHosts,

    /// List models available on reachable Ollama hosts
    ListModels {
        /// Only query a specific host by name
        #[arg(long)]
        host: Option<String>,
    },

    /// Select the best available host (for scripting)
    SelectHost {
        /// Require a specific model to be available
        #[arg(long)]
        model: Option<String>,
    },

    /// Show current configuration
    ShowConfig,

    /// Show default config file path
    ConfigPath,

    /// Send a prompt to an Ollama model and get a response
    Ask {
        /// The prompt to send
        prompt: String,

        /// Model to use (default: from config or first available)
        #[arg(long, short)]
        model: Option<String>,

        /// Specific host to use
        #[arg(long)]
        host: Option<String>,
    },

    /// Run checks AND have LLM evaluate results to enforce process
    Evaluate {
        /// Path to the project directory (default: current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Model to use for evaluation
        #[arg(long, short)]
        model: Option<String>,

        /// Specific host to use
        #[arg(long)]
        host: Option<String>,

        /// Only run specific check(s), comma-separated
        #[arg(long)]
        only: Option<String>,
    },

    /// Run checklist validation on a project
    Check {
        /// Path to the project directory (default: current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Only run specific check(s), comma-separated
        #[arg(long, value_name = "CHECKS")]
        only: Option<String>,

        /// Maximum lines of code per file
        #[arg(long, default_value = "500")]
        max_loc: usize,

        /// Warning threshold for file LOC
        #[arg(long, default_value = "350")]
        warn_loc: usize,

        /// Maximum functions per module
        #[arg(long, default_value = "7")]
        max_functions: usize,

        /// Maximum modules per crate
        #[arg(long, default_value = "4")]
        max_modules: usize,

        /// Required Rust edition
        #[arg(long, default_value = "2024")]
        edition: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let config = GuardianConfig::load(cli.config.as_deref())?;

    match cli.command {
        Commands::PingHosts => commands::ping_hosts(&config, cli.json).await,
        Commands::ListModels { host } => {
            commands::list_models(&config, host.as_deref(), cli.json).await
        }
        Commands::SelectHost { model } => {
            commands::select_host(&config, model.as_deref(), cli.json).await
        }
        Commands::ShowConfig => commands::show_config(&config, cli.json),
        Commands::ConfigPath => commands::config_path(cli.json),
        Commands::Ask {
            prompt,
            model,
            host,
        } => {
            commands::ask(&config, &prompt, model.as_deref(), host.as_deref(), cli.json).await
        }
        Commands::Evaluate {
            path,
            model,
            host,
            only,
        } => {
            commands::evaluate(
                &config,
                path.as_deref(),
                model.as_deref(),
                host.as_deref(),
                only.as_deref(),
                cli.json,
            )
            .await
        }
        Commands::Check {
            path,
            only,
            max_loc,
            warn_loc,
            max_functions,
            max_modules,
            edition,
        } => commands::run_checks(commands::CheckOptions {
            path: path.as_deref(),
            only: only.as_deref(),
            max_loc,
            warn_loc,
            max_functions,
            max_modules,
            edition: &edition,
            json_output: cli.json,
        }),
    }
}

fn init_tracing(verbose: bool) {
    use tracing_subscriber::prelude::*;

    let level = if verbose { "debug" } else { "warn" };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_level(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parsing() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_cli_ping_hosts() {
        let cli = Cli::try_parse_from(["guardian-cli", "ping-hosts"]).unwrap();
        assert!(matches!(cli.command, Commands::PingHosts));
    }

    #[test]
    fn test_cli_list_models() {
        let cli = Cli::try_parse_from(["guardian-cli", "list-models"]).unwrap();
        assert!(matches!(cli.command, Commands::ListModels { host: None }));
    }

    #[test]
    fn test_cli_list_models_with_host() {
        let cli = Cli::try_parse_from(["guardian-cli", "list-models", "--host", "big72"]).unwrap();
        match cli.command {
            Commands::ListModels { host } => assert_eq!(host, Some("big72".to_string())),
            _ => panic!("Expected ListModels command"),
        }
    }

    #[test]
    fn test_cli_json_flag() {
        let cli = Cli::try_parse_from(["guardian-cli", "--json", "ping-hosts"]).unwrap();
        assert!(cli.json);
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::try_parse_from(["guardian-cli", "-v", "ping-hosts"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_config_flag() {
        let cli = Cli::try_parse_from([
            "guardian-cli",
            "--config",
            "/path/to/config.toml",
            "ping-hosts",
        ])
        .unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
    }
}
