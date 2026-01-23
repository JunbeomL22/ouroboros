# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Prerequisites

- Rust toolchain (1.70+)
- `claude` CLI installed and accessible in PATH

## OpenCode

If you are unsure about OpenCode options or configuration, try:
- Run `opencode --help` to see available commands and flags
- Navigate to `./opencode` folder for configuration files and documentation

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
1. **Outliner** creates high-level outline
2. **Advisor** reviews and provides feedback → writes `advise-x.md`
3. **Planner** creates detailed plan from outline and advice → writes `plan-x.md`
4. **Actor** executes plan → writes `result-x.md`
5. **Checker** validates N times (configurable) → writes `check-x-y-n.md`
   - Classifies failures as MINOR or MAJOR issues
   - If passes ≥ threshold and no issues → next task
6. **Fixer** phase (if issues detected):
   - **MajorFixer** (if any MAJOR issues) → writes `fix-x-y-major.md`
     - Has full authority to modify logic/functionality
     - Also handles any minor issues present
   - **MinorFixer** (if only MINOR issues) → writes `fix-x-y-minor.md`
     - Only cosmetic fixes, no logic changes
7. **Rechecks** (after fixer runs) → writes `recheck-x-y-n.md`
   - Re-validates with `recheck_threshold` (separate from initial threshold)
   - If passes ≥ recheck_threshold → task complete
   - If fails → retry with full context (NO second fixer attempt)

### Issue Classification

The Checker classifies issues into two categories:
- **MINOR**: formatting, style, missing comments/docs, typos, cosmetic issues
- **MAJOR**: missing functionality, broken logic, security issues, core requirements not met

### Key Modules

- `agent.rs` - Spawns agent CLI subprocess (claude-code or codex)
- `pipeline.rs` - Main orchestration loop, retry logic, file I/O
- `roles/` - Role implementations (outliner, planner, advisor, actor, checker, minor_fixer, major_fixer, splitter)
- `config.rs` - CLI argument parsing with clap

### Data Flow

**Important**: Each file type MUST be placed in its designated directory. The pipeline expects this structure:

| Directory | File Pattern | Example |
|-----------|--------------|---------|
| `tasks/` | `task-{x}.md` | `tasks/task-1.md` |
| `outlines/` | `outline-{x}-{y}.md` | `outlines/outline-1-1.md` |
| `plans/` | `plan-{x}-{y}.md` | `plans/plan-1-1.md` |
| `advises/` | `advise-{x}-{y}.md` | `advises/advise-1-1.md` |
| `results/` | `result-{x}-{y}.md` | `results/result-1-1.md` |
| `hows/` | `how-{x}-{y}.md` | `hows/how-1-1.md` |
| `checks/` | `check-{x}-{y}-{n}.md` | `checks/check-1-1-1.md` |
| `fixes/` | `fix-{x}-{y}-minor.md` | `fixes/fix-1-1-minor.md` |
| `fixes/` | `fix-{x}-{y}-major.md` | `fixes/fix-1-1-major.md` |
| `rechecks/` | `recheck-{x}-{y}-{n}.md` | `rechecks/recheck-1-1-1.md` |

Where `{x}` is the task number, `{y}` is the attempt number, and `{n}` is the check run number.

- **Sequential context loading**: For task N (N > 1), the pipeline reads `result-(N-1)-{highest}.md` and `how-(N-1)-{highest}.md` where `{highest}` is the highest attempt number found for that task. This ensures each task only sees context from the immediately preceding task, not all tasks at once.
- On retry, failed result content is passed to Planner and Advisor for context

### Error Handling

Uses `anyhow::Result` with `.context()` for descriptive error messages. The `claude.rs` module wraps subprocess calls and propagates CLI errors.
