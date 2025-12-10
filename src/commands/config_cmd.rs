//! Configuration-related commands.

use crate::config::{GuardianConfig, OllamaHost};
use anyhow::Result;

/// Show current configuration.
pub fn show_config(config: &GuardianConfig, json_output: bool) -> Result<()> {
    if json_output {
        print_config_json(config)
    } else {
        print_config_text(config);
        Ok(())
    }
}

/// Show the default config file path.
pub fn config_path(json_output: bool) -> Result<()> {
    let path = crate::config::default_config_path();

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": path.as_ref().map(|p| p.display().to_string()),
            }))?
        );
    } else {
        match path {
            Some(p) => println!("{}", p.display()),
            None => {
                eprintln!("Could not determine config path");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn print_config_json(config: &GuardianConfig) -> Result<()> {
    let json = serde_json::json!({
        "default_timeout_ms": config.default_timeout_ms(),
        "default_host": config.ollama.default_host,
        "default_model": config.ollama.default_model,
        "hosts": config.ollama.hosts.iter().map(|h| {
            serde_json::json!({
                "name": h.name,
                "base_url": h.base_url,
                "enabled": h.enabled,
                "fallback": h.fallback,
                "description": h.description,
            })
        }).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

fn print_config_text(config: &GuardianConfig) {
    println!("Guardian CLI Configuration\n");
    println!("Timeout: {}ms", config.default_timeout_ms());
    if let Some(host) = &config.ollama.default_host {
        println!("Default host: {host}");
    }
    if let Some(model) = &config.ollama.default_model {
        println!("Default model: {model}");
    }

    println!("\nConfigured hosts:");
    if config.ollama.hosts.is_empty() {
        println!("  (none)");
    } else {
        for host in &config.ollama.hosts {
            print_host_info(host);
        }
    }
}

fn print_host_info(host: &OllamaHost) {
    let status = if host.enabled { "enabled" } else { "disabled" };
    let fallback = if host.fallback { ", fallback" } else { "" };
    println!("  - {} ({}) [{status}{fallback}]", host.name, host.base_url);
    if let Some(desc) = &host.description {
        println!("    {desc}");
    }
}
