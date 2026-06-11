use anyhow::Result;
use colored::Colorize;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use std::sync::{Arc, Mutex};

use crate::collector::TestItem;

#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Error,
    Skipped,
}

impl TestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Error => "error",
            TestStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> TestStatus {
        match s {
            "passed" => TestStatus::Passed,
            "failed" => TestStatus::Failed,
            "skipped" => TestStatus::Skipped,
            _ => TestStatus::Error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_id: String,
    pub file_path: String,
    pub status: TestStatus,
    pub duration_ms: i64,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    /// Files touched during this test run (from coverage)
    pub covered_files: Vec<String>,
}

pub struct Runner {
    pub workers: usize,
    pub python_bin: String,
    pub with_coverage: bool,
    pub coverage_dir: PathBuf,
}

impl Runner {
    pub fn new(workers: usize, python_bin: &str, with_coverage: bool) -> Self {
        Runner {
            workers,
            python_bin: python_bin.to_string(),
            with_coverage,
            coverage_dir: PathBuf::from(".riptide-coverage"),
        }
    }

    /// Run all tests in parallel using rayon, return results
    pub fn run_parallel(&self, tests: &[TestItem]) -> Result<Vec<TestResult>> {
        if self.with_coverage {
            std::fs::create_dir_all(&self.coverage_dir)?;
        }

        rayon::ThreadPoolBuilder::new()
            .num_threads(self.workers)
            .build_global()
            .unwrap_or(()); // Ignore if already initialized

        let results: Arc<Mutex<Vec<TestResult>>> = Arc::new(Mutex::new(Vec::new()));
        let counter = Arc::new(Mutex::new(0usize));
        let total = tests.len();

        tests.par_iter().for_each(|test| {
            let result = self.run_single(test);
            
            {
                let mut count = counter.lock().unwrap();
                *count += 1;
                let n = *count;
                
                match &result {
                    Ok(r) => print_progress(n, total, r),
                    Err(e) => eprintln!("  {} [ERROR] {}: {}", "✗".red(), test.test_id, e),
                }
            }

            if let Ok(r) = result {
                results.lock().unwrap().push(r);
            }
        });

        let final_results = Arc::try_unwrap(results)
            .unwrap()
            .into_inner()
            .unwrap();

        Ok(final_results)
    }

    /// Run a single test in its own pytest subprocess
    fn run_single(&self, test: &TestItem) -> Result<TestResult> {
        let start = Instant::now();
        let node_id = test.pytest_nodeid();

        // Build coverage data file path (unique per test to avoid conflicts)
        let safe_id = test.test_id.replace(['/', ':', '.'], "_");
        let cov_file = self.coverage_dir.join(format!(".coverage.{}", safe_id));

        let mut cmd = Command::new(&self.python_bin);
        cmd.arg("-m");

        if self.with_coverage {
            // Run via coverage run --data-file=<unique> -m pytest <nodeid>
            cmd.args([
                "coverage", "run",
                "--data-file", cov_file.to_str().unwrap_or(".coverage_tmp"),
                "--source=.",
                "--branch",
                "-m", "pytest",
                &node_id,
                "-x",
                "--tb=short",
                "-q",
                "--no-header",
            ]);
        } else {
            cmd.args([
                "pytest",
                &node_id,
                "-x",
                "--tb=short",
                "-q",
                "--no-header",
            ]);
        }

        let output = cmd.output()?;
        let duration_ms = start.elapsed().as_millis() as i64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let status = parse_status(&stdout, &stderr, output.status.success());

        // Extract covered files from coverage data
        let covered_files = if self.with_coverage && cov_file.exists() {
            extract_covered_files(&self.python_bin, &cov_file)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(TestResult {
            test_id: test.test_id.clone(),
            file_path: test.file_path.clone(),
            status,
            duration_ms,
            stdout: Some(stdout),
            stderr: if stderr.is_empty() { None } else { Some(stderr) },
            covered_files,
        })
    }
}

/// Parse test status from pytest output
fn parse_status(stdout: &str, _stderr: &str, exit_success: bool) -> TestStatus {
    let combined = stdout.to_lowercase();
    if combined.contains(" passed") && !combined.contains(" failed") && !combined.contains(" error") {
        return TestStatus::Passed;
    }
    if combined.contains(" failed") || combined.contains("assertionerror") || combined.contains("assert ") {
        return TestStatus::Failed;
    }
    if combined.contains(" skipped") && !combined.contains(" failed") {
        return TestStatus::Skipped;
    }
    if combined.contains("error") {
        return TestStatus::Error;
    }
    if exit_success {
        TestStatus::Passed
    } else {
        TestStatus::Failed
    }
}

/// Use `coverage json` to extract which files were covered
fn extract_covered_files(python_bin: &str, cov_file: &PathBuf) -> Result<Vec<String>> {
    let json_file = cov_file.with_extension("json");

    let output = Command::new(python_bin)
        .args([
            "-m", "coverage", "json",
            "--data-file", cov_file.to_str().unwrap_or(".coverage"),
            "-o", json_file.to_str().unwrap_or("coverage.json"),
            "-q",
        ])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let json_str = std::fs::read_to_string(&json_file)?;
    let v: serde_json::Value = serde_json::from_str(&json_str)?;

    let files: Vec<String> = v["files"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    // Clean up temp files
    let _ = std::fs::remove_file(&json_file);

    Ok(files)
}

fn print_progress(n: usize, total: usize, result: &TestResult) {
    let icon = match result.status {
        TestStatus::Passed => "✓".green().to_string(),
        TestStatus::Failed => "✗".red().to_string(),
        TestStatus::Error => "E".yellow().to_string(),
        TestStatus::Skipped => "s".dimmed().to_string(),
    };
    let duration = format!("{}ms", result.duration_ms).dimmed();
    println!(
        "  {} [{}/{}] {} {}",
        icon,
        n,
        total,
        result.test_id.dimmed(),
        duration
    );
}

/// Merge per-test coverage .coverage files into one via coverage combine
pub fn merge_coverage(python_bin: &str, coverage_dir: &PathBuf) -> Result<HashMap<String, CoverageInfo>> {
    // Combine all .coverage.* files
    let combine_output = Command::new(python_bin)
        .args(["-m", "coverage", "combine", "--keep",
               coverage_dir.to_str().unwrap_or(".")])
        .output()?;

    // Generate JSON report
    let json_output = Command::new(python_bin)
        .args(["-m", "coverage", "json", "-o", ".riptide-coverage/combined.json", "-q"])
        .output()?;

    if !json_output.status.success() {
        return Ok(HashMap::new());
    }

    let json_str = std::fs::read_to_string(".riptide-coverage/combined.json")?;
    let v: serde_json::Value = serde_json::from_str(&json_str)?;

    let mut coverage_map = HashMap::new();

    if let Some(files) = v["files"].as_object() {
        for (file, data) in files {
            let executed: Vec<u32> = data["executed_lines"]
                .as_array()
                .map(|a| a.iter().filter_map(|n| n.as_u64().map(|x| x as u32)).collect())
                .unwrap_or_default();
            let missing: Vec<u32> = data["missing_lines"]
                .as_array()
                .map(|a| a.iter().filter_map(|n| n.as_u64().map(|x| x as u32)).collect())
                .unwrap_or_default();
            let total = (executed.len() + missing.len()) as u32;
            let pct = if total > 0 {
                (executed.len() as f64 / total as f64) * 100.0
            } else {
                100.0
            };
            coverage_map.insert(file.clone(), CoverageInfo {
                executed_lines: executed.len() as u32,
                total_lines: total,
                percentage: pct,
            });
        }
    }

    Ok(coverage_map)
}

#[derive(Debug)]
pub struct CoverageInfo {
    pub executed_lines: u32,
    pub total_lines: u32,
    pub percentage: f64,
}
