# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Prerequisites

- Rust toolchain (1.70+)
- `claude` CLI installed and accessible in PATH

## Build & Run

```bash
cargo build              # Build the project
cargo build --release    # Build optimized release
cargo run                # Run with default settings
cargo run -- --help      # Show CLI options
```

## CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `--tasks-dir` | `./tasks` | Directory containing task files |
| `--results-dir` | `./results` | Directory for result files |
| `--plans-dir` | `./plans` | Directory for plan files |
| `--advises-dir` | `./advises` | Directory for advice files |
| `--checks-dir` | `./checks` | Directory for check result files |
| `--hows-dir` | `./hows` | Directory for how files |
| `--checks` | `5` | Number of validation runs per task |
| `--threshold` | `4` | Minimum passes required |
| `--max-retries` | `3` | Maximum retry attempts |

## Architecture

Ouroboros is a recursive Claude pipeline that processes tasks sequentially through four specialized roles, with automatic retry and validation.

### Pipeline Flow

For each `task-x.md` in `tasks/`:
1. **Planner** creates initial plan
2. **Advisor** reviews and provides feedback → writes `advise-x.md`
3. **Planner** revises based on feedback → writes `plan-x.md`
4. **Actor** executes plan → writes `result-x.md`
5. **Checker** validates N times (configurable) → writes `check-x-1.md`, `check-x-2.md`, etc.
   - Each check file contains detailed feedback on what passed/failed
   - If passes ≥ threshold → next task
   - If fails → retry with failed approach context (up to max-retries)

### Key Modules

- `claude.rs` - Spawns `claude` CLI subprocess with `-p` flag for prompts
- `pipeline.rs` - Main orchestration loop, retry logic, file I/O
- `roles/` - Four role implementations (planner, advisor, actor, checker)
- `config.rs` - CLI argument parsing with clap

### Data Flow

**Important**: Each file type MUST be placed in its designated directory. The pipeline expects this structure:

| Directory | File Pattern | Example |
|-----------|--------------|---------|
| `tasks/` | `task-{x}.md` | `tasks/task-1.md` |
| `plans/` | `plan-{x}-{y}.md` | `plans/plan-1-1.md` |
| `advises/` | `advise-{x}-{y}.md` | `advises/advise-1-1.md` |
| `results/` | `result-{x}-{y}.md` | `results/result-1-1.md` |
| `hows/` | `how-{x}-{y}.md` | `hows/how-1-1.md` |
| `checks/` | `check-{x}-{y}-{n}.md` | `checks/check-1-1-1.md` |

Where `{x}` is the task number and `{y}` is the attempt number.

- **Sequential context loading**: For task N (N > 1), the pipeline reads `result-(N-1)-{highest}.md` and `how-(N-1)-{highest}.md` where `{highest}` is the highest attempt number found for that task. This ensures each task only sees context from the immediately preceding task, not all tasks at once.
- On retry, failed result content is passed to Planner and Advisor for context

### Error Handling

Uses `anyhow::Result` with `.context()` for descriptive error messages. The `claude.rs` module wraps subprocess calls and propagates CLI errors.
