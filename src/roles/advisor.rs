use anyhow::Result;
use crate::claude::call_claude;
use crate::model;

pub fn advise(task: &str, plan: &str, failed_how: Option<&str>, check_feedbacks: Option<&str>) -> Result<String> {
    let check_context = match check_feedbacks {
        Some(feedbacks) => format!("\n\nChecker feedback from failed attempt:\n{}\n", feedbacks),
        None => String::new(),
    };

    let prompt = match failed_how {
        Some(how) => format!(
            "Task:\n{}\n\nProposed plan:\n{}\n\nPrevious failed approach:\n{}{}\n\nReview this plan. Consider what went wrong before and the checker feedback. Provide specific feedback to improve the plan.",
            task, plan, how, check_context
        ),
        None => format!(
            "Task:\n{}\n\nProposed plan:\n{}\n\nReview this plan. Identify potential issues and provide specific feedback to improve it.",
            task, plan
        ),
    };

    call_claude("Advisor", &prompt, model::ADVISOR)
}
