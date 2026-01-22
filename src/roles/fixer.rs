use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

pub struct FixerOutput {
    pub how: String,
    pub result: String,
}

/// Fix only minor issues identified by the checker
pub fn fix(
    role_config: &RoleConfig,
    task: &str,
    original_how: &str,
    minor_issues: &str,
) -> Result<FixerOutput> {
    let prompt = format!(
        r#"Original Task:
{}

What was done previously:
{}

Minor issues identified by checker that need fixing:
{}

Your job is to FIX ONLY THE MINOR ISSUES listed above. Do NOT:
- Reimplement the entire solution
- Make major changes
- Add new features
- Change core functionality

Focus ONLY on addressing the specific minor issues listed. These are typically:
- Formatting problems
- Style inconsistencies
- Missing comments/documentation
- Typos
- Small cosmetic improvements
- Non-critical edge cases

=== CRITICAL FILE LOCATION RULES ===
DO NOT create files in random locations like "fixes/", "output/", or any arbitrary folder.
The pipeline will automatically save your output to the correct locations.
You MUST NOT manually create result-*.md, plan-*.md, task-*.md, or similar files.
Just provide your output in the sections below - the system handles file creation.

===HOW===
Explain HOW you fixed the minor issues:
- Which issues did you address?
- What specific changes did you make?
- Where were the changes applied?

===RESULT===
Brief summary of the fixes applied:
- List each minor issue and how it was resolved
- Any issues that could not be fixed (and why)

Make sure to include both sections with the exact delimiters shown above."#,
        task, original_how, minor_issues
    );

    let output = call_agent(role_config, "Fixer", &prompt)?;
    parse_fixer_output(&output)
}

fn parse_fixer_output(output: &str) -> Result<FixerOutput> {
    let how_marker = "===HOW===";
    let result_marker = "===RESULT===";

    let how_start = output.find(how_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "Fixer output missing ===HOW=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;
    let result_start = output.find(result_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "Fixer output missing ===RESULT=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;

    let how = output[how_start + how_marker.len()..result_start]
        .trim()
        .to_string();
    let result = output[result_start + result_marker.len()..]
        .trim()
        .to_string();

    Ok(FixerOutput { how, result })
}
