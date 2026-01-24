# Ouroboros

A recursive Claude pipeline that processes tasks through specialized AI roles with automatic retry, validation, and self-correction.

## Overview

Ouroboros orchestrates multiple Claude instances working together in distinct roles to solve tasks. Each task goes through outlining, review, planning, execution, and validation cycles. The system automatically classifies issues and retries with context from failed attempts.

## Pipeline Flow

```
┌─────────────────────────────────────────┐
│              Task Input                 │
│           (tasks/task-x.md)             │
└──────────────────┬──────────────────────┘
                   │
┌──────────────────▼──────────────────┐
│           1. OUTLINER               │
│     Creates high-level outline      │
└──────────────────┬──────────────────┘
                   │
┌──────────────────▼──────────────────┐
│           2. ADVISOR                │
│   Reviews & provides feedback       │
│        → advise-x-y.md              │
└──────────────────┬──────────────────┘
                   │
┌──────────────────▼──────────────────┐
│           3. PLANNER                │
│  Creates detailed plan from         │
│  outline and advice                 │
│        → plan-x-y.md                │
└──────────────────┬──────────────────┘
                   │
┌──────────────────▼──────────────────┐
│           4. ACTOR                  │
│     Executes the plan               │
│        → result-x-y.md              │
│        → how-x-y.md                 │
└──────────────────┬──────────────────┘
                   │
┌──────────────────▼──────────────────┐
│           5. CHECKER                │
│   Validates N times (parallel)      │
│        → check-x-y-n.md             │
│   Classifies issues: MINOR/MAJOR    │
└──────────────────┬──────────────────┘
                   │
     ┌─────────────┼─────────────┐
     │             │             │
┌────▼────┐  ┌─────▼─────┐  ┌────▼────┐
│ passes  │  │  MINOR    │  │  MAJOR  │
│   ≥ N   │  │  issues   │  │  only   │
│         │  │  exist    │  │         │
│  next   │  │           │  │  retry  │
│  task   │  │  FIXER    │  │  with   │
│         │  │  runs     │  │ context │
└─────────┘  └─────┬─────┘  └─────────┘
                   │
       ┌───────────▼───────────┐
       │       6. FIXER        │
       │  Fixes minor issues   │
       │     → fix-x-y.md      │
       │                       │
       │  Re-validates fixes   │
       │  → check-x-y-n-verified.md
       └───────────┬───────────┘
                   │
          ┌────────┴────────┐
          │                 │
     ┌────▼────┐       ┌────▼────┐
     │ verify  │       │ verify  │
     │ passes  │       │ fails   │
     │         │       │         │
     │  next   │       │  retry  │
     │  task   │       │  with   │
     │         │       │ context │
     └─────────┘       └─────────┘
```

## Roles

| Role | Purpose |
|------|---------|
| **Outliner** | Creates a high-level outline of the approach |
| **Advisor** | Reviews the outline and provides critical feedback |
| **Planner** | Creates a detailed plan incorporating outline and advice |
| **Actor** | Executes the plan, produces `how` and `result` files |
| **Checker** | Validates output, classifies issues as MINOR or MAJOR |
| **Fixer** | Fixes minor issues (formatting, style, typos) |

## Issue Classification

| Type | Examples |
|------|----------|
| **MINOR** | Formatting, style, missing comments/docs, typos, cosmetic issues |
| **MAJOR** | Missing functionality, broken logic, security issues, core requirements not met |

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- `claude` CLI installed and accessible in PATH

### Build

```bash
cargo build --release
```

## Usage

```bash
# Run with defaults (uses ./config.json and ./secrets.json)
cargo run

# Show all options
cargo run -- --help

# Custom configuration
cargo run -- -c ./example/config.json

# Specify both config and secrets
cargo run -- -c ./example/config.json -s ./example/secrets.json
```

## Configuration

The pipeline is configured via a JSON file. By default, it looks for `config.json` in the current working directory.

```bash
# Uses ./config.json in current directory
./ouroboros

# Specify a custom config path
./ouroboros -c /path/to/config.json
```

### Example config.json

Each role can use a different CLI and provider combination:

```json
{
  "splitter": { "cli": "claude-code", "model": "opus", "provider": "anthropic" },
  "outliner": { "cli": "claude-code", "model": "MiniMax-M2.1", "provider": "minimax" },
  "advisor": { "cli": "claude-code", "model": "MiniMax-M2.1", "provider": "minimax" },
  "planner": { "cli": "claude-code", "model": "opus", "provider": "anthropic" },
  "actor": { "cli": "claude-code", "model": "opus", "provider": "anthropic" },
  "checker": { "cli": "claude-code", "model": "MiniMax-M2.1", "provider": "minimax" },
  "minor_fixer": { "cli": "codex", "model": "gpt-5.2-codex" },
  "major_fixer": { "cli": "claude-code", "model": "opus", "provider": "anthropic" },

  "tasks_dir": "./tasks",
  "results_dir": "./results",
  "plans_dir": "./plans",
  "advises_dir": "./advises",
  "checks_dir": "./checks",
  "hows_dir": "./hows",
  "outlines_dir": "./outlines",
  "fixes_dir": "./fixes",
  "rechecks_dir": "./rechecks",

  "checks": 3,
  "threshold": 3,
  "recheck_threshold": 3,
  "max_retries": 4
}
```

