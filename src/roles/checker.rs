use anyhow::Result;
use crate::claude::call_claude;
use crate::model;

/// Check result containing pass/fail status and detailed feedback
pub struct CheckResult {
    pub passed: bool,
    pub feedback: String,
}

pub fn check(task: &str, how: &str) -> Result<CheckResult> {
    let prompt = format!(
        r#"Task that was supposed to be completed:
{}

Approach taken:
{}

Evaluate whether the CURRENT STATE fully meets ALL task requirements.

Guidelines for evaluation:
- Focus on the END RESULT, not whether changes were made (if current state already satisfies requirements, that's fine)
- PASS if the current state satisfies ALL requirements completely
- FAIL if ANY requirement is not met or only partially addressed
- Check EVERY requirement mentioned in the task, not just the main ones
- Be thorough: verify each point in the task description is satisfied
- Do NOT require proof of "changes made" - only check if the final state matches what was requested

Provide a brief analysis, then on the final line write exactly "VERDICT: PASS" or "VERDICT: FAIL""#,
        task, how
    );

    let response = call_claude("Checker", &prompt, model::CHECKER)?;
    let passed = response.to_uppercase().contains("VERDICT: PASS");

    Ok(CheckResult {
        passed,
        feedback: response,
    })
}
