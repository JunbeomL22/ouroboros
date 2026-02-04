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
- Each task runs in a separate session with no shared memory between tasks
- Only add context handoff files when absolutely necessary
- For VERY complex tasks: split into two sequential tasks:
  1. First task: Create a detailed planning document (e.g., `plan-feature-x.md`) analyzing requirements, dependencies, and implementation approach
  2. Second task: Reference that planning document and execute the actual implementation
  This pattern ensures complex work is thoroughly planned before execution

Task Description Guidelines:
- Each task description should be 2-4 sentences minimum
- Include the specific objective and expected outcome
- Mention any key constraints or requirements relevant to that task
- Specify what files, modules, or components the task should focus on (if applicable)
- Include acceptance criteria: what defines "done" for this task
- If the task depends on understanding previous work, note what context is needed

Example of a GOOD task description:
"Implement user authentication module with JWT token support. The module should handle login, logout, and token refresh operations. Store tokens securely and implement proper expiration handling. Acceptance: users can log in, receive a valid JWT, and access protected routes."

Example of a BAD task description:
"Add auth" (too vague, no context or acceptance criteria)

Example of COMPLEX task splitting (two-phase pattern):
Task 1: "Create a detailed planning document for the payment integration system. Analyze the current codebase structure, identify integration points, document API requirements, and outline the implementation steps. Write the plan to `plan-payment-integration.md`. Acceptance: comprehensive planning document exists with clear implementation roadmap."
Task 2: "Implement the payment integration system following the plan in `plan-payment-integration.md`. Execute each step outlined in the planning document. Acceptance: all planned features implemented and tested as specified in the plan."

Return ONLY JSON:
{{
  "goals": [
    {{
      "name": "Descriptive goal name",
      "tasks": [
        "Detailed task 1 description with objective, scope, and acceptance criteria",
        "Detailed task 2 description with objective, scope, and acceptance criteria"
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
