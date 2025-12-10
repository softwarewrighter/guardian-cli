# Guardian CLI Usage Guide

This document provides comprehensive usage instructions for Guardian CLI, including examples, configuration, and integration with development workflows.

## Quick Start

```bash
# Show config file location
guardian-cli config-path

# Create config directory
mkdir -p ~/.config/guardian-cli

# Create minimal config
cat > ~/.config/guardian-cli/guardian.toml << 'EOF'
[ollama]
default_timeout_ms = 2500

[[ollama.hosts]]
name = "local"
base_url = "http://localhost:11434"
enabled = true
fallback = true
EOF

# Check host availability
guardian-cli ping-hosts

# List available models
guardian-cli list-models
```

## Commands

### ping-hosts

Check availability of all configured Ollama hosts.

```bash
# Basic usage
guardian-cli ping-hosts

# JSON output (for scripting)
guardian-cli --json ping-hosts

# Verbose output
guardian-cli -v ping-hosts
```

**Example Output:**
```
Pinging 3 host(s)...

  [UP] big72 (25ms)
  [UP] curiosity (18ms)
  [DOWN] hive - Connection refused
  [UP] local (5ms) [fallback]

3/4 hosts reachable
```

### list-models

List models available on reachable Ollama hosts.

```bash
# List models on all hosts
guardian-cli list-models

# List models on specific host
guardian-cli list-models --host big72

# JSON output
guardian-cli --json list-models
```

**Example Output:**
```
big72 (http://big72:11434):
  - qwen2.5-coder:7b (4.2 GB)
  - llama3:8b (4.7 GB)
  - codellama:13b (7.3 GB)

curiosity (http://curiosity:11434):
  - phi3.5-mini (2.2 GB)
  - mistral:7b (4.1 GB)
```

### select-host

Select the best available host (useful for scripting).

```bash
# Select any available host
guardian-cli select-host

# Require a specific model
guardian-cli select-host --model qwen2.5-coder:7b

# JSON output
guardian-cli --json select-host
```

**Exit Codes:**
- 0: Host found and printed
- 1: No suitable host available

### show-config

Display current configuration.

```bash
guardian-cli show-config
guardian-cli --json show-config
```

### config-path

Show the default configuration file path.

```bash
guardian-cli config-path
```

## Configuration

### Config File Location

Default: `~/.config/guardian-cli/guardian.toml`

Override with `--config` flag:
```bash
guardian-cli --config /path/to/custom.toml ping-hosts
```

### Full Configuration Example

```toml
[ollama]
# Timeout for HTTP requests (milliseconds)
default_timeout_ms = 2500

# Default host to use when not specified
default_host = "big72"

# Default model for LLM operations
default_model = "qwen2.5-coder:7b"

# Remote LAN hosts (primary)
[[ollama.hosts]]
name = "big72"
base_url = "http://big72:11434"
enabled = true
fallback = false
description = "Main 72-core server in the rack"

[[ollama.hosts]]
name = "curiosity"
base_url = "http://curiosity:11434"
enabled = true
fallback = false
description = "Blackwell GPU box"

[[ollama.hosts]]
name = "hive"
base_url = "http://hive:11434"
enabled = false  # Currently offline
fallback = false
description = "Experimental node"

# Local fallback (used only when remote hosts unavailable)
[[ollama.hosts]]
name = "local"
base_url = "http://localhost:11434"
enabled = true
fallback = true
description = "Local Mac Ollama - fallback only"
```

### Host Configuration Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| name | string | yes | - | Human-readable identifier |
| base_url | string | yes | - | Ollama API URL |
| enabled | bool | no | true | Whether to include in operations |
| fallback | bool | no | false | Use only when primaries unavailable |
| description | string | no | - | Optional description |

## Development Workflow Integration

### Pre-Commit Checklist

Guardian CLI can be integrated into a development checklist workflow. Here's an example script:

```bash
#!/bin/bash
set -euo pipefail

echo "=== Development Checklist ==="

# 1. Verify Rust 2024 edition
echo "[1/7] Checking Rust edition..."
if ! grep -q 'edition = "2024"' Cargo.toml; then
    echo "ERROR: Must use Rust 2024 edition"
    exit 1
fi
echo "  OK: Rust 2024 edition"

# 2. Run tests (TDD - Red/Green)
echo "[2/7] Running tests..."
cargo test --quiet
echo "  OK: All tests pass"

# 3. Fix clippy warnings (do not disable!)
echo "[3/7] Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings
echo "  OK: No clippy warnings"

# 4. Format code
echo "[4/7] Formatting code..."
cargo fmt --all
echo "  OK: Code formatted"

# 5. Validate markdown
echo "[5/7] Checking markdown..."
if command -v markdown-checker &> /dev/null; then
    markdown-checker -f "**/*.md"
    echo "  OK: Markdown valid"
else
    echo "  SKIP: markdown-checker not installed"
fi

# 6. Run sw-checklist
echo "[6/7] Running sw-checklist..."
if command -v sw-checklist &> /dev/null; then
    sw-checklist
    echo "  OK: sw-checklist passed"
else
    echo "  SKIP: sw-checklist not installed"
fi

# 7. Verify Ollama hosts (optional)
echo "[7/7] Checking Ollama hosts..."
if command -v guardian-cli &> /dev/null; then
    guardian-cli ping-hosts
else
    echo "  SKIP: guardian-cli not installed"
fi

echo ""
echo "=== All checks passed! Ready to commit. ==="
```

