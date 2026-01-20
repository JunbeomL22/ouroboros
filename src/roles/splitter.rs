use anyhow::Result;
use crate::claude::call_claude;
use crate::model;

/// Split a meta task into individual task files
pub fn split(meta_task: &str) -> Result<Vec<String>> {
    let prompt = format!(
        r#"You are a task splitter. Break down the following goal into a sequence of independent tasks.

GOAL:
{}

RULES FOR EACH TASK:
1. ONE clear objective only - no "and" or "then"
2. Must be independently verifiable (can check pass/fail)
3. Use simple, direct language
4. No vague words like "improve", "enhance", "optimize" - be specific
5. Each task builds on previous tasks but can be validated alone

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

BAD examples (too vague or complex):
- "Set up the project and configure everything" (multiple things)
- "Improve the code quality" (vague)
- "Handle errors appropriately" (vague)
- "Understand the codebase" (no output file specified)
- "Document the API" (no output file specified)

GOOD examples (clear and specific):
- "Create SQLite database with users table (id, name, email)"
- "Add input validation: name required, email must contain @"
- "Return 400 error with message when validation fails"
- "Analyze src/api/ directory structure and write findings to C:/project/docs/api-structure.md"
- "Read C:/project/docs/api-structure.md and add missing endpoint POST /users/login"

Now split the goal into tasks:"#,
        meta_task
    );

    let response = call_claude("TaskSplitter", &prompt, model::SPLITTER)?;

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

/// Format a task description into markdown content
pub fn format_task(task_num: usize, description: &str) -> String {
    format!(
        r#"# Task {}

{}
"#,
        task_num, description
    )
}
