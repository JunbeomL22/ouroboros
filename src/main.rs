mod agent;
mod config;
mod pipeline;
mod roles;

use config::{AgentConfig, Config};
use std::fs;

#[async_std::main]
async fn main() {
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    println!("Ouroboros - Recursive Agent Pipeline");
    println!("Roles:");
    println!("  outliner:     {} / {}", config.outliner.cli, config.outliner.model);
    println!("  advisor:      {} / {}", config.advisor.cli, config.advisor.model);
    println!("  planner:      {} / {}", config.planner.cli, config.planner.model);
    println!("  actor:        {} / {}", config.actor.cli, config.actor.model);
    println!("  checker:      {} / {}", config.checker.cli, config.checker.model);
    println!("  minor_fixer:  {} / {}", config.minor_fixer.cli, config.minor_fixer.model);
    println!("  major_fixer:  {} / {}", config.major_fixer.cli, config.major_fixer.model);
    println!("  splitter:     {} / {}", config.splitter.cli, config.splitter.model);
    println!("Directories:");
    println!("  tasks:    {:?}", config.tasks_dir);
    println!("  results:  {:?}", config.results_dir);
    println!("  plans:    {:?}", config.plans_dir);
    println!("  advises:  {:?}", config.advises_dir);
    println!("  checks:   {:?}", config.checks_dir);
    println!("  rechecks: {:?}", config.rechecks_dir);
    println!("  hows:     {:?}", config.hows_dir);
    println!("  fixes:    {:?}", config.fixes_dir);
    println!("Settings: checks={}, threshold={}, recheck_threshold={}, max_retries={}",
             config.checks, config.threshold, config.recheck_threshold, config.max_retries);

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