### TDD (Test-Driven Development) Integration

Guardian CLI supports TDD workflows by:

1. **Running tests first** - Verify existing tests pass
2. **Validating test quality** - LLM can evaluate if tests are meaningful (future)
3. **Preventing placeholder tests** - Detect empty or trivial test implementations

Example TDD workflow:
```bash
# RED: Write failing test
cargo test -- test_new_feature 2>&1 | grep FAILED

# GREEN: Implement minimal code
# ... write code ...
cargo test -- test_new_feature

# REFACTOR: Improve while keeping tests green
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo test
```

### Scripting Examples

**Select host and use with ollama CLI:**
```bash
HOST=$(guardian-cli select-host --model qwen2.5-coder:7b)
ollama run qwen2.5-coder:7b --host "http://$HOST:11434" "Explain this code..."
```

**Check if any host is available:**
```bash
if guardian-cli select-host > /dev/null 2>&1; then
    echo "Ollama available"
else
    echo "No Ollama hosts available"
fi
```

**Get host info as JSON:**
```bash
guardian-cli --json ping-hosts | jq '.[] | select(.reachable == true)'
```

## Checklist Items for AI Coding Agents

When Guardian CLI is used by AI coding agents, it enforces these checklist items:

### Code Quality

- [ ] **Rust 2024 edition** - Cargo.toml must specify `edition = "2024"`
- [ ] **Zero clippy warnings** - Run `cargo clippy -- -D warnings`
- [ ] **Code formatted** - Run `cargo fmt --all`
- [ ] **No disabled warnings** - Never use `#[allow(...)]` to suppress
- [ ] **No dead code** - Remove unused functions/types

### Testing

- [ ] **TDD process** - Write failing test first (Red), then implement (Green)
- [ ] **Meaningful tests** - Tests must verify behavior, not just exist
- [ ] **No placeholder tests** - Empty tests or `assert!(true)` are rejected
- [ ] **All tests pass** - `cargo test` must succeed

### Documentation

- [ ] **ASCII-only markdown** - Use `markdown-checker` to validate
- [ ] **No binary characters** - Prevents GitHub preview issues
- [ ] **Doc comments** - Public APIs must have `///` documentation

### Process

- [ ] **Pre-commit checks** - All quality gates must pass before commit
- [ ] **Immediate push** - Push after each commit for backup
- [ ] **Clear commit messages** - Descriptive summary and details

## Troubleshooting

### "No hosts configured"

Create a configuration file:
```bash
mkdir -p ~/.config/guardian-cli
guardian-cli config-path  # Shows expected location
```

### "Connection refused" for all hosts

1. Verify Ollama is running on the target host
2. Check firewall rules allow port 11434
3. Verify hostname resolution: `ping big72`

### Timeout errors

Increase timeout in config:
```toml
[ollama]
default_timeout_ms = 5000  # 5 seconds
```

### Model not found

List available models to see what's installed:
```bash
guardian-cli list-models --host big72
```

Install missing models on the Ollama host:
```bash
ssh big72 "ollama pull qwen2.5-coder:7b"
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| RUST_LOG | Control log level (e.g., `RUST_LOG=debug`) |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | No suitable host available / command failed |
| 2 | Configuration error |

## Integration with Other Tools

### markdown-checker

Validates markdown files contain only ASCII characters:
```bash
markdown-checker -f "**/*.md"
```

### sw-checklist

Validates project meets Software Wrighter standards:
```bash
sw-checklist
```

### Pre-commit hooks

Add to `.git/hooks/pre-commit`:
```bash
#!/bin/bash
exec ./scripts/checklist.sh
```

## Future Commands (Planned)

### check-diff

Validate staged git changes against project rules:
```bash
guardian-cli check-diff [--model MODEL] [--strict]
```

### prepare-context

Generate distilled context for AI coding agents:
```bash
guardian-cli prepare-context --task task.json --output-dir guardian/
```

### warm-up

Keep Ollama models loaded in memory:
```bash
guardian-cli warm-up --host big72 --model qwen2.5-coder:7b
```
