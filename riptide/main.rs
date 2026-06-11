mod collector;
mod db;
mod hasher;
mod impact;
mod reporter;
mod runner;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "riptide",
    about = "⚡ Rust-powered Python test engine — parallel execution, impact analysis, coverage",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Test paths to run (files or directories)
    #[arg(default_values = &["tests", "test"])]
    paths: Vec<PathBuf>,

    /// Number of parallel workers
    #[arg(short = 'n', long, default_value = "0", help = "Workers (0 = CPU count)")]
    workers: usize,

    /// Python binary to use
    #[arg(long, default_value = "python3")]
    python: String,

    /// Enable coverage measurement
    #[arg(long, short = 'c')]
    coverage: bool,

    /// Ignore impact analysis — run all tests
    #[arg(long)]
    all: bool,

    /// File name pattern for test discovery
    #[arg(long, default_value = "test_.*\\.py|.*_test\\.py")]
    pattern: String,

    /// Path to state database
    #[arg(long, default_value = ".riptide.db")]
    db: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Collect and list all tests without running
    Collect {
        paths: Vec<PathBuf>,
        #[arg(long, default_value = "test_.*\\.py|.*_test\\.py")]
        pattern: String,
    },
    /// Clear the state database (forces full re-run next time)
    Clear {
        #[arg(long, default_value = ".riptide.db")]
        db: PathBuf,
    },
    /// Show coverage report from last run
    Coverage,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Collect { paths, pattern }) => {
            let paths = if paths.is_empty() {
                vec![PathBuf::from("tests"), PathBuf::from("test")]
            } else {
                paths.clone()
            };
            return cmd_collect(&paths, pattern);
        }
        Some(Commands::Clear { db }) => {
            return cmd_clear(db);
        }
        Some(Commands::Coverage) => {
            return cmd_coverage(&cli.python);
        }
        None => {}
    }

    cmd_run(&cli)
}

fn cmd_run(cli: &Cli) -> Result<()> {
    let paths: Vec<PathBuf> = cli.paths.iter()
        .filter(|p| p.exists())
        .cloned()
        .collect();

    if paths.is_empty() {
        eprintln!("{} No test paths found. Tried: {:?}", "error:".red().bold(), cli.paths);
        std::process::exit(1);
    }

    let db = db::Database::open(&cli.db)?;

    print!("  {} collecting tests...", "⟳".cyan());
    let all_tests = collector::collect_tests(&paths, &cli.pattern)?;
    println!("\r  {} collected {} tests", "✓".green(), all_tests.len());

    if all_tests.is_empty() {
        println!("  {} No tests found in {:?}", "!".yellow(), paths);
        return Ok(());
    }

    let current_hashes = {
        let mut all = std::collections::HashMap::new();
        for path in &paths {
            let h = hasher::hash_all_python_files(path)?;
            all.extend(h);
        }
        if let Ok(h) = hasher::hash_all_python_files(&PathBuf::from(".")) {
            all.extend(h);
        }
        all
    };

    let (to_run, skipped_tests) = if cli.all {
        println!("  {} --all flag: running all {} tests", "!".yellow(), all_tests.len());
        (all_tests.clone(), vec![])
    } else {
        let changed_files = hasher::find_changed_files(&current_hashes, &db)?;
        if changed_files.is_empty() {
            println!("  {} no files changed", "⚡".cyan());
        } else {
            println!("  {} {} file(s) changed:", "⚡".cyan(), changed_files.len());
            for f in changed_files.iter().take(5) {
                println!("    {}", f.dimmed());
            }
            if changed_files.len() > 5 {
                println!("    {} more...", changed_files.len() - 5);
            }
        }
        let analyzer = impact::ImpactAnalyzer::new(&db, changed_files);
        analyzer.filter_affected(&all_tests)?
    };

    let workers = if cli.workers == 0 { num_cpus() } else { cli.workers };

    reporter::print_header(to_run.len(), skipped_tests.len(), workers, cli.coverage);

    if to_run.is_empty() {
        println!("  {} All tests skipped — no changes detected!", "⚡".cyan().bold());
        println!("  {} Use {} to force a full run.", "tip:".dimmed(), "--all".bold());
        return Ok(());
    }

    let runner = runner::Runner::new(workers, &cli.python, cli.coverage);
    let start = Instant::now();
    let results = runner.run_parallel(&to_run)?;
    let elapsed = start.elapsed();

    for result in &results {
        db.save_test_result(result)?;
        if !result.covered_files.is_empty() {
            db.save_test_deps(&result.test_id, &result.covered_files)?;
        }
    }

    hasher::save_hashes(&current_hashes, &db)?;

    let coverage_report = if cli.coverage {
        match runner::merge_coverage(&cli.python, &runner.coverage_dir) {
            Ok(cov) => Some(cov),
            Err(e) => {
                eprintln!("  {} coverage merge failed: {}", "warn:".yellow(), e);
                None
            }
        }
    } else {
        None
    };

    reporter::print_summary(&results, &skipped_tests, elapsed, coverage_report.as_ref());

    let failed = results.iter().any(|r| {
        r.status == runner::TestStatus::Failed || r.status == runner::TestStatus::Error
    });
    if failed {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_collect(paths: &[PathBuf], pattern: &str) -> Result<()> {
    let existing: Vec<PathBuf> = paths.iter().filter(|p| p.exists()).cloned().collect();
    let existing = if existing.is_empty() { vec![PathBuf::from(".")] } else { existing };
    let tests = collector::collect_tests(&existing, pattern)?;
    println!("  {} {} tests collected:", "✓".green(), tests.len());
    for t in &tests {
        println!("  {}", t.test_id.dimmed());
    }
    Ok(())
}

fn cmd_clear(db_path: &PathBuf) -> Result<()> {
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
        println!("  {} State database cleared. Next run will execute all tests.", "✓".green());
    } else {
        println!("  {} No database found at {:?}", "!".yellow(), db_path);
    }
    Ok(())
}

fn cmd_coverage(python_bin: &str) -> Result<()> {
    let cov_dir = PathBuf::from(".riptide-coverage");
    if !cov_dir.exists() {
        println!("  {} No coverage data found. Run with {} first.", "!".yellow(), "--coverage".bold());
        return Ok(());
    }
    match runner::merge_coverage(python_bin, &cov_dir) {
        Ok(cov) => {
            let dummy_results: Vec<runner::TestResult> = vec![];
            let dummy_skipped: Vec<collector::TestItem> = vec![];
            reporter::print_summary(&dummy_results, &dummy_skipped, std::time::Duration::ZERO, Some(&cov));
        }
        Err(e) => eprintln!("  {} {}", "error:".red(), e),
    }
    Ok(())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
