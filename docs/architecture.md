# Guardian CLI Architecture

## Overview

Guardian CLI is a Rust command-line tool that acts as a "local governor" LLM, sitting between your repository and cloud-based AI coding agents. It enforces development process and project architecture rules while reducing token usage for expensive cloud API calls.

## System Architecture

```
+------------------+     +------------------+     +------------------+
|  Cloud AI Agent  |     |   Guardian CLI   |     |  Remote Ollama   |
|  (Claude Code,   |<--->|   (Rust CLI)     |<--->|  Servers (LAN)   |
|   Gemini CLI)    |     |                  |     |  big72, curiosity|
+------------------+     +------------------+     +------------------+
                                  |                       |
                                  v                       v
                         +------------------+     +------------------+
                         |   Local Repo     |     |  Local Ollama    |
                         |   (git, files)   |     |  (Fallback)      |
                         +------------------+     +------------------+
```

## Component Architecture

### 1. Local Guardian Layer

**guardian-cli (this project)**
- Rust CLI application using async runtime (tokio)
- HTTP client (reqwest) for Ollama API communication
- Configuration via TOML files
- Git integration for diff analysis

**Inputs:**
- Configuration file (guardian.toml) with:
  - Ollama endpoints and host priority
  - Project rules and policies
  - Script commands for automated checks
- Task context from AI coding agent
- Repository state (diffs, file tree, docs)

**Outputs:**
- Machine-readable JSON reports
- Human-readable markdown summaries
- Pass/fail status for pre-commit gates

### 2. Ollama Server Layer

**Remote Hosts (Primary)**
- big72, curiosity, hive - Arch Linux systems on home LAN
- Not always powered on - requires availability polling
- Host larger models (7B-72B parameters)

**Local Host (Fallback)**
- Mac localhost:11434
- Used only when all remote hosts are unavailable
- Minimal model footprint to save disk space

### 3. Orchestration Layer

Thin orchestration (bash/Rust) that:
- Runs guardian-cli before agent tasks
- Runs guardian-cli as pre-commit gate
- Passes guardian output to cloud AI agents
- Manages agent session lifecycle

## Data Flow

### Task Execution Flow

```
1. Developer defines task -> task_001.json
2. Orchestrator calls guardian-cli prepare-context
3. Guardian inspects repo (git status, diff, files)
4. Guardian runs scriptable checks (cargo test, clippy)
5. Guardian queries Ollama for context selection + rule checking
6. Guardian outputs:
   - guardian/task_001_context.json (distilled context)
   - guardian/task_001_report.md (human summary)
7. Cloud agent starts fresh session with minimal context
8. Agent completes task
9. Guardian validates changes before commit
```

### Pre-Commit Flow

```
1. git commit triggers pre-commit hook
2. guardian-cli check-diff extracts staged changes
3. Runs scriptable checks + LLM evaluation
4. If violations detected -> exit non-zero, block commit
5. If warnings only -> allow commit, log warnings
6. If pass -> commit proceeds
```

## Module Structure

```
guardian-cli/
  src/
    main.rs           # CLI entrypoint, clap commands
    config.rs         # TOML config loading
    ollama.rs         # Ollama HTTP client
    host_select.rs    # Host availability + selection
    repo.rs           # Git operations (status, diff)
    checks/
      mod.rs
      scripts.rs      # Test/lint runners
      policies.rs     # Static rule checking
      llm_checks.rs   # LLM-based evaluation
    report.rs         # JSON/markdown output
```

## Key Design Decisions

### 1. Remote-Preferred, Local-Fallback

Remote LAN hosts are preferred for model capacity and to keep local disk clean. Local Ollama is only used when all remote hosts are unreachable.

### 2. Task Capsules

Each cloud agent invocation receives a minimal "task capsule" instead of full repository context:
- Task description
- Guardian-generated context (relevant files only)
- Distilled architecture rules

This dramatically reduces token usage.

### 3. Async Concurrent Host Checking

All remote hosts are pinged concurrently with short timeouts (2-3 seconds). First responding host is selected for the task.

### 4. Structured Output

All guardian outputs use structured JSON for machine consumption plus optional markdown for human review. This enables automation and debugging.

## Security Considerations

- Local LLM keeps code on-premises for heavy context work
- Cloud agent sees only summaries/distilled context
- No sensitive data in task capsules unless necessary
- API keys for remote services stored in environment variables

## Performance Requirements

- Host ping timeout: 2-3 seconds max
- Script checks: seconds, not minutes
- LLM evaluation: fast local models (1B-8B) for routine checks
- Full workflow: under 30 seconds for typical tasks
