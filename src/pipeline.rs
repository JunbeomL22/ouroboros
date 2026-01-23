use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use async_std::task;
use futures::future::join_all;

use crate::config::AgentConfig;
use crate::roles::{plan, advise, act, check, CheckResult, IssueSeverity, PrevTaskContext, split, format_task, outline, fix_minor, fix_major};

/// Context from a completed task, loaded from files
pub struct TaskContext {
    pub how: String,
    pub result: String,
}

/// Find the highest attempt number for a given task by scanning files
fn find_highest_attempt(dir: &Path, prefix: &str, task_num: usize) -> Option<usize> {
    let pattern = format!("{}-{}-", prefix, task_num);
    let mut highest: Option<usize> = None;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with(&pattern) && filename.ends_with(".md") {
                // Extract attempt number: prefix-task-attempt.md
                let rest = filename.strip_prefix(&pattern).unwrap_or("");
                let num_str = rest.strip_suffix(".md").unwrap_or("");
                if let Ok(attempt) = num_str.parse::<usize>() {
                    highest = Some(highest.map_or(attempt, |h| h.max(attempt)));
                }
            }
        }
    }
    highest
}

/// Load previous task context from files (result and how only)
fn load_prev_task_context(config: &AgentConfig, prev_task_num: usize) -> Option<TaskContext> {
    // Find highest attempt for result files
    let result_attempt = find_highest_attempt(&config.results_dir, "result", prev_task_num)?;
    let how_attempt = find_highest_attempt(&config.hows_dir, "how", prev_task_num)?;

    let result_path = config.results_dir.join(format!("result-{}-{}.md", prev_task_num, result_attempt));
    let how_path = config.hows_dir.join(format!("how-{}-{}.md", prev_task_num, how_attempt));

    let result = fs::read_to_string(&result_path).ok()?;
    let how = fs::read_to_string(&how_path).ok()?;

    Some(TaskContext { how, result })
}

pub async fn run(config: &AgentConfig) -> Result<()> {
    // Ensure output directories exist
    fs::create_dir_all(&config.hows_dir)
        .context("Failed to create hows directory")?;
    fs::create_dir_all(&config.outlines_dir)
        .context("Failed to create outlines directory")?;
    fs::create_dir_all(&config.fixes_dir)
        .context("Failed to create fixes directory")?;
    fs::create_dir_all(&config.rechecks_dir)
        .context("Failed to create rechecks directory")?;

    // Check for meta.md and split into tasks if present
    let meta_path = config.tasks_dir.join("meta.md");
    if meta_path.exists() {
        println!("[Splitter] Found meta.md, splitting into tasks...");
        let meta_content = fs::read_to_string(&meta_path)
            .context("Failed to read meta.md")?;

        let task_descriptions = split(&config.splitter, &meta_content)
            .context("Failed to split meta task")?;

        // Find highest existing task number
        let existing_tasks = collect_tasks(&config.tasks_dir).unwrap_or_default();
        let start_num = existing_tasks.iter().map(|(n, _)| *n).max().unwrap_or(0) + 1;

        println!("[Splitter] Creating {} task files (starting from task-{})...", task_descriptions.len(), start_num);
        for (i, desc) in task_descriptions.iter().enumerate() {
            let task_num = start_num + i;
            let task_path = config.tasks_dir.join(format!("task-{}.md", task_num));
            let content = format_task(task_num, desc);
            fs::write(&task_path, &content)
                .context(format!("Failed to write task-{}.md", task_num))?;
            println!("  [Created] task-{}.md: {}", task_num, desc);
        }

        // Rename meta.md to meta.md.done to avoid re-processing
        let done_path = config.tasks_dir.join("meta.md.done");
        fs::rename(&meta_path, &done_path)
            .context("Failed to rename meta.md to meta.md.done")?;
        println!("[Splitter] Renamed meta.md -> meta.md.done");
    }

    let mut last_processed_task: usize = 0;

    loop {
        let tasks = collect_tasks(&config.tasks_dir)?;

        if tasks.is_empty() {
            println!("No task files found in {:?}", config.tasks_dir);
            println!("Create files named task-1.md, task-2.md, etc.");
            return Ok(());
        }

        // Filter to only tasks we haven't processed yet
        let new_tasks: Vec<_> = tasks
            .into_iter()
            .filter(|(num, _)| *num > last_processed_task)
            .collect();

        if new_tasks.is_empty() {
            println!("\n✓ All tasks completed successfully!");
            return Ok(());
        }

        println!("Found {} new task(s) to process", new_tasks.len());

        for (task_num, task_path) in new_tasks {
            println!("\n{}", "=".repeat(60));
            println!("Task {}", task_num);
            println!("{}", "=".repeat(60));

            // Load context from files for previous task (if task_num > 1)
            let prev_context = if task_num > 1 {
                load_prev_task_context(config, task_num - 1)
            } else {
                None
            };

            if prev_context.is_some() {
                println!("[Context] Loaded result and how from task {}", task_num - 1);
            }

            process_task(config, &task_path, task_num, prev_context.as_ref()).await?;

            // Rename completed task file to .done
            let done_path = PathBuf::from(format!("{}.done", task_path.display()));
            fs::rename(&task_path, &done_path)
                .context(format!("Failed to rename task-{}.md to task-{}.md.done", task_num, task_num))?;
            println!("[Task {}] Completed: {} -> {}",
                task_num,
                task_path.file_name().unwrap().to_string_lossy(),
                done_path.file_name().unwrap().to_string_lossy());

            last_processed_task = task_num;
        }
    }
}

