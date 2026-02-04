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
        r#"Split this meta goal into well-structured goals and detailed tasks.

META GOAL:
{}

Rules:
- Focus on WHAT to achieve, not HOW (implementation details come later)
- Tasks should be substantial and self-contained (not micro-tasks)
- Do NOT split tasks too finely - prefer fewer, larger tasks over many small ones
- Each task runs in a separate session with NO shared memory between tasks
- The ONLY way to pass context between tasks is through explicit file references
- Every task that produces research, analysis, or documentation MUST specify an output file path
- Every task that depends on previous work MUST explicitly reference the input files to read

Context Chain Rules (CRITICAL):
- Tasks run in isolated sessions - they cannot see what previous tasks did unless you tell them which files to read
- If task N produces a document, specify: "Write output to `docs/path/filename.md`"
- If task N+1 needs that document, specify: "Read `docs/path/filename.md` for context"
- Use consistent, descriptive file paths (e.g., `docs/research/`, `docs/design/`, `docs/specs/`)
- For implementation tasks, reference all relevant design/spec documents

Task Description Guidelines:
- Each task description should be 2-4 sentences minimum
- Include the specific objective and expected outcome
- ALWAYS specify output file path if the task produces a document or artifact
- ALWAYS specify input file paths if the task depends on previous work
- Include acceptance criteria: what defines "done" for this task

Example of GOOD task descriptions with context chain:
Task 1: "Research authentication patterns and JWT implementations. Analyze security best practices and common pitfalls. Write findings to `docs/research/auth-patterns.md`. Acceptance: `docs/research/auth-patterns.md` exists with comprehensive analysis."
Task 2: "Design the authentication module based on research in `docs/research/auth-patterns.md`. Define interfaces, data flow, and security measures. Write design to `docs/design/auth-module.md`. Acceptance: `docs/design/auth-module.md` exists with complete design specification."
Task 3: "Implement authentication module following the design in `docs/design/auth-module.md`. Create the module in `src/auth/`. Acceptance: implementation matches design, tests pass."

Example of BAD task descriptions:
- "Research auth patterns" (no output file - next task can't find it)
- "Implement based on previous research" (no input file reference - task can't find the research)

Return ONLY JSON:
{{
  "goals": [
    {{
      "name": "Descriptive goal name",
      "tasks": [
        "Task 1 description. Write output to `docs/category/output-file.md`. Acceptance: file exists with required content.",
        "Task 2 description. Read `docs/category/output-file.md` for context. Write output to `docs/category/next-file.md`. Acceptance: file exists.",
        "Implementation task. Read `docs/category/next-file.md` for design. Modify `src/module/`. Acceptance: implementation complete, tests pass."
      ]
    }}
  ]
}}"#,
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

/// Format a task into markdown content
pub fn format_task(task: &Task, _task_index: usize, _total_tasks: usize, _meta_goal: &str) -> String {
    format!(
        r#"# Goal

{goal}

# Task

{description}
"#,
        goal = task.goal.trim(),
        description = task.description.trim()
    )
}
