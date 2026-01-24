use anyhow::Result;
use crate::agent::{call_agent, call_browser_use};
use crate::config::RoleConfig;
use once_cell::sync::Lazy;
use regex::Regex;

/// Regex to detect [BROWSER]...[/BROWSER] sections in the plan
static BROWSER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[BROWSER\]([\s\S]*?)\[/BROWSER\]").unwrap());

/// Check if a plan contains browser automation sections
fn has_browser_sections(plan: &str) -> bool {
    BROWSER_REGEX.is_match(plan)
}

/// Represents a section of the plan (either regular or browser)
enum PlanSection {
    Regular(String),
    Browser(String),
}

pub struct ActorOutput {
    pub how: String,
    pub result: String,
}

pub fn act(role_config: &RoleConfig, task: &str, plan: &str) -> Result<ActorOutput> {
    // Check if plan contains browser sections
    if has_browser_sections(plan) {
        return act_with_browser(role_config, task, plan);
    }

    // Standard execution path (no browser sections)
    let prompt = format!(
        r#"Task:
{}

Plan to follow:
{}

Execute this plan and provide your response in the following format.

=== CRITICAL FILE LOCATION RULES ===
DO NOT create files in random locations like "hows/", "output/", or any arbitrary folder.
The pipeline will automatically save your output to the correct locations:
- plans/ for plan files
- results/ for result files
- advises/ for advice files
- checks/ for check files

You MUST NOT manually create result-*.md, plan-*.md, task-*.md, or similar files.
Just provide your output in the sections below - the system handles file creation.

===HOW===
Explain HOW you executed the plan. Document your process and methodology:
- What approach did you take?
- What steps did you follow?
- What tools or techniques did you use?
- Any challenges encountered and how you solved them?

===RESULT===
Provide a brief summary/guideline about the execution result:
- Where were output files written? (full paths)
- Did any operations fail? (e.g., web search blocked by security)
- What was produced and where can it be found?
Do NOT include the actual content here - just metadata about what happened.

Make sure to include both sections with the exact delimiters shown above."#,
        task, plan
    );

    let output = call_agent(role_config, "Actor", &prompt)?;
    parse_actor_output(&output)
}

/// Parse the plan into sections, identifying browser and regular sections
fn parse_plan_sections(plan: &str) -> Vec<PlanSection> {
    let mut sections = Vec::new();
    let mut last_end = 0;

    for cap in BROWSER_REGEX.captures_iter(plan) {
        let full_match = cap.get(0).unwrap();
        let browser_content = cap.get(1).unwrap().as_str().trim();

        // Add any regular content before this browser section
        if full_match.start() > last_end {
            let regular_content = &plan[last_end..full_match.start()];
            if !regular_content.trim().is_empty() {
                sections.push(PlanSection::Regular(regular_content.trim().to_string()));
            }
        }

        // Add the browser section
        if !browser_content.is_empty() {
            sections.push(PlanSection::Browser(browser_content.to_string()));
        }

        last_end = full_match.end();
    }

    // Add any remaining regular content
    if last_end < plan.len() {
        let remaining = &plan[last_end..];
        if !remaining.trim().is_empty() {
            sections.push(PlanSection::Regular(remaining.trim().to_string()));
        }
    }

    sections
}

/// Execute plan with browser sections - handles mixed regular/browser execution
fn act_with_browser(role_config: &RoleConfig, task: &str, plan: &str) -> Result<ActorOutput> {
    let sections = parse_plan_sections(plan);

    let mut all_hows = Vec::new();
    let mut all_results = Vec::new();
    let mut section_num = 0;

    for section in sections {
        section_num += 1;
        match section {
            PlanSection::Regular(content) => {
                let prompt = format!(
                    r#"Task:
{}

Plan section to execute (part of larger plan):
{}

Execute this plan section and provide your response in the following format.

=== CRITICAL FILE LOCATION RULES ===
DO NOT create files in random locations like "hows/", "output/", or any arbitrary folder.
The pipeline will automatically save your output to the correct locations.
You MUST NOT manually create result-*.md, plan-*.md, task-*.md, or similar files.

===HOW===
Explain HOW you executed this section:
- What approach did you take?
- What steps did you follow?

===RESULT===
Brief summary of what was done in this section.

Make sure to include both sections with the exact delimiters shown above."#,
                    task, content
                );

                match call_agent(role_config, "Actor", &prompt) {
                    Ok(output) => {
                        if let Ok(parsed) = parse_actor_output(&output) {
                            all_hows.push(format!("## Section {} (Regular)\n{}", section_num, parsed.how));
                            all_results.push(format!("## Section {} (Regular)\n{}", section_num, parsed.result));
                        } else {
                            all_hows.push(format!("## Section {} (Regular)\n{}", section_num, output));
                            all_results.push(format!("## Section {} (Regular)\nExecution completed (output format not standard)", section_num));
                        }
                    }
                    Err(e) => {
                        all_hows.push(format!("## Section {} (Regular)\nFailed: {}", section_num, e));
                        all_results.push(format!("## Section {} (Regular)\nError: {}", section_num, e));
                    }
                }
            }
            PlanSection::Browser(content) => {
                let browser_prompt = format!(
                    r#"Browser automation task:
{}

Instructions:
{}

Execute the browser automation and provide your response in this format:

===HOW===
Document what browser actions were taken:
- URLs navigated
- Elements interacted with
- Data extracted

===RESULT===
Summary of browser automation results:
- Screenshots taken (if any)
- Data extracted
- Final state"#,
                    task, content
                );

                match call_browser_use("Browser Actor", &browser_prompt, "") {
                    Ok(output) => {
                        if let Ok(parsed) = parse_actor_output(&output) {
                            all_hows.push(format!("## Section {} (Browser)\n{}", section_num, parsed.how));
                            all_results.push(format!("## Section {} (Browser)\n{}", section_num, parsed.result));
                        } else {
                            all_hows.push(format!("## Section {} (Browser)\n{}", section_num, output));
                            all_results.push(format!("## Section {} (Browser)\nBrowser automation completed", section_num));
                        }
                    }
                    Err(e) => {
                        all_hows.push(format!("## Section {} (Browser)\nBrowser automation failed: {}", section_num, e));
                        all_results.push(format!("## Section {} (Browser)\nError: {}", section_num, e));
                    }
                }
            }
        }
    }

    Ok(ActorOutput {
        how: all_hows.join("\n\n"),
        result: all_results.join("\n\n"),
    })
}

fn parse_actor_output(output: &str) -> Result<ActorOutput> {
    let how_marker = "===HOW===";
    let result_marker = "===RESULT===";

    let how_start = output.find(how_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "Actor output missing ===HOW=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;
    let result_start = output.find(result_marker).ok_or_else(|| {
        anyhow::anyhow!(
            "Actor output missing ===RESULT=== section.\n\nActual output received:\n---\n{}\n---",
            output
        )
    })?;

    let how = output[how_start + how_marker.len()..result_start]
        .trim()
        .to_string();
    let result = output[result_start + result_marker.len()..]
        .trim()
        .to_string();

    Ok(ActorOutput { how, result })
}
