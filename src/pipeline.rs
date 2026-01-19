use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use async_std::task;
use futures::future::join_all;

use crate::config::Config;
use crate::roles::{plan, advise, act, check, CheckResult, PrevTaskContext};

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
fn load_prev_task_context(config: &Config, prev_task_num: usize) -> Option<TaskContext> {
    // Find highest attempt for result files
    let result_attempt = find_highest_attempt(&config.results_dir, "result", prev_task_num)?;
    let how_attempt = find_highest_attempt(&config.hows_dir, "how", prev_task_num)?;

    let result_path = config.results_dir.join(format!("result-{}-{}.md", prev_task_num, result_attempt));
    let how_path = config.hows_dir.join(format!("how-{}-{}.md", prev_task_num, how_attempt));

    let result = fs::read_to_string(&result_path).ok()?;
    let how = fs::read_to_string(&how_path).ok()?;

    Some(TaskContext { how, result })
}

pub async fn run(config: &Config) -> Result<()> {
    // Ensure hows directory exists
    fs::create_dir_all(&config.hows_dir)
        .context("Failed to create hows directory")?;

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

async fn process_task(
    config: &Config,
    task_path: &Path,
    task_num: usize,
    prev_context: Option<&TaskContext>,
) -> Result<()> {
    let task_content = fs::read_to_string(task_path)
        .context("Failed to read task file")?;

    let mut attempt = 1;
    let mut failed_result: Option<String> = None;
    let mut failed_check_feedbacks: Option<String> = None;

    while attempt <= config.max_retries {
        println!("\n--- Attempt {} of {} ---", attempt, config.max_retries);

        // Compute output paths with task number and attempt number
        let plan_path = config.plans_dir.join(format!("plan-{}-{}.md", task_num, attempt));
        let advise_path = config.advises_dir.join(format!("advise-{}-{}.md", task_num, attempt));
        let result_path = config.results_dir.join(format!("result-{}-{}.md", task_num, attempt));
        let how_path = config.hows_dir.join(format!("how-{}-{}.md", task_num, attempt));

        // Step 1: Planner creates initial plan
        println!("[Planner] Creating plan...");
        let prev_task_context = prev_context.map(|c| PrevTaskContext {
            how: &c.how,
            result: &c.result,
        });
        let initial_plan = plan(
            &task_content,
            failed_result.as_deref(),
            failed_check_feedbacks.as_deref(),
            None,
            prev_task_context,
        )?;
        println!("[Planner] Plan created.");

        // Save initial plan before advising
        fs::write(&plan_path, &initial_plan)
            .context("Failed to write initial plan file")?;
        println!("[Saved] {} (initial)", plan_path.display());

        // Step 2: Advisor reviews plan
        println!("[Advisor] Reviewing plan...");
        let advice = advise(&task_content, &initial_plan, failed_result.as_deref(), failed_check_feedbacks.as_deref())?;
        println!("[Advisor] Feedback provided.");

        // Save advise file
        fs::write(&advise_path, &advice)
            .context("Failed to write advise file")?;
        println!("[Saved] {}", advise_path.display());

        // Step 3: Planner revises based on advice
        println!("[Planner] Revising plan based on feedback...");
        let prev_task_context_revised = prev_context.map(|c| PrevTaskContext {
            how: &c.how,
            result: &c.result,
        });
        let revised_plan = plan(
            &task_content,
            failed_result.as_deref(),
            failed_check_feedbacks.as_deref(),
            Some(&advice),
            prev_task_context_revised,
        )?;
        println!("[Planner] Plan revised.");

        // Save revised plan (overwrites initial)
        fs::write(&plan_path, &revised_plan)
            .context("Failed to write revised plan file")?;
        println!("[Saved] {} (revised)", plan_path.display());

        // Step 4: Actor executes revised plan
        println!("[Actor] Executing plan...");
        let actor_output = act(&task_content, &revised_plan)?;
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

        let futures: Vec<_> = (1..=config.checks)
            .map(|i| {
                let task_clone = task_content.clone();
                let how_clone = actor_output.how.clone();
                task::spawn_blocking(move || (i, check(&task_clone, &how_clone)))
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
                        println!("  Check {}: FAIL", i);
                    }
                    check_results.push((i, check_result));
                }
                (i, Err(e)) => {
                    println!("  Check {}: ERROR - {}", i, e);
                    check_results.push((i, CheckResult {
                        passed: false,
                        feedback: format!("Error: {}", e),
                    }));
                }
            }
        }

        // Save check results to files
        let mut check_feedbacks: Vec<String> = Vec::new();
        for (i, result) in &check_results {
            let check_path = config.checks_dir.join(format!("check-{}-{}-{}.md", task_num, attempt, i));
            let status = if result.passed { "PASS" } else { "FAIL" };
            let content = format!("# Check Result: {}\n\n{}", status, result.feedback);
            fs::write(&check_path, &content)
                .context("Failed to write check file")?;
            check_feedbacks.push(result.feedback.clone());
        }

        println!("[Checker] Result: {}/{} passed (threshold: {})",
                 passes, config.checks, config.threshold);

        if passes >= config.threshold {
            println!("[SUCCESS] Task completed!");
            return Ok(());
        }

        // Failed - prepare for retry
        failed_result = Some(actor_output.result);
        // Collect all check feedbacks for the next attempt
        let combined_feedbacks: String = check_feedbacks
            .iter()
            .enumerate()
            .map(|(i, f)| format!("--- Check {} ---\n{}", i + 1, f))
            .collect::<Vec<_>>()
            .join("\n\n");
        failed_check_feedbacks = Some(combined_feedbacks);
        attempt += 1;

        if attempt <= config.max_retries {
            println!("[RETRY] Task failed. Retrying with knowledge of failed approach...");
        }
    }

    anyhow::bail!(
        "Task failed after {} retries.",
        config.max_retries
    );
}
