use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

pub struct MajorFixerOutput {
    pub how: String,
    pub result: String,
}

/// Fix major issues identified by the checker
/// Can modify logic, add missing functionality, fix bugs, etc.
/// Also handles minor issues if present alongside major ones
pub fn fix_major(
    role_config: &RoleConfig,
    task: &str,
    plan: &str,
    original_how: &str,
    major_issues: &str,
    minor_issues: Option<&str>,
) -> Result<MajorFixerOutput> {
    let minor_section = match minor_issues {
        Some(issues) => format!(
            "\n\nMinor issues also identified (fix these too if possible):\n{}",
            issues
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"Original Task:
{}

Plan that was executed:
{}

What was done previously:
{}

Major issues identified by checker that need fixing:
{}{}

Your job is to FIX THE MAJOR ISSUES listed above. You have full authority to:
- Modify logic and functionality
- Add missing features
- Fix bugs and security issues
- Make architectural adjustments
- Implement missing requirements
- Refactor code as needed

IMPORTANT:
- Focus on addressing the SPECIFIC issues listed
- Don't rewrite everything from scratch unless necessary
- Preserve working functionality
- If minor issues are also listed, fix those too while you're at it

=== CRITICAL FILE LOCATION RULES ===
DO NOT create files in random locations like "fixes/", "output/", or any arbitrary folder.
The pipeline will automatically save your output to the correct locations.
You MUST NOT manually create result-*.md, plan-*.md, task-*.md, or similar files.
Just provide your output in the sections below - the system handles file creation.

===HOW===
Explain HOW you fixed the issues:
- Which major issues did you address?
- What specific changes did you make?
- Where were the changes applied?
- Any logic or architectural changes made?
- Minor issues addressed (if any)?

===RESULT===
Brief summary of the fixes applied:
- List each major issue and how it was resolved
- List minor issues fixed (if any)
- Any issues that could not be fully fixed (and why)

Make sure to include both sections with the exact delimiters shown above."#,
        task, plan, original_how, major_issues, minor_section
    );

    let output = call_agent(role_config, "MajorFixer", &prompt)?;
    parse_fixer_output(&output)
}

fn parse_fixer_output(output: &str) -> Result<MajorFixerOutput> {
    let how_marker = "===HOW===";
    let result_marker = "===RESULT===";

    let how_start = output.find(how_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "MajorFixer output missing ===HOW=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;
    let result_start = output.find(result_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "MajorFixer output missing ===RESULT=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;

    let how = output[how_start + how_marker.len()..result_start]
        .trim()
        .to_string();
    let result = output[result_start + result_marker.len()..]
        .trim()
        .to_string();

    Ok(MajorFixerOutput { how, result })
}
