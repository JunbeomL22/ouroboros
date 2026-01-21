# Ouroboros

A recursive Claude pipeline that processes tasks sequentially through four specialized AI roles, with automatic retry and validation.

## Overview

Ouroboros orchestrates multiple Claude instances working together in distinct roles to solve tasks. Each task goes through a planning, review, execution, and validation cycle. If validation fails, the system automatically retries with context from the failed attempt.

## Pipeline Flow

```
                    ┌─────────────────────────────────────────┐
                    │              Task Input                 │
                    │         (tasks/task-x.md)               │
                    └──────────────┬──────────────────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      1. PLANNER             │
                    │   Creates initial plan      │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      2. ADVISOR             │
                    │   Reviews & gives feedback  │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      3. PLANNER             │
                    │   Revises based on feedback │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      4. ACTOR               │
                    │   Executes the plan         │
                    │   → writes how-x.md         │
                    │   → writes result-x.md      │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      5. CHECKER             │
                    │   Validates N times         │
                    └──────────────┬──────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                    │
     ┌────────▼────────┐  ┌───────▼───────┐   ┌───────▼───────┐
     │  passes >= N    │  │ passes < N    │   │ max retries   │
     │  → next task    │  │ → retry with  │   │   reached     │
     │                 │  │   context     │   │   → fail      │
     └─────────────────┘  └───────────────┘   └───────────────┘
```

## The Four Roles

| Role | Purpose |
|------|---------|
| **Planner** | Creates a structured plan for solving the task. Receives context from previous tasks and failed attempts on retry. |
| **Advisor** | Reviews the plan and provides critical feedback, identifies potential issues and improvements. |
| **Actor** | Executes the revised plan and produces two outputs: `how-x.md` (detailed approach) and `result-x.md` (summary). |
| **Checker** | Validates the output multiple times in parallel. The task passes only if enough checks succeed. |

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- `claude` CLI installed and configured

### Build

```bash
cargo build --release
```

## Usage

```bash
# Run with defaults
cargo run

# Show all options
cargo run -- --help

# Custom configuration
cargo run -- --tasks-dir ./my-tasks --hows-dir ./my-outputs --checks 7 --threshold 5
```

### CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `-c, --config` | `./config.json` | Path to configuration file |

## Configuration

The pipeline is configured via a JSON file. By default, it looks for `config.json` in the **current working directory** (where you run the command from).

```bash
# Uses ./config.json in current directory
./ouroboros.exe

# Specify a custom config path
./ouroboros.exe -c /path/to/config.json
./ouroboros.exe --config ./example/config.json
```

### Example config.json

```json
{
  "outliner": { "cli": "claude-code", "model": "haiku" },
  "planner": { "cli": "claude-code", "model": "opus" },
  "advisor": { "cli": "claude-code", "model": "sonnet" },
  "actor": { "cli": "claude-code", "model": "opus" },
  "checker": { "cli": "claude-code", "model": "sonnet" },
  "splitter": { "cli": "claude-code", "model": "opus" },

  "tasks_dir": "./tasks",
  "results_dir": "./results",
  "plans_dir": "./plans",
  "advises_dir": "./advises",
  "checks_dir": "./checks",
  "hows_dir": "./hows",

  "checks": 3,
  "threshold": 3,
  "max_retries": 3
}
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `outliner` | claude-code/haiku | Role config for outliner |
| `planner` | claude-code/opus | Role config for planner |
| `advisor` | claude-code/sonnet | Role config for advisor |
| `actor` | claude-code/opus | Role config for actor |
| `checker` | claude-code/sonnet | Role config for checker |
| `splitter` | claude-code/opus | Role config for splitter |
| `tasks_dir` | `./tasks` | Directory containing task files |
| `results_dir` | `./results` | Directory for result files |
| `plans_dir` | `./plans` | Directory for plan files |
| `advises_dir` | `./advises` | Directory for advice files |
| `checks_dir` | `./checks` | Directory for check result files |
| `hows_dir` | `./hows` | Directory for how files |
| `checks` | `3` | Number of validation runs per task |
| `threshold` | `3` | Minimum passes required |
| `max_retries` | `3` | Maximum retry attempts |

### Supported CLIs

| CLI | Description |
|-----|-------------|
| `claude-code` | Claude Code CLI |
| `codex` | OpenAI Codex CLI |

## Directory Structure

```
ouroboros/
├── tasks/              # Input: task definitions
│   ├── task-1.md
│   ├── task-2.md
│   └── ...
├── hows/               # Output: solutions and results
│   ├── how-1.md
│   ├── result-1.md
│   ├── how-2.md
│   ├── result-2.md
│   └── ...
└── src/
    ├── main.rs         # Entry point
    ├── config.rs       # CLI configuration
    ├── claude.rs       # Claude CLI subprocess wrapper
    ├── pipeline.rs     # Main orchestration loop
    └── roles/          # Role implementations
        ├── planner.rs
        ├── advisor.rs
        ├── actor.rs
        └── checker.rs
```

## Creating Tasks

Create markdown files in the `tasks/` directory following the naming convention `task-N.md`:

```markdown
# Task 1: Example Task

Describe what needs to be accomplished here.

## Requirements
- Requirement 1
- Requirement 2

## Constraints
- Any constraints or limitations
```

Tasks are processed in numerical order (task-1, task-2, etc.).

## Cross-Task Context

When processing sequential tasks, the pipeline passes context from the previous task to help inform the current one:

- The Planner receives both the `how` and `result` from the previous task
- This allows later tasks to build on earlier work and maintain continuity
- Context is passed automatically without manual configuration

## How Retries Work

When a task fails validation:

1. The failed `how-x.md` content is preserved
2. On the next attempt, both Planner and Advisor receive this failed output as context
3. This allows the pipeline to learn from mistakes and try a different approach
4. If all retries are exhausted, the last attempt is saved and the pipeline reports failure

## Parallel Validation

The Checker runs all validation checks in parallel using threads, improving performance for tasks with many checks. Results are collected and displayed as each check completes.

## License

MIT
