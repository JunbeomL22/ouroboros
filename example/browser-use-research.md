# browser_use Integration Research

## Executive Summary

This document analyzes the Ouroboros pipeline architecture to determine the optimal approach for integrating `browser_use` (a Python browser automation library). The research covers the current Actor execution flow, subprocess spawning mechanisms, configuration patterns, and provides specific recommendations for conditional browser_use invocation.

**Recommendation**: Add a `BrowserUse` variant to the `AgentCli` enum (Approach A) for seamless integration with minimal architectural disruption.

---

## 1. Current Actor Execution Flow

### Entry Point: `actor.rs:10`

```rust
pub fn act(role_config: &RoleConfig, task: &str, plan: &str) -> Result<ActorOutput>
```

The Actor role receives:
- `role_config`: Contains CLI type, model, and provider settings
- `task`: The task description from task-x.md
- `plan`: The detailed plan from the Planner role

### Prompt Construction: `actor.rs:11-47`

The Actor constructs a prompt template that includes:
1. Task content (line 13)
2. Plan to follow (line 16)
3. Critical file location rules (lines 20-29)
4. Required output format with `===HOW===` and `===RESULT===` delimiters (lines 31-44)

### Agent Invocation: `actor.rs:49`

```rust
let output = call_agent(role_config, "Actor", &prompt)?;
```

The Actor delegates execution to `call_agent()` in `agent.rs`, passing the constructed prompt.

### Output Parsing: `actor.rs:53-78`

The `parse_actor_output()` function:
1. Searches for `===HOW===` marker (line 54, 57)
2. Searches for `===RESULT===` marker (line 55, 63)
3. Extracts content between markers (lines 70-75)
4. Returns `ActorOutput { how, result }` struct (line 77)

**Critical Constraint**: Any agent used by Actor MUST produce output containing both `===HOW===` and `===RESULT===` delimiters.

---

## 2. Process Spawning Mechanism

### Dispatch Pattern: `agent.rs:27-33`

```rust
pub fn call_agent(role_config: &RoleConfig, role: &str, prompt: &str) -> Result<String> {
    match role_config.cli {
        AgentCli::ClaudeCode => call_claude_code(role, prompt, &role_config.model, &role_config.provider),
        AgentCli::Codex => call_codex(role, prompt, &role_config.model),
        AgentCli::OpenCode => call_opencode(role, prompt, &role_config.model),
    }
}
```

The dispatcher pattern allows easy extension for new CLI types.

### Subprocess Pattern: `agent.rs:92-117`

The standard subprocess pattern used across all CLI handlers:

```rust
// 1. Configure Command with stdin/stdout/stderr piped
let mut child = cmd
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .context("Failed to spawn CLI")?;

// 2. Write prompt to stdin
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(full_prompt.as_bytes())?;
}

// 3. Wait for completion and collect output
let output = child.wait_with_output()?;

// 4. Check exit status
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("CLI failed: {}", stderr);
}

// 5. Return stdout as string
String::from_utf8(output.stdout)
```

### Provider Environment Injection: `agent.rs:68-90`

For Claude Code, provider-specific environment variables are set:

```rust
match provider {
    Provider::Minimax => {
        cmd.env("ANTHROPIC_BASE_URL", creds.base_url);
        cmd.env("ANTHROPIC_AUTH_TOKEN", &creds.api_key);
        cmd.env("DISABLE_PROMPT_CACHING", "1");
    }
    Provider::Anthropic => {
        // Optional API key override from secrets
    }
}
```

### Error Handling: `agent.rs:109-112`

```rust
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("Claude CLI failed: {}", stderr);
}
```

OpenCode provides more detailed error reporting (lines 215-224), including exit code and both stdout/stderr.

---

## 3. Configuration Patterns

### AgentCli Enum: `config.rs:10-16`

```rust
pub enum AgentCli {
    #[serde(rename = "claude", alias = "claude-code")]
    ClaudeCode,
    Codex,
    #[serde(alias = "opencode")]
    OpenCode,
}
```

Supports serde aliases for flexible JSON configuration.

### Provider Enum: `config.rs:19-25`

```rust
pub enum Provider {
    #[default]
    Anthropic,
    Minimax,
}
```

### RoleConfig Struct: `config.rs:60-66`

```rust
pub struct RoleConfig {
    pub cli: AgentCli,
    pub model: String,
    #[serde(default)]
    pub provider: Provider,
}
```

Each role (actor, checker, planner, etc.) has its own `RoleConfig`.

