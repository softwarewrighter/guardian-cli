//! Output formatting utilities for command results.

use crate::checks::{CheckResult, Severity};
use crate::config::OllamaHost;
use crate::ollama::{GenerateResponse, OllamaModel, PingResult};
use anyhow::Result;

/// Print an error when no hosts are available.
pub fn no_hosts_error(json_output: bool, msg: &str) -> Result<()> {
    if json_output {
        println!(r#"{{"error": "{msg}"}}"#);
    } else {
        println!("{msg}. Add hosts to your guardian.toml file.");
    }
    Ok(())
}

/// Format ping results for output.
pub fn ping_results(results: &[PingResult], json_output: bool) -> Result<()> {
    if json_output {
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.host.name,
                    "base_url": r.host.base_url,
                    "reachable": r.reachable,
                    "fallback": r.host.fallback,
                    "latency_ms": r.latency_ms,
                    "error": r.error,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        for result in results {
            let status = if result.reachable { "UP" } else { "DOWN" };
            let latency = result
                .latency_ms
                .map(|ms| format!(" ({ms}ms)"))
                .unwrap_or_default();
            let fallback = if result.host.fallback {
                " [fallback]"
            } else {
                ""
            };

            if result.reachable {
                println!("  [{status}] {}{latency}{fallback}", result.host.name);
            } else {
                let err = result.error.as_deref().unwrap_or("unknown error");
                println!("  [{status}] {}{fallback} - {err}", result.host.name);
            }
        }
    }
    Ok(())
}

/// Format models list for a host.
pub fn models_list(host: &OllamaHost, models: &[OllamaModel]) {
    println!("\n{} ({}):", host.name, host.base_url);
    if models.is_empty() {
        println!("  (no models)");
    } else {
        for model in models {
            let size = model
                .size
                .map(|s| format!(" ({:.1} GB)", s as f64 / 1e9))
                .unwrap_or_default();
            println!("  - {}{size}", model.name);
        }
    }
}

/// Format selected host for output.
pub fn selected_host(host: &OllamaHost, json_output: bool) -> Result<()> {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "host": host.name,
                "base_url": host.base_url,
                "fallback": host.fallback,
            }))?
        );
    } else {
        println!("{}", host.name);
    }
    Ok(())
}

/// Format LLM ask response.
pub fn ask_response(
    host: &OllamaHost,
    model: &str,
    prompt: &str,
    response: &GenerateResponse,
    json_output: bool,
) -> Result<()> {
    if json_output {
        let json = serde_json::json!({
            "host": host.name,
            "model": model,
            "prompt": prompt,
            "response": response.response,
            "done": response.done,
            "total_duration_ns": response.total_duration,
            "eval_count": response.eval_count,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("[{}] Using model: {}\n", host.name, model);
        println!("{}", response.response);

        if let Some(duration) = response.total_duration {
            let duration_secs = duration as f64 / 1_000_000_000.0;
            println!("\n---");
            println!("Duration: {:.2}s", duration_secs);
            if let Some(tokens) = response.eval_count {
                let tps = tokens as f64 / duration_secs;
                println!("Tokens: {} ({:.1} tokens/sec)", tokens, tps);
            }
        }
    }
    Ok(())
}

/// Format LLM evaluation response.
pub fn evaluate_response(
    host: &OllamaHost,
    model: &str,
    results: &[CheckResult],
    response: &GenerateResponse,
    json_output: bool,
) -> Result<()> {
    if json_output {
        let failures: Vec<_> = results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| {
                serde_json::json!({
                    "check": r.check_name,
                    "severity": format!("{:?}", r.severity).to_lowercase(),
                    "message": r.message,
                    "file": r.file,
                    "line": r.line,
                    "fix": r.fix,
                })
            })
            .collect();

        let json = serde_json::json!({
            "host": host.name,
            "model": model,
            "total_checks": results.len(),
            "passed": results.iter().filter(|r| r.passed).count(),
            "failed": failures.len(),
            "violations": failures,
            "llm_evaluation": response.response,
            "eval_duration_ns": response.total_duration,
        });

        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("=== LLM Evaluation ({} on {}) ===\n", model, host.name);
        println!("{}", response.response);

        if let Some(duration) = response.total_duration {
            let secs = duration as f64 / 1_000_000_000.0;
            println!("\n[Evaluation took {:.1}s]", secs);
        }
    }
    Ok(())
}

/// Format check results for output.
pub fn check_results(results: &[CheckResult], json_output: bool) -> Result<()> {
    if json_output {
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "check": r.check_name,
                    "passed": r.passed,
                    "severity": format!("{:?}", r.severity).to_lowercase(),
                    "message": r.message,
                    "file": r.file,
                    "line": r.line,
                    "fix": r.fix,
                })
            })
            .collect();

        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.len() - passed;
        let errors = results
            .iter()
            .filter(|r| !r.passed && r.severity == Severity::Error)
            .count();

        let summary = serde_json::json!({
            "total": results.len(),
            "passed": passed,
            "failed": failed,
            "errors": errors,
            "results": json_results,
        });

        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    println!("Guardian Checklist Results\n");

    let mut current_check = String::new();
    for result in results {
        if result.check_name != current_check {
            if !current_check.is_empty() {
                println!();
            }
            println!("[{}]", result.check_name);
            current_check = result.check_name.clone();
        }

        let icon = if result.passed { "OK" } else { "FAIL" };
        let severity = match result.severity {
            Severity::Info => "",
            Severity::Warning => " [WARN]",
            Severity::Error => " [ERROR]",
        };

        println!("  [{icon}]{severity} {}", result.message);

        if let Some(fix) = &result.fix {
            println!("       Fix: {fix}");
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let errors = results
        .iter()
        .filter(|r| !r.passed && r.severity == Severity::Error)
        .count();
    let warnings = failed - errors;

    println!("\n---");
    println!(
        "Total: {} | Passed: {} | Failed: {} ({} errors, {} warnings)",
        results.len(),
        passed,
        failed,
        errors,
        warnings
    );

    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
