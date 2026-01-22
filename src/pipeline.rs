use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use async_std::task;
use futures::future::join_all;

use crate::config::AgentConfig;
use crate::roles::{plan, advise, act, check, CheckResult, IssueSeverity, PrevTaskContext, split, format_task, outline, fix};

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
    verification_feedbacks: Option<String>,
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
                    if let Some(verify) = &ctx.verification_feedbacks {
                        feedback.push_str(&format!("\n\n## Verification Results\n{}", verify));
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
        let task_outline = outline(
            &config.outliner,
            &task_content,
            failed_results.as_deref(),
            failed_feedbacks.as_deref(),
            prev_task_context,
        )?;
        println!("[Outliner] Outline created.");

        // Save outline file
        fs::write(&outline_path, &task_outline)
            .context("Failed to write outline file")?;
        println!("[Saved] {}", outline_path.display());

        // Step 2: Advisor reviews outline
        println!("[Advisor] Reviewing outline...");
        let advice = advise(&config.advisor, &task_content, &task_outline, failed_results.as_deref(), failed_feedbacks.as_deref())?;
        println!("[Advisor] Feedback provided.");

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
        let revised_plan = plan(
            &config.planner,
            &task_content,
            failed_results.as_deref(),
            failed_feedbacks.as_deref(),
            Some(&format!("Outline:\n{}\n\nAdvisor Feedback:\n{}", task_outline, advice)),
            prev_task_context_for_plan,
        )?;
        println!("[Planner] Plan created.");

        // Save plan (include outline at the top)
        let plan_content = format!("# Outline\n\n{}\n\n# Plan\n\n{}", task_outline, revised_plan);
        fs::write(&plan_path, &plan_content)
            .context("Failed to write plan file")?;
        println!("[Saved] {}", plan_path.display());

        // Step 4: Actor executes revised plan
        println!("[Actor] Executing plan...");
        let actor_output = act(&config.actor, &task_content, &revised_plan)?;
        println!("[Actor] Execution complete.");

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

        // Check if any checks failed with minor issues
        let has_minor_failures = check_results
            .iter()
            .any(|(_, r)| r.severity == IssueSeverity::Minor);

        // Track fix and verification context
        let mut fix_context: Option<String> = None;
        let mut verification_feedbacks: Vec<String> = Vec::new();
        let mut final_passes = passes;

        // If there are any minor issues, run fixer (even if threshold already passed)
        if has_minor_failures {
            println!("[Fixer] Minor issues detected. Attempting to fix...");

            // Collect minor issues, falling back to full feedback if specific issues weren't extracted
            let combined_minor_issues: String = check_results
                .iter()
                .filter(|(_, r)| r.severity == IssueSeverity::Minor)
                .map(|(_, r)| r.minor_issues.clone().unwrap_or_else(|| r.feedback.clone()))
                .collect::<Vec<_>>()
                .join("\n\n");
            let fix_path = config.fixes_dir.join(format!("fix-{}-{}.md", task_num, attempt));

            match fix(&config.fixer, &task_content, &actor_output.how, &combined_minor_issues) {
                Ok(fixer_output) => {
                    // Save fixer output
                    let fix_content = format!(
                        "# Fix Result\n\n## How\n{}\n\n## Result\n{}",
                        fixer_output.how, fixer_output.result
                    );
                    fs::write(&fix_path, &fix_content)
                        .context("Failed to write fix file")?;
                    println!("[Saved] {}", fix_path.display());

                    // Store fix context for retry
                    fix_context = Some(format!(
                        "Minor issues fixed:\n{}\n\nHow fixed:\n{}\n\nResult:\n{}",
                        combined_minor_issues, fixer_output.how, fixer_output.result
                    ));

                    // Re-run checker to verify fixes
                    println!("[Checker] Verifying fixes...");

                    // Combine original how with fixer how for verification
                    let combined_how = format!(
                        "{}\n\n--- Fixes Applied ---\n{}",
                        actor_output.how, fixer_output.how
                    );

                    let verify_config = config.checker.clone();
                    let verify_futures: Vec<_> = (1..=config.checks)
                        .map(|i| {
                            let task_clone = task_content.clone();
                            let how_clone = combined_how.clone();
                            let checker_clone = verify_config.clone();
                            task::spawn_blocking(move || (i, check(&checker_clone, &task_clone, &how_clone)))
                        })
                        .collect();

                    let verify_results = join_all(verify_futures).await;

                    let mut verify_passes = 0;
                    for result in &verify_results {
                        match result {
                            (i, Ok(check_result)) => {
                                if check_result.passed {
                                    verify_passes += 1;
                                    println!("  Verify {}: PASS", i);
                                } else {
                                    let severity_str = match &check_result.severity {
                                        IssueSeverity::Minor => "FAIL_MINOR",
                                        IssueSeverity::Major => "FAIL_MAJOR",
                                        IssueSeverity::None => "FAIL",
                                    };
                                    println!("  Verify {}: {}", i, severity_str);
                                }
                            }
                            (i, Err(e)) => {
                                println!("  Verify {}: ERROR - {}", i, e);
                            }
                        }
                    }

                    // Save verification results and collect feedback
                    for (i, result) in &verify_results {
                        if let Ok(check_result) = result {
                            let verify_path = config.checks_dir.join(
                                format!("check-{}-{}-{}-verified.md", task_num, attempt, i)
                            );
                            let status = if check_result.passed { "PASS" } else { "FAIL" };
                            let content = format!(
                                "# Verification Check Result: {}\n\n{}",
                                status, check_result.feedback
                            );
                            fs::write(&verify_path, &content)
                                .context("Failed to write verification check file")?;
                            verification_feedbacks.push(check_result.feedback.clone());
                        }
                    }

                    println!("[Checker] Verification: {}/{} passed (threshold: {})",
                             verify_passes, config.checks, config.threshold);

                    // Use verification passes as the final count
                    final_passes = verify_passes;
                }
                Err(e) => {
                    println!("[Fixer] Failed to fix minor issues: {}. Using original check results...", e);
                }
            }
        }

        // Check if task completed (either original passes or verification passes met threshold)
        if final_passes >= config.threshold {
            println!("[SUCCESS] Task completed!");
            return Ok(());
        }

        // Failed - collect all context for this attempt
        let combined_check_feedbacks: String = check_feedbacks
            .iter()
            .enumerate()
            .map(|(i, f)| format!("--- Check {} ---\n{}", i + 1, f))
            .collect::<Vec<_>>()
            .join("\n\n");

        let verification_section: Option<String> = if !verification_feedbacks.is_empty() {
            Some(verification_feedbacks
                .iter()
                .enumerate()
                .map(|(i, f)| format!("--- Verification Check {} ---\n{}", i + 1, f))
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
            verification_feedbacks: verification_section,
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