#### Role Configuration Fields

| Field | Required | Description |
|-------|----------|-------------|
| `cli` | Yes | CLI to use: `claude-code`, `codex`, or `opencode` |
| `model` | Yes | Model name (e.g., `opus`, `sonnet`, `MiniMax-M2.1`, `gpt-5.2-codex`) |
| `provider` | No | API provider for `claude-code` only: `anthropic` (default) or `minimax` |

**Note:** The `provider` field is only used with `claude-code` CLI. For `codex` and `opencode`, omit it.

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `tasks_dir` | `./tasks` | Directory containing task files |
| `results_dir` | `./results` | Directory for result files |
| `plans_dir` | `./plans` | Directory for plan files |
| `advises_dir` | `./advises` | Directory for advice files |
| `checks_dir` | `./checks` | Directory for check result files |
| `hows_dir` | `./hows` | Directory for how files |
| `outlines_dir` | `./outlines` | Directory for outline files |
| `fixes_dir` | `./fixes` | Directory for fix files |
| `checks` | `5` | Number of validation runs per task |
| `threshold` | `4` | Minimum passes required |
| `max_retries` | `3` | Maximum retry attempts |

### Supported CLIs

| CLI | Description |
|-----|-------------|
| `claude-code` | Claude Code CLI (supports multiple providers) |
| `codex` | OpenAI Codex CLI |
| `opencode` | OpenCode CLI |

### Supported Providers (for claude-code)

| Provider | Description |
|----------|-------------|
| `anthropic` | Default Anthropic API |
| `minimax` | MiniMax M2.1 via Anthropic-compatible API |

### secrets.json

To use non-default providers (e.g., MiniMax), create a `secrets.json` file:

```bash
# Secrets path priority:
# 1. CLI option: -s ./path/to/secrets.json
# 2. Same directory as config.json
# 3. Current working directory: ./secrets.json
```

```json
{
  "minimax": {
    "api_key": "your-minimax-api-key-here",
    "base_url": "https://api.minimax.io/anthropic"
  },
  "anthropic": {
    "api_key": ""
  }
}
```

| Field | Description |
|-------|-------------|
| `minimax.api_key` | Your MiniMax API key from [platform.minimax.io](https://platform.minimax.io) |
| `minimax.base_url` | MiniMax API endpoint (default: `https://api.minimax.io/anthropic`) |
| `anthropic.api_key` | Optional. Leave empty to use existing Claude Code authentication |

**Important:** Add `secrets.json` to your `.gitignore` to avoid committing API keys.

See `example/secrets.json` for a template.

## Directory Structure

```
ouroboros/
├── tasks/           # Input: task definitions
│   └── task-1.md
├── outlines/        # Outliner output
│   └── outline-1-1.md
├── advises/         # Advisor feedback
│   └── advise-1-1.md
├── plans/           # Planner output
│   └── plan-1-1.md
├── results/         # Actor results
│   └── result-1-1.md
├── hows/            # Actor approach details
│   └── how-1-1.md
├── checks/          # Checker validations
│   └── check-1-1-1.md
├── fixes/           # Fixer output
│   └── fix-1-1.md
└── src/
    ├── main.rs
    ├── config.rs
    ├── agent.rs
    ├── pipeline.rs
    └── roles/
        ├── outliner.rs
        ├── advisor.rs
        ├── planner.rs
        ├── actor.rs
        ├── checker.rs
        ├── fixer.rs
        └── splitter.rs
```

### File Naming Convention

| Directory | Pattern | Description |
|-----------|---------|-------------|
| `tasks/` | `task-{x}.md` | Task number |
| `outlines/` | `outline-{x}-{y}.md` | Task and attempt number |
| `advises/` | `advise-{x}-{y}.md` | Task and attempt number |
| `plans/` | `plan-{x}-{y}.md` | Task and attempt number |
| `results/` | `result-{x}-{y}.md` | Task and attempt number |
| `hows/` | `how-{x}-{y}.md` | Task and attempt number |
| `checks/` | `check-{x}-{y}-{n}.md` | Task, attempt, and check number |
| `fixes/` | `fix-{x}-{y}.md` | Task and attempt number |

## Creating Tasks

Create markdown files in the `tasks/` directory following the naming convention `task-N.md`:

```markdown
# Task 1: Example Task

Describe what needs to be accomplished.

## Requirements
- Requirement 1
- Requirement 2

## Constraints
- Any constraints or limitations
```

Tasks are processed in numerical order (task-1, task-2, etc.).

## Cross-Task Context

For sequential tasks, the pipeline automatically passes context:

- For task N (where N > 1), the pipeline loads `result-(N-1)-{highest}.md` and `how-(N-1)-{highest}.md`
- `{highest}` is the highest attempt number found for that task
- This ensures each task only sees context from the immediately preceding task

## How Retries Work

When a task fails validation:

1. Failed content is preserved with the attempt number
2. On retry, Planner and Advisor receive the failed output as context
3. The pipeline learns from mistakes and tries a different approach
4. If all retries exhausted, the last attempt is saved and failure is reported

## License

MIT
