mod agent;
mod config;
mod pipeline;
mod roles;
mod secrets;

use config::{AgentConfig, Config};
use std::fs;

#[async_std::main]
async fn main() {
    let (config, secrets_path) = match Config::load() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize secrets
    if secrets_path.exists() {
        if let Err(e) = agent::init_secrets(&secrets_path) {
            eprintln!("Warning: Failed to load secrets: {}", e);
            eprintln!("Provider-specific features (e.g., MiniMax) may not work.");
        } else {
            println!("Secrets loaded from {:?}", secrets_path);
        }
    } else {
        println!("Note: {:?} not found. Using default provider settings.", secrets_path);
    }

    println!("\nOuroboros - Recursive Agent Pipeline");
    println!("Roles:");
    println!("  splitter:     {} / {} ({:?})", config.splitter.cli, config.splitter.model, config.splitter.provider);
    println!();
    println!("  outliner:     {} / {} ({:?})", config.outliner.cli, config.outliner.model, config.outliner.provider);
    println!("  advisor:      {} / {} ({:?})", config.advisor.cli, config.advisor.model, config.advisor.provider);
    println!("  planner:      {} / {} ({:?})", config.planner.cli, config.planner.model, config.planner.provider);
    println!("  actor:        {} / {} ({:?})", config.actor.cli, config.actor.model, config.actor.provider);
    println!("  checker:      {} / {} ({:?})", config.checker.cli, config.checker.model, config.checker.provider);
    println!("  minor_fixer:  {} / {} ({:?})", config.minor_fixer.cli, config.minor_fixer.model, config.minor_fixer.provider);
    println!("  major_fixer:  {} / {} ({:?})", config.major_fixer.cli, config.major_fixer.model, config.major_fixer.provider);
    println!("Directories:");
    println!("  tasks:    {:?}", config.tasks_dir);
    println!("  results:  {:?}", config.results_dir);
    println!("  plans:    {:?}", config.plans_dir);
    println!("  advises:  {:?}", config.advises_dir);
    println!("  checks:   {:?}", config.checks_dir);
    println!("  rechecks: {:?}", config.rechecks_dir);
    println!("  hows:     {:?}", config.hows_dir);
    println!("  fixes:    {:?}", config.fixes_dir);
    println!("Settings: checks={}, threshold={}, recheck_threshold={}, max_tries={}",
             config.checks, config.threshold, config.recheck_threshold, config.max_tries);

    if let Err(e) = create_directories(&config) {
        eprintln!("Failed to create directories: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = pipeline::run(&config).await {
        eprintln!("\nError: {:#}", e);
        std::process::exit(1);
    }
}

fn create_directories(config: &AgentConfig) -> std::io::Result<()> {
    fs::create_dir_all(&config.tasks_dir)?;
    fs::create_dir_all(&config.results_dir)?;
    fs::create_dir_all(&config.plans_dir)?;
    fs::create_dir_all(&config.advises_dir)?;
    fs::create_dir_all(&config.checks_dir)?;
    fs::create_dir_all(&config.rechecks_dir)?;
    fs::create_dir_all(&config.hows_dir)?;
    println!("Directories created.");
    Ok(())
}
