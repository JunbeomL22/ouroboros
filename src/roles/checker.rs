use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

/// Issue severity classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    /// All requirements met
    None,
    /// Minor issues: formatting, style, small improvements, documentation, typos
    Minor,
    /// Major issues: missing functionality, broken logic, security problems, core requirements not met
    Major,
}

/// Check result containing pass/fail status, severity, and detailed feedback
pub struct CheckResult {
    pub passed: bool,
    pub severity: IssueSeverity,
    pub feedback: String,
    /// Specific minor issues identified (if any)
    pub minor_issues: Option<String>,
}

pub fn check(
    role_config: &RoleConfig,
    task: &str,
    plan: &str,
    how: &str,
    result: &str,
) -> Result<CheckResult> {
    let prompt = format!(
        r#"Task that was supposed to be completed:
{}

Plan that was executed:
{}

Approach taken (how it was done):
{}

Execution result summary:
{}

Evaluate whether the CURRENT STATE fully meets ALL task requirements.

Guidelines for evaluation:
- Focus on the END RESULT, not whether changes were made (if current state already satisfies requirements, that's fine)
- PASS if the current state satisfies ALL requirements completely
- FAIL if ANY requirement is not met or only partially addressed
- Check EVERY requirement mentioned in the task, not just the main ones
- Be thorough: verify each point in the task description is satisfied
- Do NOT require proof of "changes made" - only check if the final state matches what was requested

Issue Classification (when FAIL):
- MINOR issues: formatting problems, style inconsistencies, missing comments/documentation, typos,
  small cosmetic improvements, non-critical edge cases, minor code cleanup opportunities
- MAJOR issues: missing core functionality, broken logic, security vulnerabilities, performance problems,
  core requirements not implemented, architectural problems, breaking changes

Provide a brief analysis, then at the end:
1. If PASS: Write "VERDICT: PASS"
2. If FAIL with only MINOR issues: Write "VERDICT: FAIL_MINOR" followed by a new line with "MINOR_ISSUES:"
   and list each minor issue on its own line starting with "- "
3. If FAIL with any MAJOR issues: Write "VERDICT: FAIL_MAJOR""#,
        task, plan, how, result
    );

    let response = call_agent(role_config, "Checker", &prompt)?;
    let upper_response = response.to_uppercase();

    let (passed, severity, minor_issues) = if upper_response.contains("VERDICT: PASS") {
        (true, IssueSeverity::None, None)
    } else if upper_response.contains("VERDICT: FAIL_MINOR") {
        // Extract minor issues from the response
        let minor_issues = extract_minor_issues(&response);
        (false, IssueSeverity::Minor, minor_issues)
    } else {
        // FAIL_MAJOR or just FAIL (treat as major)
        (false, IssueSeverity::Major, None)
    };

    Ok(CheckResult {
        passed,
        severity,
        feedback: response,
        minor_issues,
    })
}

/// Extract minor issues from checker response
fn extract_minor_issues(response: &str) -> Option<String> {
    // Find "MINOR_ISSUES:" section and extract the list
    let lower = response.to_lowercase();
    if let Some(start) = lower.find("minor_issues:") {
        let rest = &response[start + "minor_issues:".len()..];
        // Collect lines starting with "- " until we hit an empty line or end
        let issues: Vec<&str> = rest
            .lines()
            .map(|l| l.trim())
            .take_while(|l| !l.is_empty() || l.starts_with('-'))
            .filter(|l| l.starts_with('-'))
            .collect();

        if !issues.is_empty() {
            return Some(issues.join("\n"));
        }
    }
    None
}
