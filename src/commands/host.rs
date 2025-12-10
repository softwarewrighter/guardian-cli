//! Host-related commands: ping, list-models, select-host.

use super::output;
use crate::config::{GuardianConfig, OllamaHost};
use crate::ollama::OllamaClient;
use anyhow::Result;

fn host_result_json(
    host: &OllamaHost,
    reachable: bool,
    models: &[&str],
    error: Option<String>,
) -> serde_json::Value {
    let mut json = serde_json::json!({
        "host": host.name,
        "base_url": host.base_url,
        "reachable": reachable,
        "models": models,
    });
    if let Some(e) = error {
        json["error"] = serde_json::Value::String(e);
    }
    json
}

/// Ping all configured hosts and report their status.
pub async fn ping_hosts(config: &GuardianConfig, json_output: bool) -> Result<()> {
    let hosts = config.enabled_hosts();

    if hosts.is_empty() {
        return output::no_hosts_error(json_output, "No hosts configured");
    }

    let client = OllamaClient::new(config.default_timeout_ms())?;
    let results = client.ping_hosts(&hosts).await;

    if !json_output {
        println!("Pinging {} host(s)...\n", hosts.len());
    }

    output::ping_results(&results, json_output)?;

    if !json_output {
        let reachable = results.iter().filter(|r| r.reachable).count();
        println!("\n{reachable}/{} hosts reachable", results.len());
    }
    Ok(())
}

/// List models on reachable hosts.
pub async fn list_models(
    config: &GuardianConfig,
    host_filter: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let hosts = match host_filter {
        Some(name) => config
            .enabled_hosts()
            .into_iter()
            .filter(|h| h.name == name)
            .collect(),
        None => config.enabled_hosts(),
    };

    if hosts.is_empty() {
        return output::no_hosts_error(json_output, "No matching hosts found");
    }

    let client = OllamaClient::new(config.default_timeout_ms())?;
    let mut results = Vec::new();

    for host in &hosts {
        let ping = client.ping_host(host).await;
        if !ping.reachable {
            if !json_output {
                println!("\n{} ({}): UNREACHABLE", host.name, host.base_url);
            }
            results.push(host_result_json(host, false, &[], None));
            continue;
        }

        match client.list_models(host).await {
            Ok(models) => {
                if !json_output {
                    output::models_list(host, &models);
                }
                let names: Vec<_> = models.iter().map(|m| m.name.as_str()).collect();
                results.push(host_result_json(host, true, &names, None));
            }
            Err(e) => {
                if !json_output {
                    println!("\n{} ({}): ERROR - {e}", host.name, host.base_url);
                }
                results.push(host_result_json(host, true, &[], Some(e.to_string())));
            }
        }
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&results)?);
    }
    Ok(())
}

/// Select the best available host.
pub async fn select_host(
    config: &GuardianConfig,
    required_model: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let client = OllamaClient::new(config.default_timeout_ms())?;

    // Try primary hosts first, then fallbacks
    for host in config.primary_hosts().iter().chain(config.fallback_hosts().iter()) {
        if let Some(h) = try_host(&client, host, required_model).await {
            return output::selected_host(h, json_output);
        }
    }

    if json_output {
        println!(r#"{{"error": "No suitable hosts available"}}"#);
    } else {
        eprintln!("No suitable hosts available");
    }
    std::process::exit(1);
}

async fn try_host<'a>(
    client: &OllamaClient,
    host: &'a OllamaHost,
    required_model: Option<&str>,
) -> Option<&'a OllamaHost> {
    if !client.ping_host(host).await.reachable {
        return None;
    }

    match required_model {
        Some(model) => match client.list_models(host).await {
            Ok(models) if models.iter().any(|m| m.name == model) => Some(host),
            _ => None,
        },
        None => Some(host),
    }
}
