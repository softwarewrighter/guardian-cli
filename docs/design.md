# Guardian CLI - Technical Design Document

## Overview

This document details the technical design decisions for Guardian CLI, a Rust-based tool for enforcing development process and architecture via local LLM evaluation.

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust 2024 edition | Performance, safety, existing tooling expertise |
| Async Runtime | tokio | Industry standard for async Rust |
| HTTP Client | reqwest + rustls | Robust HTTP client, pure Rust TLS |
| CLI Framework | clap (derive) | Ergonomic CLI definition |
| Config Format | TOML | Human-readable, Rust ecosystem standard |
| Serialization | serde + serde_json | De facto Rust serialization |
| Error Handling | anyhow + thiserror | Ergonomic error handling |
| Logging | tracing | Structured, async-friendly logging |

## Module Design

### Core Modules

#### config.rs

Handles configuration loading from TOML files.

```rust
pub struct GuardianConfig {
    pub ollama: OllamaSection,
    pub policies: Option<PoliciesSection>,
    pub scripts: Option<ScriptsSection>,
    pub paths: Option<PathsSection>,
}

pub struct OllamaHost {
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub fallback: bool,
    pub description: Option<String>,
}
```

**Config Location:**
- Default: `~/.config/guardian-cli/guardian.toml`
- Override: `--config <path>` flag

#### ollama.rs

HTTP client for Ollama API communication.

**Key Functions:**
```rust
impl OllamaClient {
    pub fn new(timeout_ms: u64) -> Result<Self>;
    pub async fn ping_host(&self, host: &OllamaHost) -> Result<bool>;
    pub async fn list_models(&self, host: &OllamaHost) -> Result<Vec<OllamaTagModel>>;
    pub async fn generate(&self, host: &OllamaHost, prompt: &str, model: &str) -> Result<String>;
    pub async fn chat(&self, host: &OllamaHost, messages: &[Message], model: &str) -> Result<String>;
}
```

**API Endpoints Used:**
- `GET /api/tags` - List available models
- `POST /api/generate` - Text generation
- `POST /api/chat` - Chat completion

#### host_select.rs

Implements host selection strategy.

```rust
pub async fn select_best_host(cfg: &GuardianConfig) -> Result<Option<OllamaHost>>;
pub async fn ping_all_hosts(cfg: &GuardianConfig) -> Result<Vec<(OllamaHost, bool)>>;
```

**Selection Algorithm:**
1. Filter to enabled hosts only
2. Separate into primary (fallback=false) and fallback (fallback=true) lists
3. Ping primary hosts concurrently
4. Return first responding primary host
5. If no primaries respond, try fallback hosts
6. Return None if all hosts unreachable

#### repo.rs

Git repository operations.

```rust
pub fn get_staged_diff() -> Result<String>;
pub fn get_status() -> Result<GitStatus>;
pub fn list_files(include: &[&str], exclude: &[&str]) -> Result<Vec<PathBuf>>;
pub fn read_files(paths: &[PathBuf]) -> Result<HashMap<PathBuf, String>>;
```

**Implementation:**
- Shell out to `git` commands for reliability
- Parse output for structured data
- Handle non-git directories gracefully

### Checks Subsystem

#### checks/scripts.rs

Runs configurable shell commands.

```rust
pub struct ScriptResult {
    pub script: String,
    pub status: ScriptStatus,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

pub async fn run_script(command: &str, timeout_ms: u64) -> Result<ScriptResult>;
pub async fn run_scripts(commands: &[String], timeout_ms: u64) -> Result<Vec<ScriptResult>>;
```

**Configured Scripts:**
```toml
[scripts]
pre_analysis = ["cargo fmt --check", "cargo clippy -D warnings"]
pre_commit = ["cargo test -q"]
```

#### checks/policies.rs

Static rule checking without LLM.

```rust
pub struct PolicyViolation {
    pub rule: String,
    pub severity: Severity,
    pub location: Option<String>,
    pub message: String,
}

pub fn check_policies(diff: &str, rules: &[PolicyRule]) -> Vec<PolicyViolation>;
```

**Policy Types:**
- File size limits
- Forbidden patterns (regex)
- Required patterns
- Path restrictions

#### checks/llm_checks.rs

LLM-based evaluation of changes.

```rust
pub struct LlmCheckResult {
    pub ok_to_proceed: bool,
    pub severity: Severity,
    pub reasons: Vec<String>,
    pub file_context_suggestions: Vec<String>,
}

pub async fn evaluate_diff(
    client: &OllamaClient,
    host: &OllamaHost,
    model: &str,
    diff: &str,
    rules: &str,
    task: Option<&str>,
) -> Result<LlmCheckResult>;
```

