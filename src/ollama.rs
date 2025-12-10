//! Ollama HTTP client for Guardian CLI.
//!
//! Provides async communication with Ollama API servers for:
//! - Health checks (ping)
//! - Model listing
//! - Text generation

use crate::config::OllamaHost;
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Information about a model available on an Ollama server.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct OllamaModel {
    /// Model name/tag (e.g., "qwen2.5-coder:7b").
    pub name: String,

    /// Last modification timestamp.
    #[serde(default)]
    pub modified_at: Option<String>,

    /// Model size in bytes.
    #[serde(default)]
    pub size: Option<u64>,

    /// Additional model details.
    #[serde(default)]
    pub digest: Option<String>,
}

/// Response from the /api/tags endpoint.
#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<OllamaModel>,
}

/// Request for text generation.
#[derive(Debug, Serialize)]
pub struct GenerateRequest {
    /// Model name to use.
    pub model: String,
    /// The prompt to send.
    pub prompt: String,
    /// Whether to stream responses (false for single response).
    pub stream: bool,
}

/// Response from text generation.
#[derive(Debug, Deserialize)]
pub struct GenerateResponse {
    /// The generated text.
    pub response: String,
    /// Whether generation is complete.
    pub done: bool,
    /// Total duration in nanoseconds.
    #[serde(default)]
    pub total_duration: Option<u64>,
    /// Tokens evaluated per second.
    #[serde(default)]
    pub eval_count: Option<u64>,
}

/// Result of pinging a host.
#[derive(Debug, Clone)]
pub struct PingResult {
    /// The host that was pinged.
    pub host: OllamaHost,

    /// Whether the host responded successfully.
    pub reachable: bool,

    /// Response time in milliseconds (if reachable).
    pub latency_ms: Option<u64>,

    /// Error message (if not reachable).
    pub error: Option<String>,
}

/// HTTP client for communicating with Ollama servers.
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
}

impl OllamaClient {
    /// Create a new Ollama client with the specified timeout.
    pub fn new(timeout_ms: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client })
    }

    /// Ping a host to check if it's reachable and Ollama is responding.
    pub async fn ping_host(&self, host: &OllamaHost) -> PingResult {
        let url = format!("{}/api/tags", host.base_url.trim_end_matches('/'));
        let start = std::time::Instant::now();

        debug!(host = %host.name, url = %url, "Pinging Ollama host");

        match self.client.get(&url).send().await {
            Ok(resp) if resp.status() == StatusCode::OK => {
                let latency = start.elapsed().as_millis() as u64;
                info!(host = %host.name, latency_ms = latency, "Host reachable");
                PingResult {
                    host: host.clone(),
                    reachable: true,
                    latency_ms: Some(latency),
                    error: None,
                }
            }
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                let status = resp.status();
                warn!(host = %host.name, status = %status, "Host returned non-OK status");
                PingResult {
                    host: host.clone(),
                    reachable: false,
                    latency_ms: Some(latency),
                    error: Some(format!("HTTP status: {status}")),
                }
            }
            Err(e) => {
                warn!(host = %host.name, error = %e, "Failed to reach host");
                PingResult {
                    host: host.clone(),
                    reachable: false,
                    latency_ms: None,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// List all models available on a host.
    pub async fn list_models(&self, host: &OllamaHost) -> Result<Vec<OllamaModel>> {
        let url = format!("{}/api/tags", host.base_url.trim_end_matches('/'));

        debug!(host = %host.name, url = %url, "Listing models");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {}", host.name))?;

        if !resp.status().is_success() {
            warn!(host = %host.name, status = %resp.status(), "Failed to list models");
            anyhow::bail!(
                "Host {} returned HTTP {}: {}",
                host.name,
                resp.status().as_u16(),
                resp.status().canonical_reason().unwrap_or("Unknown")
            );
        }

        let tags: TagsResponse = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse response from {}", host.name))?;

        info!(host = %host.name, model_count = tags.models.len(), "Listed models");
        Ok(tags.models)
    }

    /// Generate text using a model on a host.
    pub async fn generate(
        &self,
        host: &OllamaHost,
        model: &str,
        prompt: &str,
    ) -> Result<GenerateResponse> {
        let url = format!("{}/api/generate", host.base_url.trim_end_matches('/'));

        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
        };

        info!(
            host = %host.name,
            model = %model,
            prompt_len = prompt.len(),
            "Sending generate request"
        );
        debug!(prompt = %prompt, "Full prompt");

        let start = std::time::Instant::now();

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {}", host.name))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(
                host = %host.name,
                status = %status,
                body = %body,
                "Generate request failed"
            );
            anyhow::bail!("Host {} returned HTTP {}: {}", host.name, status, body);
        }

        let gen_resp: GenerateResponse = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse generate response from {}", host.name))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            host = %host.name,
            model = %model,
            response_len = gen_resp.response.len(),
            duration_ms = duration_ms,
            eval_count = ?gen_resp.eval_count,
            "Generate complete"
        );
        debug!(response = %gen_resp.response, "Full response");

        Ok(gen_resp)
    }

    /// Ping multiple hosts concurrently and return results.
    pub async fn ping_hosts(&self, hosts: &[&OllamaHost]) -> Vec<PingResult> {
        let futures: Vec<_> = hosts.iter().map(|host| self.ping_host(host)).collect();

        futures::future::join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_host(name: &str, port: u16) -> OllamaHost {
        OllamaHost {
            name: name.to_string(),
            base_url: format!("http://127.0.0.1:{port}"),
            enabled: true,
            fallback: false,
            description: None,
        }
    }

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new(2500);
        assert!(client.is_ok());
    }

    #[test]
    fn test_ping_result_reachable() {
        let host = test_host("test", 11434);
        let result = PingResult {
            host: host.clone(),
            reachable: true,
            latency_ms: Some(50),
            error: None,
        };
        assert!(result.reachable);
        assert_eq!(result.latency_ms, Some(50));
    }

    #[test]
    fn test_ping_result_unreachable() {
        let host = test_host("test", 11434);
        let result = PingResult {
            host: host.clone(),
            reachable: false,
            latency_ms: None,
            error: Some("Connection refused".to_string()),
        };
        assert!(!result.reachable);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_ping_unreachable_host() {
        // Use a port that's unlikely to be listening
        let host = test_host("unreachable", 59999);
        let client = OllamaClient::new(500).unwrap();

        let result = client.ping_host(&host).await;
        assert!(!result.reachable);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_model_deserialization() {
        let json = r#"{
            "name": "qwen2.5-coder:7b",
            "modified_at": "2024-01-01T00:00:00Z",
            "size": 4000000000
        }"#;
        let model: OllamaModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.name, "qwen2.5-coder:7b");
        assert_eq!(model.size, Some(4000000000));
    }

    #[test]
    fn test_tags_response_deserialization() {
        let json = r#"{
            "models": [
                {"name": "model1"},
                {"name": "model2", "size": 1000}
            ]
        }"#;
        let resp: TagsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.models.len(), 2);
        assert_eq!(resp.models[0].name, "model1");
        assert_eq!(resp.models[1].name, "model2");
    }
}
