use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::agent::call_agent;
use crate::config::RoleConfig;

/// A task with its parent goal context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub goal: String,
    pub description: String,
}

/// Split result containing goals and their tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitResult {
    pub goals: Vec<Goal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub name: String,
    pub tasks: Vec<String>,
}

/// Parsed task file content
#[derive(Debug, Clone)]
struct ParsedTaskFile {
    pub task_num: usize,
    pub goal: String,
    pub description: String,
    pub referenced_paths: Vec<String>,
}

/// Split a meta task into goals and their individual tasks
pub fn split(role_config: &RoleConfig, meta_task: &str) -> Result<Vec<Task>> {
    let prompt = format!(
        r#"Split this meta goal into well-structured goals and detailed tasks.

META GOAL:
{}

Rules:
- Focus on WHAT to achieve, not HOW (implementation details come later)
- Tasks should be substantial and self-contained (not micro-tasks)
- Do NOT split tasks too finely - prefer fewer, larger tasks over many small ones
- Each task runs in a separate session with NO shared memory between tasks
- The ONLY way to pass context between tasks is through explicit file references
- Every task that produces research, analysis, or documentation MUST specify an output file path
- Every task that depends on previous work MUST explicitly reference the input files to read

Context Chain Rules (CRITICAL):
- Tasks run in isolated sessions - they cannot see what previous tasks did unless you tell them which files to read
- If task N produces a document, specify: "Write output to `docs/path/filename.md`"
- If task N+1 needs that document, specify: "Read `docs/path/filename.md` for context"
- Use consistent, descriptive file paths (e.g., `docs/research/`, `docs/design/`, `docs/specs/`)
- For implementation tasks, reference all relevant design/spec documents

Task Description Guidelines:
- Each task description should be 2-4 sentences minimum
- Include the specific objective and expected outcome
- ALWAYS specify output file path if the task produces a document or artifact
- ALWAYS specify input file paths if the task depends on previous work
- Include acceptance criteria: what defines "done" for this task

Example of GOOD task descriptions with context chain:
Task 1: "Research authentication patterns and JWT implementations. Analyze security best practices and common pitfalls. Write findings to `docs/research/auth-patterns.md`. Acceptance: `docs/research/auth-patterns.md` exists with comprehensive analysis."
Task 2: "Design the authentication module based on research in `docs/research/auth-patterns.md`. Define interfaces, data flow, and security measures. Write design to `docs/design/auth-module.md`. Acceptance: `docs/design/auth-module.md` exists with complete design specification."
Task 3: "Implement authentication module following the design in `docs/design/auth-module.md`. Create the module in `src/auth/`. Acceptance: implementation matches design, tests pass."

Example of BAD task descriptions:
- "Research auth patterns" (no output file - next task can't find it)
- "Implement based on previous research" (no input file reference - task can't find the research)

Return ONLY JSON:
{{
  "goals": [
    {{
      "name": "Descriptive goal name",
      "tasks": [
        "Task 1 description. Write output to `docs/category/output-file.md`. Acceptance: file exists with required content.",
        "Task 2 description. Read `docs/category/output-file.md` for context. Write output to `docs/category/next-file.md`. Acceptance: file exists.",
        "Implementation task. Read `docs/category/next-file.md` for design. Modify `src/module/`. Acceptance: implementation complete, tests pass."
      ]
    }}
  ]
}}"#,
        meta_task
    );

    let response = call_agent(role_config, "TaskSplitter", &prompt)?;

    // Parse JSON from response
    let split_result: SplitResult = serde_json::from_str(&response)
        .or_else(|_| {
            // Try to extract JSON object if there's extra text
            let start = response.find('{').unwrap_or(0);
            let end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
            serde_json::from_str(&response[start..end])
        })
        .map_err(|e| anyhow::anyhow!("Failed to parse split JSON: {}. Response was: {}", e, response))?;

    // Flatten into Task structs with goal context
    let tasks: Vec<Task> = split_result.goals
        .into_iter()
        .flat_map(|goal| {
            goal.tasks.into_iter().map(move |desc| Task {
                goal: goal.name.clone(),
                description: desc,
            })
        })
        .collect();

    Ok(tasks)
}

/// Verify that all generated task files match the meta.md requirements
/// This function recursively scans all task-i.md files and validates them
pub fn verify_split(role_config: &RoleConfig, tasks_dir: &Path, meta_content: &str) -> Result<SplitVerification> {
    // Collect all task files
    let task_files = collect_task_files(tasks_dir)?;
    
    if task_files.is_empty() {
        return Ok(SplitVerification {
            all_goals_covered: true,
            no_redundant_goals: true,
            all_paths_valid: true,
            issues: vec!["No task files found to verify".to_string()],
        });
    }

    // Parse all task files
    let mut parsed_tasks: Vec<ParsedTaskFile> = Vec::new();
    for (task_num, path) in &task_files {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read task file: {:?}", path))?;
        let parsed = parse_task_file(*task_num, &content)?;
        parsed_tasks.push(parsed);
    }

    // Extract goals from meta content
    let meta_goals = extract_meta_goals(meta_content);
    
    // Extract goals from task files
    let task_goals: Vec<String> = parsed_tasks.iter()
        .map(|t| t.goal.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Check for missing goals (in meta but not in tasks)
    let missing_goals: Vec<String> = meta_goals.iter()
        .filter(|g| !task_goals.iter().any(|tg| goals_match(g, tg)))
        .cloned()
        .collect();

    // Check for redundant goals (in tasks but not obviously in meta)
    let potentially_redundant: Vec<String> = task_goals.iter()
        .filter(|g| !meta_goals.iter().any(|mg| goals_match(mg, g)))
        .cloned()
        .collect();

    // Validate all referenced paths
    let mut path_issues: Vec<String> = Vec::new();
    for task in &parsed_tasks {
        for path in &task.referenced_paths {
            // Check for common path issues
            if let Some(issue) = validate_path(path, tasks_dir) {
                path_issues.push(format!("Task {}: {}", task.task_num, issue));
            }
        }
    }

    // Check for context chain consistency
    let chain_issues = check_context_chain(&parsed_tasks);

    // Build verification result
    let mut issues: Vec<String> = Vec::new();
    
    if !missing_goals.is_empty() {
        issues.push(format!(
            "Missing goals (defined in meta.md but not found in tasks): {}",
            missing_goals.join("; ")
        ));
    }
    
    if !potentially_redundant.is_empty() {
        issues.push(format!(
            "Potentially redundant goals (in tasks but may not align with meta): {}",
            potentially_redundant.join("; ")
        ));
    }
    
    let has_path_issues = !path_issues.is_empty();
    issues.extend(path_issues);
    issues.extend(chain_issues);

    // Use LLM for final verification judgment
    let llm_verification = verify_with_llm(role_config, meta_content, &parsed_tasks, &issues)?;

    Ok(SplitVerification {
        all_goals_covered: missing_goals.is_empty() && llm_verification.goals_complete,
        no_redundant_goals: potentially_redundant.is_empty() || llm_verification.redundancy_acceptable,
        all_paths_valid: !has_path_issues,
        issues,
    })
}

/// Verification result for a split operation
#[derive(Debug, Clone)]
pub struct SplitVerification {
    pub all_goals_covered: bool,
    pub no_redundant_goals: bool,
    pub all_paths_valid: bool,
    pub issues: Vec<String>,
}

/// LLM verification judgment
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LLMVerification {
    goals_complete: bool,
    redundancy_acceptable: bool,
    reasoning: String,
}

/// Collect all task-i.md files from the tasks directory
fn collect_task_files(tasks_dir: &Path) -> Result<Vec<(usize, std::path::PathBuf)>> {
    let mut tasks: Vec<(usize, std::path::PathBuf)> = Vec::new();

    for entry in fs::read_dir(tasks_dir).context("Failed to read tasks directory")? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();

        if let Some(num) = parse_task_number(&filename) {
            tasks.push((num, entry.path()));
        }
    }

    tasks.sort_by_key(|(num, _)| *num);
    Ok(tasks)
}

fn parse_task_number(filename: &str) -> Option<usize> {
    let stem = filename.strip_suffix(".md")?;
    let num_str = stem.strip_prefix("task-")?;
    num_str.parse().ok()
}

/// Parse a task file to extract goal, description, and referenced paths
fn parse_task_file(task_num: usize, content: &str) -> Result<ParsedTaskFile> {
    let lines: Vec<&str> = content.lines().collect();
    
    let mut goal = String::new();
    let mut description = String::new();
    let mut in_goal = false;
    let mut in_task = false;
    let mut referenced_paths: Vec<String> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        
        if trimmed == "# Goal" {
            in_goal = true;
            in_task = false;
            continue;
        }
        
        if trimmed == "# Task" {
            in_goal = false;
            in_task = true;
            continue;
        }

        if in_goal && !trimmed.is_empty() && !trimmed.starts_with('#') {
            goal.push_str(trimmed);
            goal.push(' ');
        }

        if in_task {
            description.push_str(line);
            description.push('\n');
            
            // Extract file paths from backticks
            extract_paths_from_line(line, &mut referenced_paths);
        }
    }

    Ok(ParsedTaskFile {
        task_num,
        goal: goal.trim().to_string(),
        description: description.trim().to_string(),
        referenced_paths,
    })
}

/// Extract file paths referenced in backticks from a line
fn extract_paths_from_line(line: &str, paths: &mut Vec<String>) {
    // Look for backtick-enclosed paths that look like file paths
    let mut start = 0;
    while let Some(backtick_pos) = line[start..].find('`') {
        let abs_pos = start + backtick_pos;
        if let Some(end_pos) = line[abs_pos + 1..].find('`') {
            let content = &line[abs_pos + 1..abs_pos + 1 + end_pos];
            // Check if it looks like a file path
            if content.contains('/') || content.contains('.') || content.contains('\\') {
                if !content.starts_with("http") && !content.starts_with("www") {
                    paths.push(content.to_string());
                }
            }
            start = abs_pos + 1 + end_pos + 1;
        } else {
            break;
        }
    }
}

/// Extract goals from meta.md content
fn extract_meta_goals(meta_content: &str) -> Vec<String> {
    let mut goals: Vec<String> = Vec::new();
    
    // Look for common goal patterns in meta.md
    let lines: Vec<&str> = meta_content.lines().collect();
    
    for (_i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Pattern: "**Goal**: ..." or "Goal: ..." or "## Goal"
        if let Some(goal_start) = trimmed.find("Goal") {
            let after_goal = &trimmed[goal_start + 4..];
            if after_goal.starts_with(':') || after_goal.starts_with("**:") {
                let goal_text = after_goal.trim_start_matches(':')
                    .trim_start_matches("**:")
                    .trim();
                if !goal_text.is_empty() {
                    goals.push(goal_text.to_string());
                }
            }
        }
        
        // Pattern: "- Goal: ..." or "1. Goal: ..."
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            if let Some(colon_pos) = content.find(':') {
                let potential_goal = &content[colon_pos + 1..].trim();
                if potential_goal.len() > 10 && potential_goal.len() < 200 {
                    goals.push(potential_goal.to_string());
                }
            }
        }
    }

    // If no structured goals found, use the first paragraph as the main goal
    if goals.is_empty() {
        let first_para: String = meta_content.lines()
            .take_while(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !first_para.is_empty() {
            goals.push(first_para);
        }
    }

    goals
}

/// Check if two goals match (fuzzy matching)
fn goals_match(a: &str, b: &str) -> bool {
    let a_norm = normalize_goal(a);
    let b_norm = normalize_goal(b);
    
    // Exact match
    if a_norm == b_norm {
        return true;
    }
    
    // Substring match
    if a_norm.contains(&b_norm) || b_norm.contains(&a_norm) {
        return true;
    }
    
    // Word overlap check (at least 50% of words match)
    let a_words: std::collections::HashSet<String> = a_norm.split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let b_words: std::collections::HashSet<String> = b_norm.split_whitespace()
        .map(|s| s.to_string())
        .collect();
    
    let intersection: std::collections::HashSet<_> = a_words.intersection(&b_words).collect();
    let total_unique = a_words.union(&b_words).count();
    
    if total_unique > 0 && intersection.len() * 2 >= total_unique {
        return true;
    }
    
    false
}

fn normalize_goal(goal: &str) -> String {
    goal.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validate a referenced path
fn validate_path(path: &str, _base_dir: &Path) -> Option<String> {
    // Check for path separators
    if path.contains("\\") && !cfg!(windows) {
        return Some(format!("Path uses Windows separators: {}", path));
    }
    
    // Check for relative path issues
    if path.starts_with("../") || path.contains("/..") {
        return Some(format!("Path uses relative parent references: {}", path));
    }
    
    // Check for tilde expansion
    if path.starts_with("~") {
        return Some(format!("Path uses tilde expansion which may not work: {}", path));
    }
    
    // Check for common mistakes
    if path.ends_with("/") || path.ends_with("\\") {
        return Some(format!("Path ends with separator (likely a directory): {}", path));
    }
    
    None
}

/// Check context chain consistency between tasks
fn check_context_chain(tasks: &[ParsedTaskFile]) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();
    
    // Build a map of output files from each task
    let mut output_files: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut input_files: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    
    for task in tasks {
        // Look for "Write output to" or "Write to" patterns
        if let Some(write_pos) = task.description.to_lowercase().find("write") {
            let after_write = &task.description[write_pos..];
            // Find backtick-enclosed path after "write"
            for path in &task.referenced_paths {
                if after_write.contains(path) {
                    output_files.insert(path.clone(), task.task_num);
                }
            }
        }
        
        // Look for "Read" patterns
        if let Some(read_pos) = task.description.to_lowercase().find("read") {
            let after_read = &task.description[read_pos..];
            for path in &task.referenced_paths {
                if after_read.contains(path) {
                    input_files.entry(path.clone()).or_default().push(task.task_num);
                }
            }
        }
    }
    
    // Check that all input files have corresponding outputs from previous tasks
    for (input_path, input_tasks) in &input_files {
        if !output_files.contains_key(input_path) {
            // Check if it's a source code path (which might already exist)
            if !input_path.starts_with("src/") && !input_path.starts_with("lib/") {
                issues.push(format!(
                    "Context chain issue: Tasks {:?} read from '{}' but no task writes to this file",
                    input_tasks, input_path
                ));
            }
        } else {
            // Check ordering - inputs should come after outputs
            let output_task = output_files[input_path];
            for input_task in input_tasks {
                if *input_task <= output_task {
                    issues.push(format!(
                        "Context chain issue: Task {} reads '{}' but it's only written by task {} (should be after)",
                        input_task, input_path, output_task
                    ));
                }
            }
        }
    }
    
    issues
}

/// Use LLM for final verification judgment
fn verify_with_llm(
    role_config: &RoleConfig,
    meta_content: &str,
    tasks: &[ParsedTaskFile],
    preliminary_issues: &[String],
) -> Result<LLMVerification> {
    let tasks_summary: String = tasks.iter()
        .map(|t| format!(
            "Task {}:\n  Goal: {}\n  Description: {}\n",
            t.task_num, t.goal, t.description
        ))
        .collect::<Vec<_>>()
        .join("\n");

    let issues_text = if preliminary_issues.is_empty() {
        "No preliminary issues detected.".to_string()
    } else {
        format!("Preliminary issues:\n{}", preliminary_issues.join("\n"))
    };

    let prompt = format!(
        r#"Verify that the split tasks properly cover the meta goal.

META CONTENT:
{}

GENERATED TASKS:
{}

{}

Evaluate:
1. Are all objectives from meta.md covered by the tasks? (respond: goals_complete: true/false)
2. Are any tasks redundant or unnecessary? (respond: redundancy_acceptable: true/false - true means no concerning redundancy)
3. Provide brief reasoning

Return ONLY JSON:
{{
  "goals_complete": boolean,
  "redundancy_acceptable": boolean,
  "reasoning": "explanation"
}}"#,
        meta_content.chars().take(2000).collect::<String>(),
        tasks_summary,
        issues_text
    );

    let response = call_agent(role_config, "SplitVerifier", &prompt)?;

    // Parse JSON response
    #[derive(Deserialize)]
    struct VerificationResponse {
        #[serde(default)]
        goals_complete: bool,
        #[serde(default = "default_true")]
        redundancy_acceptable: bool,
        #[serde(default)]
        reasoning: String,
    }

    fn default_true() -> bool { true }

    let result: VerificationResponse = serde_json::from_str(&response)
        .or_else(|_| {
            let start = response.find('{').unwrap_or(0);
            let end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
            serde_json::from_str(&response[start..end])
        })
        .unwrap_or(VerificationResponse {
            goals_complete: preliminary_issues.is_empty(),
            redundancy_acceptable: true,
            reasoning: "Failed to parse verification response, using heuristic".to_string(),
        });

    Ok(LLMVerification {
        goals_complete: result.goals_complete,
        redundancy_acceptable: result.redundancy_acceptable,
        reasoning: result.reasoning,
    })
}

/// Format a task into markdown content
pub fn format_task(task: &Task, _task_index: usize, _total_tasks: usize, _meta_goal: &str) -> String {
    format!(
        r#"# Goal

{goal}

# Task

{description}
"#,
        goal = task.goal.trim(),
        description = task.description.trim()
    )
}

/// Re-split and verify tasks with full validation
pub fn split_and_verify(
    role_config: &RoleConfig,
    meta_content: &str,
    tasks_dir: &Path,
) -> Result<Vec<Task>> {
    // First, perform the split
    let tasks = split(role_config, meta_content)?;
    
    // Write tasks to files first (so verify can read them back)
    fs::create_dir_all(tasks_dir)?;
    
    for (i, task) in tasks.iter().enumerate() {
        let task_num = i + 1;
        let task_path = tasks_dir.join(format!("task-{}.md", task_num));
        let content = format_task(task, i, tasks.len(), meta_content);
        fs::write(&task_path, &content)
            .with_context(|| format!("Failed to write task-{}.md", task_num))?;
    }
    
    // Now verify the split
    println!("[Splitter] Verifying {} generated task files...", tasks.len());
    let verification = verify_split(role_config, tasks_dir, meta_content)?;
    
    // Report issues
    if !verification.issues.is_empty() {
        println!("[Splitter] Verification found {} issues:", verification.issues.len());
        for issue in &verification.issues {
            println!("  - {}", issue);
        }
    }
    
    if verification.all_goals_covered && verification.no_redundant_goals && verification.all_paths_valid {
        println!("[Splitter] ✓ All verification checks passed");
    } else {
        if !verification.all_goals_covered {
            println!("[Splitter] ⚠ Not all goals from meta.md are covered in tasks");
        }
        if !verification.no_redundant_goals {
            println!("[Splitter] ⚠ Some tasks may have redundant goals");
        }
        if !verification.all_paths_valid {
            println!("[Splitter] ⚠ Some file paths may have issues");
        }
    }
    
    Ok(tasks)
}
