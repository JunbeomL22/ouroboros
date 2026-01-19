mod config;
mod claude;
mod model;
mod pipeline;
mod roles;

use config::Config;
use std::fs;

#[async_std::main]
async fn main() {
    let config = Config::parse_args();

    println!("Ouroboros - Recursive Claude Pipeline");
    println!("Tasks dir: {:?}", config.tasks_dir);
    println!("Results dir: {:?}", config.results_dir);
    println!("Plans dir: {:?}", config.plans_dir);
    println!("Advises dir: {:?}", config.advises_dir);
    println!("Checks dir: {:?}", config.checks_dir);
    println!("Checks: {}, Threshold: {}, Max retries: {}",
             config.checks, config.threshold, config.max_retries);

    // Create all directories upfront at launch
    if let Err(e) = create_directories(&config) {
        eprintln!("Failed to create directories: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = pipeline::run(&config).await {
        eprintln!("\nError: {:#}", e);
        std::process::exit(1);
    }
}

fn create_directories(config: &Config) -> std::io::Result<()> {
    fs::create_dir_all(&config.tasks_dir)?;
    fs::create_dir_all(&config.results_dir)?;
    fs::create_dir_all(&config.plans_dir)?;
    fs::create_dir_all(&config.advises_dir)?;
    fs::create_dir_all(&config.checks_dir)?;
    println!("All directories created successfully.");
    Ok(())
}
