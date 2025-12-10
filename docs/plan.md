# Guardian CLI - Implementation Plan

## Overview

This document outlines the phased implementation plan for Guardian CLI. The approach is incremental, with each phase delivering usable functionality.

## Phase 1: MVP - Host Management

**Goal:** Basic CLI that can discover and communicate with Ollama hosts.

### Tasks

- [ ] Set up project structure with proper module layout
- [ ] Implement config.rs - TOML configuration loading
- [ ] Implement ollama.rs - Basic Ollama HTTP client
- [ ] Implement host_select.rs - Host selection with fallback
- [ ] Implement ping-hosts command
- [ ] Implement list-models command
- [ ] Add comprehensive error handling
- [ ] Write unit tests for config parsing
- [ ] Write integration tests with mock server

### Dependencies

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
directories = "5"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1"
```

### Deliverables

- Working `guardian-cli ping-hosts` command
- Working `guardian-cli list-models` command
- Configuration via `~/.config/guardian-cli/guardian.toml`
- Documentation for setup and usage

---

## Phase 2: Git Integration

**Goal:** Add repository analysis capabilities.

### Tasks

- [ ] Implement repo.rs - Git operations module
- [ ] Add get_staged_diff() function
- [ ] Add get_status() function
- [ ] Add list_files() with include/exclude filters
- [ ] Add read_files() for content loading
- [ ] Create check-diff command skeleton
- [ ] Add --json output flag support
- [ ] Write tests with git repository fixtures

### Deliverables

- Working `guardian-cli check-diff` (outputs diff only, no LLM yet)
- Git status and diff extraction
- File listing with filtering

---

## Phase 3: Script Checks

**Goal:** Run configurable shell commands as part of validation.

### Tasks

- [ ] Implement checks/scripts.rs
- [ ] Add script execution with timeout
- [ ] Capture stdout/stderr/exit code
- [ ] Add ScriptResult struct for structured output
- [ ] Configure scripts via guardian.toml
- [ ] Integrate with check-diff command
- [ ] Add pre-commit hook installation docs

### Deliverables

- Configurable script execution
- Script results in check-diff output
- Pre-commit integration guide

---

## Phase 4: LLM Evaluation

**Goal:** Use local LLM to evaluate changes against rules.

### Tasks

- [ ] Implement checks/llm_checks.rs
- [ ] Add Ollama generate/chat API support
- [ ] Design structured prompt templates
- [ ] Implement JSON response parsing
- [ ] Add rules.md loading from config
- [ ] Integrate LLM evaluation into check-diff
- [ ] Add severity levels (low/medium/high)
- [ ] Implement blocking vs warning logic

### Deliverables

- LLM-based diff evaluation
- Structured violation reports
- Pass/fail with reasons

---

## Phase 5: Report Generation

**Goal:** Generate machine and human-readable reports.

### Tasks

- [ ] Implement report.rs
- [ ] Create GuardianReport struct
- [ ] Add JSON report generation
- [ ] Add Markdown report generation
- [ ] Add output directory configuration
- [ ] Create report templates
- [ ] Add timestamp and metadata

### Deliverables

- JSON reports for automation
- Markdown reports for humans
- Configurable output locations

---

## Phase 6: Context Preparation

**Goal:** Generate distilled context for cloud AI agents.

### Tasks

- [ ] Implement prepare-context command
- [ ] Add task.json input parsing
- [ ] Implement file relevance scoring (LLM-based)
- [ ] Generate context capsules
- [ ] Add context size limits
- [ ] Document integration with Claude Code/Gemini CLI

### Deliverables

- Working `guardian-cli prepare-context --task task.json`
- Context capsule generation
- Integration documentation

---

## Phase 7: Metrics and Observability

**Goal:** Enable experimentation and validation of the approach.

### Tasks

- [ ] Implement metrics collection
- [ ] Add JSONL logging for metrics
- [ ] Track token usage (input/output)
- [ ] Track timing for each operation
- [ ] Add --usage-report command
- [ ] Create baseline comparison tooling

### Deliverables

- Metrics logging to `guardian/logs/`
- Usage reporting command
- Documentation for A/B testing

---

## Phase 8: Polish and Production Readiness

**Goal:** Production-quality tool ready for daily use.

### Tasks

- [ ] Add warm-up command for keeping models hot
- [ ] Implement graceful degradation
- [ ] Add shell completion generation
- [ ] Improve error messages
- [ ] Performance optimization
- [ ] Security review
- [ ] Comprehensive documentation
- [ ] Create release artifacts

### Deliverables

- Polished CLI experience
- Complete documentation
- Release builds for macOS/Linux

---

## Milestone Summary

| Phase | Name | Key Deliverable |
|-------|------|-----------------|
| 1 | MVP | ping-hosts, list-models |
| 2 | Git Integration | check-diff skeleton |
| 3 | Script Checks | Configurable script execution |
| 4 | LLM Evaluation | LLM-based diff review |
| 5 | Reports | JSON/Markdown output |
| 6 | Context Prep | prepare-context command |
| 7 | Metrics | Usage tracking and reporting |
| 8 | Polish | Production-ready release |

## Current Focus

**Phase 1: MVP - Host Management**

Starting with the foundation: configuration loading, Ollama client, and basic host commands.

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Remote hosts often offline | Local fallback configured by default |
| LLM responses unparseable | Retry once, then fail open with warning |
| Large diffs overwhelm context | Truncate with summary, or split into chunks |
| Script timeouts | Configurable timeout with sensible defaults |

## Success Criteria

Phase is complete when:
1. All tasks checked off
2. Tests passing
3. Documentation updated
4. Dogfooding successful (using guardian on itself)
