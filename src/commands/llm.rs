//! LLM interaction commands: ask, evaluate.

use super::output;
use crate::checks::{CheckConfig, CheckResult, Severity};
use crate::config::{GuardianConfig, OllamaHost};
use crate::ollama::OllamaClient;
use anyhow::Result;
use std::path::Path;

use super::checks::run_selected_checks;

/// Send a prompt to an Ollama model and get a response.
pub async fn ask(
    config: &GuardianConfig,
    prompt: &str,
    model: Option<&str>,
    host_name: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let client = OllamaClient::new(120_000)?;
    let host = resolve_host(config, &client, host_name).await?;
    let model_name = resolve_model(config, &client, host, model).await?;

    let response = client.generate(host, &model_name, prompt).await?;
    output::ask_response(host, &model_name, prompt, &response, json_output)
}

/// Run checks and have LLM evaluate results to enforce process.
pub async fn evaluate(
    config: &GuardianConfig,
    path: Option<&Path>,
    model: Option<&str>,
    host_name: Option<&str>,
    only: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let project_dir = path.unwrap_or(Path::new("."));

    println!("Running checks on {}...\n", project_dir.display());

    let check_config = CheckConfig::default();
    let results = run_selected_checks(project_dir, &check_config, only);

    let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    let passes: Vec<_> = results.iter().filter(|r| r.passed).collect();

    println!(
        "Checks complete: {} passed, {} failed\n",
        passes.len(),
        failures.len()
    );

    if failures.is_empty() {
        println!("All checks passed. No LLM evaluation needed.");
        return Ok(());
    }

    println!(
        "Sending {} violations to LLM for evaluation...\n",
        failures.len()
    );

    let client = OllamaClient::new(180_000)?;
    let host = resolve_host(config, &client, host_name).await?;
    let model_name = resolve_model(config, &client, host, model).await?;

    let prompt = build_evaluation_prompt(&results, project_dir);
    let response = client.generate(host, &model_name, &prompt).await?;

    output::evaluate_response(host, &model_name, &results, &response, json_output)?;

    if failures.iter().any(|r| r.severity == Severity::Error) {
        std::process::exit(1);
    }

    Ok(())
}

async fn resolve_host<'a>(
    config: &'a GuardianConfig,
    client: &OllamaClient,
    host_name: Option<&str>,
) -> Result<&'a OllamaHost> {
    match host_name {
        Some(name) => config
            .enabled_hosts()
            .into_iter()
            .find(|h| h.name == name)
            .ok_or_else(|| anyhow::anyhow!("Host '{}' not found or disabled", name)),
        None => {
            for host in config.primary_hosts().iter().chain(config.fallback_hosts().iter()) {
                if client.ping_host(host).await.reachable {
                    return Ok(host);
                }
            }
            anyhow::bail!("No reachable hosts found")
        }
    }
}

async fn resolve_model(
    config: &GuardianConfig,
    client: &OllamaClient,
    host: &OllamaHost,
    model: Option<&str>,
) -> Result<String> {
    match model {
        Some(m) => Ok(m.to_string()),
        None => {
            if let Some(default) = &config.ollama.default_model {
                return Ok(default.clone());
            }
            let models = client.list_models(host).await?;
            models
                .first()
                .map(|m| m.name.clone())
                .ok_or_else(|| anyhow::anyhow!("No models available on host {}", host.name))
        }
    }
}

fn build_evaluation_prompt(results: &[CheckResult], project_dir: &Path) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are a code quality guardian enforcing development process rules.\n\n");
    prompt.push_str("## Project\n");
    prompt.push_str(&format!("Directory: {}\n\n", project_dir.display()));
    prompt.push_str("## Check Results\n\n");

    let mut current_check = String::new();
    for result in results {
        if result.check_name != current_check {
            prompt.push_str(&format!("### {}\n", result.check_name));
            current_check = result.check_name.clone();
        }

        let status = if result.passed { "PASS" } else { "FAIL" };
        let severity = match result.severity {
            Severity::Info => "",
            Severity::Warning => " [WARNING]",
            Severity::Error => " [ERROR]",
        };

        prompt.push_str(&format!("- [{}]{} {}\n", status, severity, result.message));

        if let Some(file) = &result.file {
            prompt.push_str(&format!("  File: {}\n", file));
        }
        if let Some(line) = result.line {
            prompt.push_str(&format!("  Line: {}\n", line));
        }
        if let Some(fix) = &result.fix {
            prompt.push_str(&format!("  Suggested fix: {}\n", fix));
        }
    }

    prompt.push_str("\n## Your Task\n\n");
    prompt.push_str(
        "Analyze the FAILED checks above and provide:\n\
        1. A brief summary of the violations\n\
        2. For each ERROR, explain WHY this violates good architecture/process\n\
        3. Specific, actionable instructions to fix each violation\n\
        4. Priority order for fixes (most critical first)\n\n\
        Be concise and direct. Focus on actionable guidance.\n",
    );

    prompt
}
