use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::agent::call_agent;
use crate::config::RoleConfig;

/// A task with its parent goal context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub goal: String,
    pub description: String,
}

/// Split result containing goals and their tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitResult {
    pub goals: Vec<Goal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub name: String,
    pub tasks: Vec<String>,
}

/// Split a meta task into goals and their individual tasks
pub fn split(role_config: &RoleConfig, meta_task: &str) -> Result<Vec<Task>> {
    let prompt = format!(
        r#"You are a task splitter. Break down the following meta goal into separate GOALS, then split each goal into tasks.

META GOAL:
{}

STEP 1 - IDENTIFY GOALS:
Extract distinct goals from the meta goal. Each goal is a cohesive feature or objective.
Example meta: "Create user auth and add REST API for products"
Goals: ["User authentication system", "Product REST API"]

STEP 2 - IDENTIFY GLOBAL CONSTRAINTS:
Extract ALL constraints that apply across goals:
- Technology restrictions (e.g., "do not use tokio", "use async-std")
- Architecture requirements (e.g., "must use epoll", "thread-safe")
- Style requirements (e.g., "follow existing patterns")

STEP 3 - SPLIT EACH GOAL INTO TASKS:
For each goal, create specific tasks. EMBED constraints in every task.

RULES FOR EACH TASK:
1. ONE clear objective only - no "and" or "then"
2. Must be independently verifiable
3. MUST include global constraints inline
4. Specify WHAT (artifact), WHERE (file path), SUCCESS CRITERIA

CONTEXT HANDOFF (CRITICAL):
- Each task runs in a SEPARATE SESSION with NO MEMORY of previous tasks
- The ONLY way to pass information is through PHYSICAL FILES (.md files)
- If a task extracts/analyzes information, it MUST SAVE to a .md file
- The next task MUST READ that .md file to get the context

PATTERN FOR INFORMATION EXTRACTION:
Task N: "Read [source], extract [info], SAVE TO [output.md]"
Task N+1: "Read [output.md] from previous task and [do something with it]"

BAD EXAMPLES (context is LOST):
- "Read spsc.md and analyze syntax" ← Analysis vanishes after session!
- "Extract design patterns from code" ← Extraction result is lost!

GOOD EXAMPLES (context is PRESERVED):
- "Read C:/project/spsc.md, extract all C++ syntax elements, SAVE analysis to C:/project/outputs/spsc-analysis.md"
- "Read C:/project/outputs/spsc-analysis.md (from previous task) and create syntax documentation in C:/project/docs/syntax.md"

RULES:
1. Information extraction task → MUST specify output .md file path
2. Next task that needs that info → MUST specify input .md file path from previous task
3. Use ABSOLUTE PATHS for all file references
4. Output directory should be ./outputs/ or similar for intermediate files

SIMPLIFICATION RULE:
If a task is simple enough to complete in ONE session (read + process + output),
combine it into a SINGLE task instead of splitting into multiple tasks.

Example - DO split (complex):
- Task 1: "Analyze 10 source files, extract patterns, SAVE to analysis.md"
- Task 2: "Read analysis.md and generate documentation"

Example - DON'T split (simple):
- Single task: "Read config.json and add a new field 'timeout' with value 30"
- Single task: "Read utils.rs, find the parse function, fix the off-by-one bug"

Only split when information MUST be preserved across sessions.

OUTPUT FORMAT:
Return ONLY JSON in this exact format:
{{
  "goals": [
    {{
      "name": "Goal name here",
      "tasks": ["task 1 description", "task 2 description"]
    }},
    {{
      "name": "Another goal",
      "tasks": ["task 1", "task 2"]
    }}
  ]
}}

No markdown, no explanation. Just JSON."#,
        meta_task
    );

    let response = call_agent(role_config, "TaskSplitter", &prompt)?;

    // Parse JSON from response
    let split_result: SplitResult = serde_json::from_str(&response)
        .or_else(|_| {
            // Try to extract JSON object if there's extra text
            let start = response.find('{').unwrap_or(0);
            let end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
            serde_json::from_str(&response[start..end])
        })
        .map_err(|e| anyhow::anyhow!("Failed to parse split JSON: {}. Response was: {}", e, response))?;

    // Flatten into Task structs with goal context
    let tasks: Vec<Task> = split_result.goals
        .into_iter()
        .flat_map(|goal| {
            goal.tasks.into_iter().map(move |desc| Task {
                goal: goal.name.clone(),
                description: desc,
            })
        })
        .collect();

    Ok(tasks)
}

/// Format a task into markdown content with only its goal context
pub fn format_task(task: &Task) -> String {
    format!(
        r#"# Goal: {}

## Task

{}
"#,
        task.goal.trim(),
        task.description.trim()
    )
}