fn collect_tasks(tasks_dir: &Path) -> Result<Vec<(usize, PathBuf)>> {
    let mut tasks: Vec<(usize, PathBuf)> = Vec::new();

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

/// Context from a single attempt
struct AttemptContext {
    result: String,
    how: String,
    check_feedbacks: String,
    fix_context: Option<String>,
    recheck_feedbacks: Option<String>,
}

/// Check if any results have major failures
fn has_major_failures(results: &[(usize, CheckResult)]) -> bool {
    results.iter().any(|(_, r)| r.severity == IssueSeverity::Major)
}

/// Check if any results have minor failures
fn has_minor_failures(results: &[(usize, CheckResult)]) -> bool {
    results.iter().any(|(_, r)| r.severity == IssueSeverity::Minor)
}

/// Collect major issues from check results
fn collect_major_issues(results: &[(usize, CheckResult)]) -> String {
    results
        .iter()
        .filter(|(_, r)| r.severity == IssueSeverity::Major)
        .map(|(i, r)| format!("--- Check {} (MAJOR) ---\n{}", i, r.feedback))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Collect minor issues from check results
fn collect_minor_issues(results: &[(usize, CheckResult)]) -> String {
    results
        .iter()
        .filter(|(_, r)| r.severity == IssueSeverity::Minor)
        .map(|(i, r)| {
            let issues = r.minor_issues.clone().unwrap_or_else(|| r.feedback.clone());
            format!("--- Check {} (MINOR) ---\n{}", i, issues)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

async fn process_task(
    config: &AgentConfig,
    task_path: &Path,
    task_num: usize,
    prev_context: Option<&TaskContext>,
) -> Result<()> {
    let task_content = fs::read_to_string(task_path)
        .context("Failed to read task file")?;

    let mut attempt = 1;
    let mut previous_attempts: Vec<AttemptContext> = Vec::new();

    while attempt <= config.max_retries {
        println!("\n--- Attempt {} of {} ---", attempt, config.max_retries);

        // Compute output paths with task number and attempt number
        let outline_path = config.outlines_dir.join(format!("outline-{}-{}.md", task_num, attempt));
        let plan_path = config.plans_dir.join(format!("plan-{}-{}.md", task_num, attempt));
        let advise_path = config.advises_dir.join(format!("advise-{}-{}.md", task_num, attempt));
        let result_path = config.results_dir.join(format!("result-{}-{}.md", task_num, attempt));
        let how_path = config.hows_dir.join(format!("how-{}-{}.md", task_num, attempt));

        // Build context from all previous attempts
        let (failed_results, failed_feedbacks) = if previous_attempts.is_empty() {
            (None, None)
        } else {
            let results: String = previous_attempts
                .iter()
                .enumerate()
                .map(|(i, ctx)| format!(
                    "=== Attempt {} ===\n\n## Result\n{}\n\n## How\n{}",
                    i + 1, ctx.result, ctx.how
                ))
                .collect::<Vec<_>>()
                .join("\n\n");

            let feedbacks: String = previous_attempts
                .iter()
                .enumerate()
                .map(|(i, ctx)| {
                    let mut feedback = format!(
                        "=== Attempt {} Feedback ===\n\n## Check Results\n{}",
                        i + 1, ctx.check_feedbacks
                    );
                    if let Some(fix) = &ctx.fix_context {
                        feedback.push_str(&format!("\n\n## Fix Applied\n{}", fix));
                    }
                    if let Some(recheck) = &ctx.recheck_feedbacks {
                        feedback.push_str(&format!("\n\n## Recheck Results\n{}", recheck));
                    }
                    feedback
                })
                .collect::<Vec<_>>()
                .join("\n\n");

            (Some(results), Some(feedbacks))
        };

        // Step 1: Outliner creates high-level outline
        println!("[Outliner] Creating outline...");
        let prev_task_context = prev_context.map(|c| PrevTaskContext {
            how: &c.how,
            result: &c.result,
        });
        let task_outline = match outline(
            &config.outliner,
            &task_content,
            failed_results.as_deref(),
            failed_feedbacks.as_deref(),
            prev_task_context,
        ) {
            Ok(outline) => {
                println!("[Outliner] Outline created.");
                outline
            }
            Err(e) => {
                println!("[Outliner] FAILED: {}", e);
                previous_attempts.push(AttemptContext {
                    result: format!("Outliner failed: {}", e),
                    how: String::new(),
                    check_feedbacks: String::new(),
                    fix_context: None,
                    recheck_feedbacks: None,
                });
                attempt += 1;
                continue;
            }
        };

        // Save outline file
        fs::write(&outline_path, &task_outline)
            .context("Failed to write outline file")?;
        println!("[Saved] {}", outline_path.display());

        // Step 2: Advisor reviews outline
        println!("[Advisor] Reviewing outline...");
        let advice = match advise(&config.advisor, &task_content, &task_outline, failed_results.as_deref(), failed_feedbacks.as_deref()) {
            Ok(advice) => {
                println!("[Advisor] Feedback provided.");
                advice
            }
            Err(e) => {
                println!("[Advisor] FAILED: {}", e);
                previous_attempts.push(AttemptContext {
                    result: format!("Advisor failed: {}", e),
                    how: String::new(),
                    check_feedbacks: String::new(),
                    fix_context: None,
                    recheck_feedbacks: None,
                });
                attempt += 1;
                continue;
            }
        };

        // Save advise file
        fs::write(&advise_path, &advice)
            .context("Failed to write advise file")?;
        println!("[Saved] {}", advise_path.display());

        // Step 3: Planner creates detailed plan based on outline and advice
        println!("[Planner] Creating plan from outline and feedback...");
        let prev_task_context_for_plan = prev_context.map(|c| PrevTaskContext {
            how: &c.how,
            result: &c.result,
        });
        let revised_plan = match plan(
            &config.planner,
            &task_content,
            failed_results.as_deref(),
            failed_feedbacks.as_deref(),
            Some(&format!("Outline:\n{}\n\nAdvisor Feedback:\n{}", task_outline, advice)),
            prev_task_context_for_plan,
        ) {
            Ok(plan) => {
                println!("[Planner] Plan created.");
                plan
            }
            Err(e) => {
                println!("[Planner] FAILED: {}", e);
                previous_attempts.push(AttemptContext {
                    result: format!("Planner failed: {}", e),
                    how: String::new(),
                    check_feedbacks: String::new(),
                    fix_context: None,
                    recheck_feedbacks: None,
                });
                attempt += 1;
                continue;
            }
        };

        // Save plan (include outline at the top)
        let plan_content = format!("# Outline\n\n{}\n\n# Plan\n\n{}", task_outline, revised_plan);
        fs::write(&plan_path, &plan_content)
            .context("Failed to write plan file")?;
        println!("[Saved] {}", plan_path.display());

        // Step 4: Actor executes revised plan
        println!("[Actor] Executing plan...");
        let actor_output = match act(&config.actor, &task_content, &revised_plan) {
            Ok(output) => {
                println!("[Actor] Execution complete.");
                output
            }
            Err(e) => {
                println!("[Actor] FAILED: {}", e);
                previous_attempts.push(AttemptContext {
                    result: format!("Actor failed: {}", e),
                    how: String::new(),
                    check_feedbacks: String::new(),
                    fix_context: None,
                    recheck_feedbacks: None,
                });
                attempt += 1;
                continue;
            }
        };

        // Save result file
        fs::write(&result_path, &actor_output.result)
            .context("Failed to write result file")?;
        println!("[Saved] {}", result_path.display());

        // Save how file
        fs::write(&how_path, &actor_output.how)
            .context("Failed to write how file")?;
        println!("[Saved] {}", how_path.display());

        // Step 5: Checker validates multiple times (parallel)
        println!("[Checker] Running {} validation checks in parallel...", config.checks);

        // Clone checker config for parallel checks
        let checker_config = config.checker.clone();
        let futures: Vec<_> = (1..=config.checks)
            .map(|i| {
                let task_clone = task_content.clone();
                let how_clone = actor_output.how.clone();
                let checker_clone = checker_config.clone();
                task::spawn_blocking(move || (i, check(&checker_clone, &task_clone, &how_clone)))
            })
            .collect();

        let results = join_all(futures).await;

        let mut passes = 0;
        let mut check_results: Vec<(usize, CheckResult)> = Vec::new();

        for result in results {
            match result {
                (i, Ok(check_result)) => {
                    if check_result.passed {
                        passes += 1;
                        println!("  Check {}: PASS", i);
                    } else {
                        let severity_str = match &check_result.severity {
                            IssueSeverity::Minor => "FAIL_MINOR",
                            IssueSeverity::Major => "FAIL_MAJOR",
                            IssueSeverity::None => "FAIL",
                        };
                        println!("  Check {}: {}", i, severity_str);
                    }
                    check_results.push((i, check_result));
                }
                (i, Err(e)) => {
                    println!("  Check {}: ERROR - {}", i, e);
                    check_results.push((i, CheckResult {
                        passed: false,
                        severity: IssueSeverity::Major,
                        feedback: format!("Error: {}", e),
                        minor_issues: None,
                    }));
                }
            }
        }

        // Save check results to files
        let mut check_feedbacks: Vec<String> = Vec::new();
        for (i, result) in &check_results {
            let check_path = config.checks_dir.join(format!("check-{}-{}-{}.md", task_num, attempt, i));
            let status = if result.passed {
                "PASS"
            } else {
                match &result.severity {
                    IssueSeverity::Minor => "FAIL_MINOR",
                    IssueSeverity::Major => "FAIL_MAJOR",
                    IssueSeverity::None => "FAIL",
                }
            };
            let content = format!("# Check Result: {}\n\n{}", status, result.feedback);
            fs::write(&check_path, &content)
                .context("Failed to write check file")?;
            check_feedbacks.push(result.feedback.clone());
        }

        println!("[Checker] Result: {}/{} passed (threshold: {})",
                 passes, config.checks, config.threshold);

        // Determine issue types
        let has_major = has_major_failures(&check_results);
        let has_minor = has_minor_failures(&check_results);

        // Track fix and recheck context
        let mut fix_context: Option<String> = None;
        let mut recheck_feedbacks: Vec<String> = Vec::new();
        let mut combined_how = actor_output.how.clone();

        // Early success check - all passed with no issues
        if passes >= config.threshold && !has_major && !has_minor {
            println!("[SUCCESS] Task completed - all checks passed!");
            return Ok(());
        }

        // Determine which fixer to run based on issue severity
        let fixer_ran = if has_major {
            // MAJOR issues present → run major_fixer (handles both major and minor)
            println!("[MajorFixer] Major issues detected. Attempting to fix...");

            let major_issues = collect_major_issues(&check_results);
            let minor_issues = if has_minor {
                Some(collect_minor_issues(&check_results))
            } else {
                None
            };

            let fix_path = config.fixes_dir.join(format!("fix-{}-{}-major.md", task_num, attempt));

            match fix_major(
                &config.major_fixer,
                &task_content,
                &revised_plan,
                &actor_output.how,
                &major_issues,
                minor_issues.as_deref(),
            ) {
                Ok(fixer_output) => {
                    println!("[MajorFixer] Fix complete.");
                    // Save fixer output
                    let fix_content = format!(
                        "# Major Fix Result\n\n## How\n{}\n\n## Result\n{}",
                        fixer_output.how, fixer_output.result
                    );
                    fs::write(&fix_path, &fix_content)
                        .context("Failed to write major fix file")?;
                    println!("[Saved] {}", fix_path.display());

                    // Store fix context for retry
                    fix_context = Some(format!(
                        "Major issues:\n{}\n\nMinor issues:\n{}\n\nHow fixed:\n{}\n\nResult:\n{}",
                        major_issues,
                        minor_issues.as_deref().unwrap_or("None"),
                        fixer_output.how,
                        fixer_output.result
                    ));

                    // Update combined_how for rechecks
                    combined_how = format!(
                        "{}\n\n--- Major Fixes Applied ---\n{}",
                        actor_output.how, fixer_output.how
                    );

                    true
                }
                Err(e) => {
                    println!("[MajorFixer] Failed to fix issues: {}", e);
                    false
                }
            }
        } else if has_minor {
            // Only MINOR issues → run minor_fixer
            println!("[MinorFixer] Minor issues detected. Attempting to fix...");

            let minor_issues = collect_minor_issues(&check_results);
            let fix_path = config.fixes_dir.join(format!("fix-{}-{}-minor.md", task_num, attempt));

            match fix_minor(
                &config.minor_fixer,
                &task_content,
                &actor_output.how,
                &minor_issues,
            ) {
                Ok(fixer_output) => {
                    println!("[MinorFixer] Fix complete.");
                    // Save fixer output
                    let fix_content = format!(
                        "# Minor Fix Result\n\n## How\n{}\n\n## Result\n{}",
                        fixer_output.how, fixer_output.result
                    );
                    fs::write(&fix_path, &fix_content)
                        .context("Failed to write minor fix file")?;
                    println!("[Saved] {}", fix_path.display());

                    // Store fix context for retry
                    fix_context = Some(format!(
                        "Minor issues:\n{}\n\nHow fixed:\n{}\n\nResult:\n{}",
                        minor_issues, fixer_output.how, fixer_output.result
                    ));

                    // Update combined_how for rechecks
                    combined_how = format!(
                        "{}\n\n--- Minor Fixes Applied ---\n{}",
                        actor_output.how, fixer_output.how
                    );

                    true
                }
                Err(e) => {
                    println!("[MinorFixer] Failed to fix issues: {}", e);
                    false
                }
            }
        } else {
            // No issues detected but threshold not met - skip fixer
            false
        };

        // Run rechecks only if fixer ran successfully
        if fixer_ran {
            println!("[Recheck] Running {} rechecks in parallel (threshold: {})...",
                     config.checks, config.recheck_threshold);

            let recheck_config = config.checker.clone();
            let recheck_futures: Vec<_> = (1..=config.checks)
                .map(|i| {
                    let task_clone = task_content.clone();
                    let how_clone = combined_how.clone();
                    let checker_clone = recheck_config.clone();
                    task::spawn_blocking(move || (i, check(&checker_clone, &task_clone, &how_clone)))
                })
                .collect();

            let recheck_results = join_all(recheck_futures).await;

            let mut recheck_passes = 0;
            for result in &recheck_results {
                match result {
                    (i, Ok(check_result)) => {
                        if check_result.passed {
                            recheck_passes += 1;
                            println!("  Recheck {}: PASS", i);
                        } else {
                            let severity_str = match &check_result.severity {
                                IssueSeverity::Minor => "FAIL_MINOR",
                                IssueSeverity::Major => "FAIL_MAJOR",
                                IssueSeverity::None => "FAIL",
                            };
                            println!("  Recheck {}: {}", i, severity_str);
                        }
                    }
                    (i, Err(e)) => {
                        println!("  Recheck {}: ERROR - {}", i, e);
                    }
                }
            }

            // Save recheck results to rechecks/ directory
            for (i, result) in &recheck_results {
                if let Ok(check_result) = result {
                    let recheck_path = config.rechecks_dir.join(
                        format!("recheck-{}-{}-{}.md", task_num, attempt, i)
                    );
                    let status = if check_result.passed {
                        "PASS"
                    } else {
                        match &check_result.severity {
                            IssueSeverity::Minor => "FAIL_MINOR",
                            IssueSeverity::Major => "FAIL_MAJOR",
                            IssueSeverity::None => "FAIL",
                        }
                    };
                    let content = format!(
                        "# Recheck Result: {}\n\n{}",
                        status, check_result.feedback
                    );
                    fs::write(&recheck_path, &content)
                        .context("Failed to write recheck file")?;
                    recheck_feedbacks.push(check_result.feedback.clone());
                }
            }

            println!("[Recheck] Result: {}/{} passed (threshold: {})",
                     recheck_passes, config.checks, config.recheck_threshold);

            // Check if rechecks passed
            if recheck_passes >= config.recheck_threshold {
                println!("[SUCCESS] Task completed after fix and recheck!");
                return Ok(());
            }

            // Recheck failed → go to retry (NO second fixer attempt)
            println!("[Recheck] Failed. Going directly to retry (no second fixer)...");
        }

        // Failed - collect all context for this attempt
        let combined_check_feedbacks: String = check_feedbacks
            .iter()
            .enumerate()
            .map(|(i, f)| format!("--- Check {} ---\n{}", i + 1, f))
            .collect::<Vec<_>>()
            .join("\n\n");

        let recheck_section: Option<String> = if !recheck_feedbacks.is_empty() {
            Some(recheck_feedbacks
                .iter()
                .enumerate()
                .map(|(i, f)| format!("--- Recheck {} ---\n{}", i + 1, f))
                .collect::<Vec<_>>()
                .join("\n\n"))
        } else {
            None
        };

        // Store this attempt's context
        previous_attempts.push(AttemptContext {
            result: actor_output.result,
            how: actor_output.how,
            check_feedbacks: combined_check_feedbacks,
            fix_context,
            recheck_feedbacks: recheck_section,
        });

        attempt += 1;

        if attempt <= config.max_retries {
            println!("[RETRY] Task failed. Retrying with knowledge of all {} previous attempt(s)...", previous_attempts.len());
        }
    }

    anyhow::bail!(
        "Task failed after {} retries.",
        config.max_retries
    );
}
