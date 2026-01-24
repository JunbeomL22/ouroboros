use anyhow::Result;
use crate::agent::call_agent;
use crate::config::RoleConfig;

pub struct MinorFixerOutput {
    pub how: String,
    pub result: String,
}

/// Fix only minor/cosmetic issues identified by the checker
/// Does NOT modify logic or functionality - only formatting, style, comments, typos, etc.
pub fn fix_minor(
    role_config: &RoleConfig,
    task: &str,
    plan: &str,
    original_how: &str,
    result: &str,
    minor_issues: &str,
) -> Result<MinorFixerOutput> {
    let prompt = format!(
        r#"Original Task:
{}

Plan that was executed:
{}

What was done previously:
{}

Execution result summary:
{}

Minor issues identified by checker that need fixing:
{}

Your job is to FIX ONLY THE MINOR/COSMETIC ISSUES listed above.

ALLOWED fixes (do these):
- Formatting problems (indentation, spacing, line breaks)
- Style inconsistencies (naming conventions, code style)
- Missing comments/documentation
- Typos in strings, comments, or identifiers
- Small cosmetic improvements
- Non-critical edge case handling

FORBIDDEN (do NOT do any of these):
- Changing any logic or functionality
- Adding new features
- Fixing bugs or security issues
- Modifying core behavior
- Architectural changes
- Performance optimizations

If an issue requires changing logic, skip it and note that it requires a major fix.

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
- Any issues skipped because they require logic changes?

===RESULT===
Brief summary of the fixes applied:
- List each minor issue and how it was resolved
- Any issues that could not be fixed (and why)

Make sure to include both sections with the exact delimiters shown above."#,
        task, plan, original_how, result, minor_issues
    );

    let output = call_agent(role_config, "MinorFixer", &prompt)?;
    parse_fixer_output(&output)
}

fn parse_fixer_output(output: &str) -> Result<MinorFixerOutput> {
    let how_marker = "===HOW===";
    let result_marker = "===RESULT===";

    let how_start = output.find(how_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "MinorFixer output missing ===HOW=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;
    let result_start = output.find(result_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "MinorFixer output missing ===RESULT=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;

    let how = output[how_start + how_marker.len()..result_start]
        .trim()
        .to_string();
    let result = output[result_start + result_marker.len()..]
        .trim()
        .to_string();

    Ok(MinorFixerOutput { how, result })
}