### AgentConfig: `config.rs:82-108`

Contains all role configurations and pipeline settings:
- 8 role configs: outliner, planner, advisor, actor, checker, splitter, minor_fixer, major_fixer
- 9 directory paths for various output types
- 4 pipeline settings: checks, threshold, recheck_threshold, max_tries

---

## 4. Integration Gap Analysis

### 4.1 Output Format Mismatch

**Problem**: `browser_use` is a Python library that performs browser automation and returns structured results, not `===HOW===`/`===RESULT===` formatted text.

**Impact**: Direct invocation would cause `parse_actor_output()` to fail at lines 57-68.

**Solution Required**: Wrapper script that:
1. Invokes browser_use
2. Captures actions and results
3. Formats output with required delimiters

### 4.2 Process Lifecycle Differences

**Current Model**: Subprocess is spawned, receives prompt via stdin, runs to completion, returns stdout.

**browser_use Model**:
- Requires Python runtime
- May need persistent browser session
- Actions are interactive, not batch

**Solution Required**: Either:
- Batch mode: Script that runs browser actions and terminates
- Session mode: Long-running server with request/response pattern

### 4.3 Dependency Chain

**Current Dependencies**:
- Claude CLI (claude-code binary)
- Codex CLI (codex binary)
- OpenCode CLI (opencode binary)

**browser_use Dependencies**:
- Python 3.9+
- browser_use package (`pip install browser_use`)
- Browser binaries (Chrome/Chromium/Firefox)
- WebDriver (playwright or selenium backend)

**Solution Required**: Installation verification and graceful degradation.

### 4.4 Error Classification

**Current Error Types** (from agent.rs):
- Spawn failure
- Stdin write failure
- Wait failure
- Non-zero exit status
- UTF-8 parsing failure

**browser_use Error Types**:
- Browser launch failure
- Navigation timeout
- Element not found
- JavaScript errors
- Network errors
- Screenshot failures

**Solution Required**: Extended error handling in wrapper.

---

## 5. Implementation Approaches

### Approach A: Add BrowserUse Variant to AgentCli (Recommended)

**Description**: Extend the existing `AgentCli` enum with a `BrowserUse` variant and add corresponding dispatcher logic.

**Code Changes Required**:

1. **config.rs:10-16** - Add enum variant:
```rust
pub enum AgentCli {
    ClaudeCode,
    Codex,
    OpenCode,
    BrowserUse,  // NEW
}
```

2. **agent.rs:27-33** - Add dispatch arm:
```rust
match role_config.cli {
    AgentCli::ClaudeCode => call_claude_code(...),
    AgentCli::Codex => call_codex(...),
    AgentCli::OpenCode => call_opencode(...),
    AgentCli::BrowserUse => call_browser_use(role, prompt, &role_config.model),  // NEW
}
```

3. **agent.rs** - Add new function:
```rust
fn call_browser_use(role: &str, prompt: &str, model: &str) -> Result<String> {
    // Invoke Python wrapper script
    // Return formatted output with ===HOW=== and ===RESULT===
}
```

**Pros**:
- Minimal architectural changes
- Follows established patterns
- Easy to configure per-role
- Maintains single dispatch point

**Cons**:
- Requires Python wrapper script
- All roles would share same browser_use config structure

### Approach B: Create Separate BrowserActor Role

**Description**: Create a dedicated `browser_actor` role with its own execution flow.

**Code Changes Required**:

1. **New file** `src/roles/browser_actor.rs`
2. **config.rs:82-90** - Add `browser_actor: RoleConfig`
3. **pipeline.rs:410** - Add routing logic to choose Actor vs BrowserActor

**Pros**:
- Complete separation of concerns
- Can have different output format
- Specialized error handling

**Cons**:
- Significant code duplication
- More complex pipeline logic
- Harder to maintain

### Approach C: Conditional Detection in Actor

**Description**: Detect browser-related tasks in Actor and conditionally invoke browser_use.

**Not Recommended** due to:
- Violates single responsibility principle
- Makes Actor logic complex
- Task detection is error-prone
- Hard to test and debug

---

## 6. Recommended Implementation Path

### Phase 1: Foundation

1. **Create Python wrapper script** (`browser_use_wrapper.py`):
   - Accept task prompt via stdin
   - Parse browser automation instructions
   - Execute browser_use actions
   - Format output with `===HOW===` and `===RESULT===`
   - Return via stdout

