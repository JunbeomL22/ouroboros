use anyhow::Result;
use crate::agent::{call_agent_with_subagents, SubagentDef};
use crate::config::RoleConfig;

/// Context from a previously completed task (only how and result)
pub struct PrevTaskContext<'a> {
    pub how: &'a str,
    pub result: &'a str,
}

pub fn plan(
    role_config: &RoleConfig,
    web_searcher_config: &RoleConfig,
    task: &str,
    failed_how: Option<&str>,
    failed_plan: Option<&str>,
    check_feedbacks: Option<&str>,
    advisor_feedback: Option<&str>,
    prev_context: Option<PrevTaskContext>,
) -> Result<String> {
    // Create web-searcher subagent definition
    let web_searcher_subagent = SubagentDef::web_searcher_from_config(web_searcher_config);

    let context = match prev_context {
        Some(ctx) => {
            format!(
                "\n\nContext from previous task:\n\nHow it was done:\n{}\n\nResult:\n{}\n",
                ctx.how, ctx.result
            )
        }
        None => String::new(),
    };

    let check_context = match check_feedbacks {
        Some(feedbacks) => format!("\n\nChecker feedback from failed attempt:\n{}\n", feedbacks),
        None => String::new(),
    };

    let plan_context = match failed_plan {
        Some(plan) => format!("\n\nPlan from failed attempt (includes outline and advise):\n{}\n", plan),
        None => String::new(),
    };

    let browser_instructions = r#"

=== BROWSER AUTOMATION ===
If any step requires web browser interaction (navigating websites, filling forms,
clicking buttons, extracting web content, screenshots), wrap those steps with
[BROWSER] and [/BROWSER] markers:

Example:
## Step 3: Fetch data from website
[BROWSER]
Navigate to https://example.com/data
Extract the data table
Take screenshot
[/BROWSER]

Only use browser markers for genuine browser automation needs."#;

    let search_instructions = r#"

=== WEB SEARCH ===
If you need to search the web for information to create a better plan, use the Task tool to delegate to the "web-searcher" subagent:
- Call: Task tool with subagent_type="web-searcher" and a clear search query in the prompt
- The web-searcher uses a cheaper model to save tokens
- Only use when you genuinely need external information not available in the codebase
- Do NOT use WebSearch directly - always delegate to web-searcher for cost efficiency"#;

    let important_note = format!(
        "=== IMPORTANT ===
- DO NOT create task-*.md files in the current directory or any other location. The task files are read-only inputs managed by the system.
- DO NOT execute any actions, modify files, run commands, or implement anything. Your ONLY job is to output a written plan. The Actor role will execute the plan later.

=== CONTEXT HANDOFF ===
The next task runs in a SEPARATE SESSION with no memory of this one.
- If this task reads files or gathers information, plan for the Actor to INCLUDE that content in the RESULT section
- The RESULT section is the ONLY way to pass context to the next task
- For research/analysis tasks: explicitly plan to output findings, code snippets, and analysis in the result{}{}",
        browser_instructions, search_instructions
    );

    let prompt = match (failed_how, advisor_feedback) {
        (Some(how), Some(feedback)) => format!(
            "Task:\n{}{}\n\nPrevious failed approach:\n{}{}{}\n\nAdvisor feedback on your previous plan:\n{}\n\nCreate a revised plan considering what failed, the checker feedback, and the advisor's feedback.\n\n{}",
            task, context, how, plan_context, check_context, feedback, important_note
        ),
        (Some(how), None) => format!(
            "Task:\n{}{}\n\nPrevious failed approach:\n{}{}{}\n\nCreate a new plan that avoids the mistakes in the previous approach.\n\n{}",
            task, context, how, plan_context, check_context, important_note
        ),
        (None, Some(feedback)) => format!(
            "Task:\n{}{}\n\nAdvisor feedback:\n{}\n\nRevise your plan based on the advisor's feedback.\n\n{}",
            task, context, feedback, important_note
        ),
        (None, None) => format!(
            "Task:\n{}{}\n\nCreate a detailed step-by-step plan to accomplish this task.\n\n{}",
            task, context, important_note
        ),
    };

    call_agent_with_subagents(
        role_config,
        "Planner",
        &prompt,
        Some(vec![web_searcher_subagent]),
    )
}
