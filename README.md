# Guardian CLI

A Rust CLI tool that acts as a "local governor" LLM, enforcing development process and project architecture rules while reducing token usage for cloud-based AI coding agents.

## Overview

Guardian CLI sits between your repository and cloud AI coding agents (Claude Code, Gemini CLI, etc.), providing:

- **Process enforcement** - Validate changes against architecture rules before commits
- **Token reduction** - Distill context so cloud agents receive only relevant information
- **Distributed LLM management** - Leverage multiple Ollama hosts on your LAN with automatic fallback

## Features (Planned)

- Poll and manage remote Ollama hosts
- Run configurable script checks (tests, linting)
- LLM-based diff evaluation against project rules
- Generate distilled context for cloud AI agents
- Pre-commit hook integration
- Metrics collection for validating the approach

## Installation

```bash
# Build from source
cargo build --release

# Install to ~/.local/softwarewrighter/bin/
sw-install -p .
```

## Usage

```bash
# Check which Ollama hosts are available
guardian-cli ping-hosts

# List models on reachable hosts
guardian-cli list-models

# Validate staged changes (coming soon)
guardian-cli check-diff

# Generate context for AI agent (coming soon)
guardian-cli prepare-context --task task.json
```

## Configuration

Create `~/.config/guardian-cli/guardian.toml`:

```toml
[ollama]
default_timeout_ms = 2500

[[ollama.hosts]]
name = "big72"
base_url = "http://big72:11434"
enabled = true
fallback = false

[[ollama.hosts]]
name = "local"
base_url = "http://localhost:11434"
enabled = true
fallback = true
```

## Documentation

- [Usage Guide](docs/usage.md) - Comprehensive usage instructions and examples
- [Architecture](docs/architecture.md) - System architecture and component design
- [PRD](docs/prd.md) - Product requirements document
- [Design](docs/design.md) - Technical design and API details
- [Plan](docs/plan.md) - Implementation plan and milestones
- [Status](docs/status.md) - Current project status

### Development

- [Process](docs/process.md) - Development workflow and quality gates
- [Tools](docs/tools.md) - Recommended development tools
- [AI Agent Instructions](docs/ai_agent_instructions.md) - Guidelines for AI coding agents

## License

MIT License - See [LICENSE](LICENSE) for details.

Copyright (c) 2025 Michael A Wright