2. **Add BrowserUse variant to config.rs:10-16**

3. **Update config.rs:46-55** (FromStr impl):
```rust
"browser-use" | "browser_use" => Ok(AgentCli::BrowserUse),
```

4. **Update config.rs:33-40** (Display impl):
```rust
AgentCli::BrowserUse => write!(f, "browser-use"),
```

### Phase 2: Agent Integration

5. **Add call_browser_use() to agent.rs** after line 231:
```rust
fn call_browser_use(role: &str, prompt: &str, _model: &str) -> Result<String> {
    let full_prompt = format!(
        "You are acting as a {}. Execute the following browser automation task.\n\n{}",
        role, prompt
    );

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/C", "python", "browser_use_wrapper.py"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn browser_use wrapper")?;

    // ... standard stdin/stdout handling
}
```

6. **Update agent.rs:27-33** dispatch:
```rust
AgentCli::BrowserUse => call_browser_use(role, prompt, &role_config.model),
```

### Phase 3: Configuration

7. **Add Provider variant if needed** (`config.rs:19-25`):
```rust
pub enum Provider {
    Anthropic,
    Minimax,
    BrowserUse,  // For browser-specific config
}
```

8. **Update secrets.json.example**:
```json
{
  "browser_use": {
    "browser_type": "chromium",
    "headless": true,
    "timeout": 30000
  }
}
```

### Phase 4: Testing

9. Create test task using browser_use
10. Validate output format parsing
11. Test error scenarios

---

## 7. Security Considerations

### 7.1 Web Navigation Risks

- **Arbitrary URL Access**: browser_use can navigate to any URL
- **Mitigation**: Implement URL allowlist in wrapper script
- **Config Location**: `secrets.json` → `browser_use.allowed_domains`

### 7.2 Credential Exposure

- **Browser Sessions**: May contain cached credentials
- **Mitigation**: Use isolated browser profiles
- **Config**: `browser_use.profile_dir = temp`

### 7.3 Data Exfiltration

- **file:// Protocol**: Can access local filesystem via browser
- **Mitigation**: Block file:// URLs in wrapper
- **Validation**: Check URL scheme before navigation

### 7.4 Resource Exhaustion

- **Memory**: Browser processes are memory-intensive
- **CPU**: JavaScript execution can spike CPU
- **Mitigation**: Implement timeout and max_memory limits
- **Config**: `browser_use.timeout`, `browser_use.max_memory_mb`

---

## 8. Code Location Reference Table

| Component | File | Lines | Description |
|-----------|------|-------|-------------|
| Actor entry | actor.rs | 10 | `act()` function signature |
| Prompt template | actor.rs | 11-47 | Task + plan formatting |
| Output parsing | actor.rs | 53-78 | HOW/RESULT extraction |
| Agent dispatch | agent.rs | 27-33 | CLI type matching |
| Claude subprocess | agent.rs | 92-117 | stdin/stdout piping |
| Provider env | agent.rs | 68-90 | Environment injection |
| Error handling | agent.rs | 109-112 | Exit status check |
| AgentCli enum | config.rs | 10-16 | CLI type definitions |
| Provider enum | config.rs | 19-25 | API providers |
| RoleConfig struct | config.rs | 60-66 | Per-role settings |
| AgentConfig | config.rs | 82-108 | Full configuration |
| Pipeline Actor call | pipeline.rs | 410 | `act(&config.actor, ...)` |
| Check handling | pipeline.rs | 442-489 | Parallel validation |

---

## 9. Implementation Checklist

- [ ] Create `browser_use_wrapper.py` script
- [ ] Add `BrowserUse` to `AgentCli` enum in `config.rs`
- [ ] Update `FromStr` impl in `config.rs`
- [ ] Update `Display` impl in `config.rs`
- [ ] Add `call_browser_use()` in `agent.rs`
- [ ] Add dispatch arm in `call_agent()` in `agent.rs`
- [ ] Add browser_use section to `secrets.json.example`
- [ ] Update CLAUDE.md documentation
- [ ] Create test task file
- [ ] Test on Windows and Unix platforms
- [ ] Document security mitigations

---

## Appendix: Example Configuration

```json
{
  "actor": {
    "cli": "browser-use",
    "model": "n/a",
    "provider": "anthropic"
  },
  "checker": {
    "cli": "claude-code",
    "model": "sonnet",
    "provider": "anthropic"
  }
}
```

This configuration would use browser_use for Actor execution while keeping Claude Code for validation.
