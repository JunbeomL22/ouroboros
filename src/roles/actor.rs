use anyhow::Result;
use crate::agent::{call_agent_with_subagents, SubagentDef};
use crate::config::RoleConfig;

pub struct ActorOutput {
    pub how: String,
    pub result: String,
}

pub fn act(
    role_config: &RoleConfig,
    web_searcher_config: &RoleConfig,
    task: &str,
    plan: &str,
) -> Result<ActorOutput> {
    // Create web-searcher subagent definition
    let web_searcher_subagent = SubagentDef::web_searcher_from_config(web_searcher_config);

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

=== WEB SEARCH (MANDATORY) ===
IMPORTANT: You MUST use the "web-searcher" subagent for ANY web-related task.
- DO NOT use WebSearch or WebFetch tools directly - they will fail.
- ALWAYS delegate to "web-searcher" for: documentation lookups, API references, library usage, version info, external resources.
- When the plan mentions searching, researching, or looking up anything online - delegate to web-searcher.
- The web-searcher has WebSearch, WebFetch, and Read tools. You do not.

To use web-searcher, call the Task tool with:
  - subagent_type: "web-searcher"
  - description: Brief description of what to search
  - prompt: The search query or URL to fetch

Example: To search for "rust async tutorial", delegate like this:
Task(subagent_type="web-searcher", description="search rust async", prompt="Search for rust async tutorial and summarize the key concepts")

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

    let output = call_agent_with_subagents(
        role_config,
        "Actor",
        &prompt,
        Some(vec![web_searcher_subagent]),
    )?;

    parse_actor_output(&output)
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
