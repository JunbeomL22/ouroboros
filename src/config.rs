use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "ouroboros")]
#[command(about = "Recursive Claude pipeline for sequential task processing")]
pub struct Config {
    /// Tasks directory containing task-1.md, task-2.md, etc.
    #[arg(long, default_value = "./tasks")]
    pub tasks_dir: PathBuf,

    /// Results directory for output files result-1.md, result-2.md, etc.
    #[arg(long, default_value = "./results")]
    pub results_dir: PathBuf,

    /// Plans directory for plan files plan-1.md, plan-2.md, etc.
    #[arg(long, default_value = "./plans")]
    pub plans_dir: PathBuf,

    /// Advises directory for advice files advise-1.md, advise-2.md, etc.
    #[arg(long, default_value = "./advises")]
    pub advises_dir: PathBuf,

    /// Checks directory for check result files check-1-1.md, check-1-2.md, etc.
    #[arg(long, default_value = "./checks")]
    pub checks_dir: PathBuf,

    /// Hows directory for how files how-1-1.md, how-1-2.md, etc.
    #[arg(long, default_value = "./hows")]
    pub hows_dir: PathBuf,

    /// Number of checker runs per task
    #[arg(long, default_value = "4")]
    pub checks: usize,

    /// Threshold for passing (must pass this many checks)
    #[arg(long, default_value = "4")]
    pub threshold: usize,

    /// Maximum retry attempts before failing
    #[arg(long, default_value = "4")]
    pub max_retries: usize,
}

impl Config {
    pub fn parse_args() -> Self {
        Config::parse()
    }
}
