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
        r#"You are a task splitter. Break down the following meta goal into GOALS, then split each goal into meaningful tasks.

META GOAL:
{}

=== SPLITTING PHILOSOPHY ===

1. GOAL-ORIENTED, NOT ACTION-ORIENTED
   - Focus on WHAT to achieve, not HOW to do it
   - Each task should have a clear PURPOSE and DELIVERABLE
   - Avoid micro-tasks like "read file X" or "create variable Y"

2. MEANINGFUL TASK SIZE
   - A task should be substantial enough to produce a tangible outcome
   - Combine related small actions into one cohesive task
   - Think "implement feature X" not "write function A, then function B, then..."

3. CONTEXT CONTINUITY
   - Each task runs in a SEPARATE SESSION with NO MEMORY
   - If context must flow to the next task, specify an output .md file
   - Use ./outputs/ directory for intermediate context files

=== TASK WRITING GUIDELINES ===

GOOD task descriptions:
- "Implement user authentication module with JWT support"
- "Create REST API endpoints for product CRUD operations"
- "Refactor database layer to support async operations"

BAD task descriptions (too granular):
- "Create auth.rs file"
- "Add login function"
- "Add logout function"
- "Add token validation"

=== CONTEXT HANDOFF ===

When a task produces information needed by subsequent tasks:
- Explicitly state: "Save analysis/findings to ./outputs/[descriptive-name].md"
- Next task should reference: "Using context from ./outputs/[descriptive-name].md, ..."

=== OUTPUT FORMAT ===

Return ONLY JSON:
{{
  "goals": [
    {{
      "name": "Clear goal name",
      "tasks": ["Meaningful task 1 description", "Meaningful task 2 description"]
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

/// Format a task into markdown content with clear context hierarchy
pub fn format_task(task: &Task, task_index: usize, total_tasks: usize, meta_goal: &str) -> String {
    format!(
        r#"# Context

## Meta Goal (Big Picture)

{meta_goal}

---

## Current Focus

**Goal**: {goal}

**Task {current} of {total}**

---

# Your Task

{description}

---

## Notes

- This task is part of a larger objective described above
- Focus ONLY on completing this specific task
- If you need to pass context to subsequent tasks, save findings to `./outputs/` as `.md` files
"#,
        meta_goal = meta_goal.trim(),
        goal = task.goal.trim(),
        current = task_index + 1,
        total = total_tasks,
        description = task.description.trim()
    )
}
