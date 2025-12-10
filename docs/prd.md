# Guardian CLI - Product Requirements Document

## Problem Statement

Cloud-based AI coding agents (Claude Code, Gemini CLI, opencode, Codex) consume large amounts of tokens when provided full repository context, chat history, and documentation. This leads to:

1. **High costs** - Token usage scales with context size
2. **Quality issues** - Agents may introduce architecture violations or process errors
3. **Slow feedback** - Issues discovered only after commits are pushed
4. **Context pollution** - Irrelevant information dilutes focus on the actual task

## Solution

Guardian CLI is a local "governor" LLM tool that:

1. **Enforces process and architecture** before mistakes are committed
2. **Reduces token usage** by distilling context for cloud agents
3. **Provides fast feedback** through local LLM evaluation
4. **Manages LLM infrastructure** across distributed home lab servers

## Target Users

- Solo developers using AI coding agents
- Developers with home lab LAN environments (multiple Ollama hosts)
- Teams wanting to enforce coding standards via automated pre-commit checks

## Goals

### Primary Goals

1. **Prevent bad commits** - Block commits that violate architecture rules or coding standards
2. **Reduce cloud token usage** - Provide distilled context instead of full repository
3. **Leverage local LLM capacity** - Use home lab servers for heavy context processing

### Secondary Goals

1. **Enable experimentation** - Collect metrics to validate the approach
2. **Provide fast feedback** - Local LLM checks should complete in seconds
3. **Support offline development** - Fallback to local Ollama when remote hosts unavailable

## Non-Goals

- Replacing cloud AI agents entirely
- Building a full CI/CD system
- Supporting non-Ollama LLM backends (initially)

## Requirements

### Functional Requirements

#### FR-1: Host Management

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1.1 | Poll configured Ollama hosts for availability | P0 |
| FR-1.2 | List models available on each reachable host | P0 |
| FR-1.3 | Select best host based on availability and capability | P0 |
| FR-1.4 | Fallback to local Ollama when remote hosts unavailable | P0 |
| FR-1.5 | Configure host priority and preferences via TOML | P1 |

#### FR-2: Repository Analysis

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-2.1 | Extract staged git diff | P0 |
| FR-2.2 | List files in repository with filtering | P1 |
| FR-2.3 | Read file contents for LLM context | P1 |
| FR-2.4 | Detect project type (Rust, Node, etc.) | P2 |

#### FR-3: Scriptable Checks

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-3.1 | Run configurable shell commands (tests, linting) | P0 |
| FR-3.2 | Capture stdout/stderr and exit codes | P0 |
| FR-3.3 | Summarize script results for LLM consumption | P1 |
| FR-3.4 | Support timeouts for long-running commands | P1 |

#### FR-4: LLM-Based Evaluation

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-4.1 | Send structured prompts to Ollama API | P0 |
| FR-4.2 | Parse structured JSON responses from LLM | P0 |
| FR-4.3 | Evaluate diffs against architecture rules | P0 |
| FR-4.4 | Suggest relevant files for task context | P1 |
| FR-4.5 | Summarize code changes for human review | P2 |

#### FR-5: Report Generation

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-5.1 | Output machine-readable JSON reports | P0 |
| FR-5.2 | Output human-readable markdown reports | P1 |
| FR-5.3 | Include pass/fail status with reasons | P0 |
| FR-5.4 | Include suggested context files for agents | P1 |

#### FR-6: Pre-Commit Integration

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-6.1 | Exit non-zero on blocking violations | P0 |
| FR-6.2 | Allow warnings without blocking | P1 |
| FR-6.3 | Provide clear guidance on fixing issues | P1 |

### Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1 | Host ping latency | < 3 seconds timeout |
| NFR-2 | Script check execution | < 60 seconds typical |
| NFR-3 | LLM evaluation latency | < 10 seconds for small diffs |
| NFR-4 | Configuration file format | TOML |
| NFR-5 | Output formats | JSON, Markdown |
| NFR-6 | Platform support | macOS (primary), Linux |

## Success Metrics

### Token Reduction

- **Baseline**: Measure average tokens per task without guardian
- **Target**: 50%+ reduction in cloud agent tokens per task

### Error Prevention

- **Baseline**: Track CI failures and post-commit fixes
- **Target**: 80%+ of architecture violations caught pre-commit

### Performance

- **Target**: < 30 seconds for complete guardian workflow
- **Target**: < 3 seconds for host availability check

## User Stories

### US-1: Check Host Availability

> As a developer, I want to see which Ollama hosts are available so I know where my LLM requests will be processed.

**Acceptance Criteria:**
- Run `guardian-cli ping-hosts`
- See list of configured hosts with up/down status
- Fallback host clearly identified

### US-2: List Available Models

> As a developer, I want to see what models are available across my hosts so I can choose the right one for my task.

**Acceptance Criteria:**
- Run `guardian-cli list-models`
- See models grouped by host
- Handle unreachable hosts gracefully

### US-3: Validate Diff Before Commit

> As a developer, I want my staged changes validated against project rules before committing so I catch mistakes early.

**Acceptance Criteria:**
- Run `guardian-cli check-diff`
- See pass/fail status with specific reasons
- Get suggestions for fixing violations
- Command returns non-zero on blocking issues

### US-4: Generate Task Context

> As a developer using an AI coding agent, I want minimal but sufficient context generated so the agent works efficiently without excessive token usage.

**Acceptance Criteria:**
- Run `guardian-cli prepare-context --task task.json`
- Output contains only relevant files and rules
- Output is in format consumable by AI agents

## Open Questions

1. Should guardian-cli support multiple concurrent tasks?
2. What model sizes are optimal for different check types?
3. How should we handle partial LAN availability (some hosts up, some down)?
4. Should we support model warm-up/keep-alive functionality?

## Future Considerations

- Support for additional LLM backends (OpenAI, Anthropic)
- IDE/editor integration
- Team-shared rules and policies
- Metrics dashboard for tracking effectiveness
