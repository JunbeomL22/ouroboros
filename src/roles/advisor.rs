use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

pub fn advise(
    role_config: &RoleConfig,
    task: &str,
    plan: &str,
    failed_how: Option<&str>,
    check_feedbacks: Option<&str>,
) -> Result<String> {
    let check_context = match check_feedbacks {
        Some(feedbacks) => format!("\n\nChecker feedback from failed attempt:\n{}\n", feedbacks),
        None => String::new(),
    };

    let prompt = match failed_how {
        Some(how) => format!(
            r#"You are a critical code reviewer. Your job is to find flaws, not to be agreeable.

Task:
{}

Proposed plan:
{}

Previous failed approach:
{}
{}

CRITICAL REVIEW INSTRUCTIONS:
1. Assume the plan has flaws - your job is to find them
2. Question every assumption the planner made
3. Look for edge cases, error conditions, and failure modes that are not addressed
4. Identify any vague or hand-wavy steps that lack concrete implementation details
5. Check if the plan actually addresses the root cause of the previous failure, or just patches symptoms
6. Point out any missing steps, dependencies, or prerequisites
7. Challenge the approach - is there a simpler or more robust alternative?

Be harsh but constructive. Do not praise the plan. Focus entirely on what needs to be fixed or improved."#,
            task, plan, how, check_context
        ),
        None => format!(
            r#"You are a critical code reviewer. Your job is to find flaws, not to be agreeable.

Task:
{}

Proposed plan:
{}

CRITICAL REVIEW INSTRUCTIONS:
1. Assume the plan has flaws - your job is to find them
2. Question every assumption the planner made
3. Look for edge cases, error conditions, and failure modes that are not addressed
4. Identify any vague or hand-wavy steps that lack concrete implementation details
5. Point out any missing steps, dependencies, or prerequisites
6. Challenge the approach - is there a simpler or more robust alternative?
7. Consider what could go wrong during execution

Be harsh but constructive. Do not praise the plan. Focus entirely on what needs to be fixed or improved."#,
            task, plan
        ),
    };

    call_agent(role_config, "Advisor", &prompt)
}