**Prompt Structure:**
```
System: You are a strict code reviewer enforcing the repo policies...
User:
<rules>
{rules.md content}
</rules>

<task>
{task description if provided}
</task>

<diff>
{git diff}
</diff>

Respond with JSON only:
{
  "ok_to_proceed": boolean,
  "severity": "low" | "medium" | "high",
  "reasons": ["..."],
  "file_context_suggestions": ["path/to/file.rs", ...]
}
```

### Output Module

#### report.rs

Generates structured output.

```rust
pub struct GuardianReport {
    pub task_id: Option<String>,
    pub timestamp: String,
    pub host_used: String,
    pub model_used: String,
    pub script_results: Vec<ScriptResult>,
    pub policy_violations: Vec<PolicyViolation>,
    pub llm_evaluation: Option<LlmCheckResult>,
    pub overall_status: OverallStatus,
}

pub fn write_json_report(report: &GuardianReport, path: &Path) -> Result<()>;
pub fn write_markdown_report(report: &GuardianReport, path: &Path) -> Result<()>;
```

## CLI Commands

### ping-hosts

```
guardian-cli ping-hosts [--json]
```

Pings all enabled hosts and reports availability.

### list-models

```
guardian-cli list-models [--host <name>] [--json]
```

Lists models on reachable hosts.

### check-diff

```
guardian-cli check-diff [--model <name>] [--json] [--strict]
```

Validates staged changes against rules.

**Exit Codes:**
- 0: All checks passed
- 1: Warnings only (non-strict mode)
- 2: Blocking violations found
- 3: Configuration or runtime error

### prepare-context

```
guardian-cli prepare-context --task <file.json> [--output-dir <path>]
```

Generates distilled context for cloud AI agent.

**Output Files:**
- `guardian/task_XXX_context.json` - Relevant files and content
- `guardian/task_XXX_report.md` - Human summary

### warm-up

```
guardian-cli warm-up [--host <name>] [--model <name>]
```

Sends a simple prompt to keep model loaded in memory.

## Configuration Schema

```toml
[ollama]
default_timeout_ms = 2500
default_host = "big72"
default_model = "qwen2.5-coder:7b"

[[ollama.hosts]]
name = "big72"
base_url = "http://big72:11434"
enabled = true
fallback = false
description = "Main 72-core box"

[[ollama.hosts]]
name = "local"
base_url = "http://localhost:11434"
enabled = true
fallback = true
description = "Local fallback"

[policies]
rules_file = "guardian/rules.md"
max_file_size_kb = 500
max_function_lines = 50

[scripts]
timeout_ms = 60000
pre_analysis = ["cargo fmt --check", "cargo clippy -D warnings"]
pre_commit = ["cargo test -q"]

[paths]
include = ["src", "backend", "frontend"]
exclude = ["target", "node_modules", ".git"]
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum GuardianError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("No Ollama hosts available")]
    NoHostsAvailable,

    #[error("Ollama API error: {0}")]
    OllamaApi(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Script execution failed: {0}")]
    Script(String),

    #[error("LLM response parse error: {0}")]
    LlmParse(String),
}
```

### Fallback Behavior

| Scenario | Behavior |
|----------|----------|
| All remote hosts down | Try local fallback |
| Local also down | Exit with clear error |
| LLM returns malformed JSON | Retry once, then fail open with warning |
| Script times out | Report timeout, continue with other checks |
| Config file missing | Use sensible defaults + warning |

## Testing Strategy

### Unit Tests

- Config parsing with various TOML inputs
- Policy rule matching
- JSON response parsing

### Integration Tests

- Mock HTTP server for Ollama API
- Git repository fixtures
- End-to-end command execution

### Manual Testing

- Real Ollama server interactions
- Pre-commit hook installation
- Performance benchmarking

## Metrics Collection

For validating the approach:

```rust
pub struct TaskMetrics {
    pub task_id: String,
    pub timestamp: String,
    pub local_tokens_in: u64,
    pub local_tokens_out: u64,
    pub guardian_duration_ms: u64,
    pub host_used: String,
    pub model_used: String,
    pub scripts_run: u32,
    pub check_passed: bool,
}
```

Metrics logged to: `guardian/logs/YYYY-MM-DD.jsonl`

## Security Considerations

1. **No secrets in config** - API keys via environment variables only
2. **Local-first processing** - Sensitive code stays on LAN
3. **Minimal cloud exposure** - Only distilled context sent to cloud agents
4. **TLS for remote** - Use HTTPS for any internet-facing endpoints
5. **Sandboxed scripts** - Consider timeout and resource limits

## Future Extensions

1. **Model warm-up daemon** - Keep models loaded with periodic pings
2. **Embedding-based search** - Semantic file selection for context
3. **Multi-model routing** - Different models for different check types
4. **Cached summaries** - Avoid re-summarizing unchanged files
5. **IDE integration** - VS Code extension for real-time feedback
