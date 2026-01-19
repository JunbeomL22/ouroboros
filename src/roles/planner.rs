use anyhow::Result;
use crate::claude::call_claude;
use crate::model;

/// Context from a previously completed task (only how and result)
pub struct PrevTaskContext<'a> {
    pub how: &'a str,
    pub result: &'a str,
}

pub fn plan(
    task: &str,
    failed_how: Option<&str>,
    check_feedbacks: Option<&str>,
    advisor_feedback: Option<&str>,
    prev_context: Option<PrevTaskContext>,
) -> Result<String> {
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

    let prompt = match (failed_how, advisor_feedback) {
        (Some(how), Some(feedback)) => format!(
            "Task:\n{}{}\n\nPrevious failed approach:\n{}{}\n\nAdvisor feedback on your previous plan:\n{}\n\nCreate a revised plan considering what failed, the checker feedback, and the advisor's feedback.\n\n=== IMPORTANT ===\nDO NOT create task-*.md files in the current directory or any other location. The task files are read-only inputs managed by the system.",
            task, context, how, check_context, feedback
        ),
        (Some(how), None) => format!(
            "Task:\n{}{}\n\nPrevious failed approach:\n{}{}\n\nCreate a new plan that avoids the mistakes in the previous approach.\n\n=== IMPORTANT ===\nDO NOT create task-*.md files in the current directory or any other location. The task files are read-only inputs managed by the system.",
            task, context, how, check_context
        ),
        (None, Some(feedback)) => format!(
            "Task:\n{}{}\n\nAdvisor feedback:\n{}\n\nRevise your plan based on the advisor's feedback.\n\n=== IMPORTANT ===\nDO NOT create task-*.md files in the current directory or any other location. The task files are read-only inputs managed by the system.",
            task, context, feedback
        ),
        (None, None) => format!(
            "Task:\n{}{}\n\nCreate a detailed step-by-step plan to accomplish this task.\n\n=== IMPORTANT ===\nDO NOT create task-*.md files in the current directory or any other location. The task files are read-only inputs managed by the system.",
            task, context
        ),
    };

    call_claude("Planner", &prompt, model::PLANNER)
}
