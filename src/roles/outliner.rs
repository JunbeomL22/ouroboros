use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

use super::PrevTaskContext;

pub fn outline(
    role_config: &RoleConfig,
    task: &str,
    failed_how: Option<&str>,
    check_feedbacks: Option<&str>,
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

    let prompt = match failed_how {
        Some(how) => format!(
            r#"Task:
{}{}

Previous failed approach:
{}{}

Create a high-level outline for this task. Focus on:
1. Key objectives and deliverables
2. Major steps or phases (not detailed implementation)
3. Potential risks or challenges to consider
4. Success criteria

Keep the outline concise and strategic. The detailed planning will be done by the Planner based on your outline."#,
            task, context, how, check_context
        ),
        None => format!(
            r#"Task:
{}{}

Create a high-level outline for this task. Focus on:
1. Key objectives and deliverables
2. Major steps or phases (not detailed implementation)
3. Potential risks or challenges to consider
4. Success criteria

Keep the outline concise and strategic. The detailed planning will be done by the Planner based on your outline."#,
            task, context
        ),
    };

    call_agent(role_config, "Outliner", &prompt)
}
