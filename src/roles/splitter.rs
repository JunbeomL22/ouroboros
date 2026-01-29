use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

/// Split a meta task into individual task files
pub fn split(role_config: &RoleConfig, meta_task: &str) -> Result<Vec<String>> {
    let prompt = format!(
        r#"You are a task splitter. Break down the following goal into a sequence of independent tasks.

GOAL:
{}

STEP 1 - IDENTIFY GLOBAL CONSTRAINTS:
First, extract ALL global constraints from the goal. These are requirements that apply to EVERY task:
- Technology restrictions (e.g., "do not use tokio", "use std-async", "no external dependencies")
- Architecture requirements (e.g., "must use epoll", "must be thread-safe")
- Style requirements (e.g., "follow existing patterns", "use async/await")
- Performance requirements (e.g., "must handle 10k connections")

STEP 2 - EMBED CONSTRAINTS IN EVERY TASK:
CRITICAL: Every single task description MUST include the global constraints inline.
DO NOT assume later tasks will "remember" constraints from earlier tasks - they won't!

Example goal: "Create a websocket server. Do not use tokio, use async-std instead."
BAD split (constraints lost):
- ["Create WebSocket server struct", "Add connection handling", "Add message routing"]
GOOD split (constraints preserved):
- ["Create WebSocket server struct using async-std (NOT tokio)", "Add connection handling using async-std (NOT tokio)", "Add message routing using async-std (NOT tokio)"]

RULES FOR EACH TASK:
1. ONE clear objective only - no "and" or "then"
2. Must be independently verifiable (can check pass/fail)
3. Use simple, direct language
4. No vague words like "improve", "enhance", "optimize" - be specific
5. Each task builds on previous tasks but can be validated alone
6. MUST include all global constraints in the task description itself

CRITICAL - SPECIFIC PURPOSE STATEMENT:
Each task MUST include:
- WHAT: The exact artifact to create/modify (file, function, struct, etc.)
- WHERE: The specific file path or module location
- SUCCESS CRITERIA: How to verify the task is complete (e.g., "compiles without errors", "test passes", "file exists with X content")

BAD (vague purpose):
- "Implement error handling for the server"
- "Add validation logic"

GOOD (specific purpose):
- "Create ErrorKind enum in src/error.rs with variants: IoError, ParseError, ConnectionClosed. Must compile with 'cargo check'."
- "Add validate_message() function in src/protocol.rs that returns Result<(), ErrorKind> for Message struct. Must pass 'cargo test validate_message'."

IMPORTANT - CONTEXT HANDOFF:
- Each task runs in a separate session with NO memory of previous sessions
- If a task creates something the next task needs, specify writing to a file with ABSOLUTE PATH
- The next task must read from that file to continue
- Example: "Write the API schema to C:/project/docs/api-schema.json"
- Example: "Read C:/project/docs/api-schema.json and implement the endpoints"

CRITICAL - DOCUMENT/UNDERSTAND TASKS:
- When a task involves "document", "understand", "analyze", or "research" something, it MUST write findings to a specific file
- Specify the EXACT filename with ABSOLUTE PATH where results will be saved
- The next task MUST explicitly reference that filename to read from
- Example: "Analyze the authentication flow in src/auth/ and write findings to C:/project/docs/auth-analysis.md"
- Example: "Read C:/project/docs/auth-analysis.md and implement the improvements described"
- NEVER use vague outputs like "document it" or "write notes" - always specify the exact file path

OUTPUT FORMAT:
Return ONLY a JSON array of task descriptions. No markdown, no explanation.
Example: ["Create user model with id, name, email fields", "Add REST endpoint GET /users", "Add REST endpoint POST /users"]

BAD examples (constraints lost in later tasks):
- ["Create socket using async-std", "Add polling logic", "Handle messages"] - tasks 2 and 3 lost "async-std" constraint!

GOOD examples (constraints preserved):
- ["Create UDP socket struct using async-std (NOT tokio)", "Add epoll-style polling to UDP socket using async-std (NOT tokio)", "Add message handling to UDP socket using async-std (NOT tokio)"]

Now split the goal into tasks:"#,
        meta_task
    );

    let response = call_agent(role_config, "TaskSplitter", &prompt)?;

    // Parse JSON array from response
    let tasks: Vec<String> = serde_json::from_str(&response)
        .or_else(|_| {
            // Try to extract JSON array if there's extra text
            let start = response.find('[').unwrap_or(0);
            let end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
            serde_json::from_str(&response[start..end])
        })
        .map_err(|e| anyhow::anyhow!("Failed to parse tasks JSON: {}. Response was: {}", e, response))?;

    Ok(tasks)
}

/// Format a task description into markdown content with full context
pub fn format_task(task_num: usize, total_tasks: usize, description: &str, original_goal: &str) -> String {
    format!(
        r#"# Task {task_num} of {total_tasks}

## Original Goal

{original_goal}

## Current Task

{description}

## Context

- This is task {task_num} in a sequence of {total_tasks} tasks
- Each task builds upon previous tasks
- Focus on completing THIS task's objective while keeping the original goal in mind
"#,
        task_num = task_num,
        total_tasks = total_tasks,
        description = description,
        original_goal = original_goal.trim()
    )
}
