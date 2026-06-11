use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct TestItem {
    /// Unique ID: "path/to/test_file.py::test_function_name"
    pub test_id: String,
    pub file_path: String,
    pub function_name: String,
    pub class_name: Option<String>,
}

impl TestItem {
    pub fn pytest_nodeid(&self) -> String {
        match &self.class_name {
            Some(cls) => format!("{}::{}::{}", self.file_path, cls, self.function_name),
            None => format!("{}::{}", self.file_path, self.function_name),
        }
    }
}

/// Discover all test items in the given paths
pub fn collect_tests(paths: &[PathBuf], pattern: &str) -> Result<Vec<TestItem>> {
    let mut items = Vec::new();
    let file_re = Regex::new(pattern)?;

    for path in paths {
        if path.is_file() {
            if path.extension().map_or(false, |e| e == "py") {
                collect_from_file(path, &mut items)?;
            }
        } else {
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_file()
                        && p.extension().map_or(false, |ext| ext == "py")
                        && file_re.is_match(&p.file_name().unwrap_or_default().to_string_lossy())
                        && !p.components().any(|c| {
                            let s = c.as_os_str().to_string_lossy();
                            s == ".git" || s == "__pycache__" || s == ".venv" || s == "venv"
                        })
                })
            {
                collect_from_file(entry.path(), &mut items)?;
            }
        }
    }

    Ok(items)
}

/// Parse a Python file and extract test function names (fast regex-based, no AST)
fn collect_from_file(path: &Path, items: &mut Vec<TestItem>) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let file_path = path.to_string_lossy().to_string();

    // Match class definitions
    let class_re = Regex::new(r"^class\s+(Test\w*)\s*[:(]")?;
    // Match test functions (top-level or inside class)
    let func_re = Regex::new(r"^(\s*)def\s+(test_\w+)\s*\(")?;

    let mut current_class: Option<(String, usize)> = None; // (name, indent_level)

    for line in content.lines() {
        // Check for class definition
        if let Some(caps) = class_re.captures(line) {
            let class_name = caps[1].to_string();
            current_class = Some((class_name, 0));
            continue;
        }

        // Check for function definition
        if let Some(caps) = func_re.captures(line) {
            let indent = caps[1].len();
            let func_name = caps[2].to_string();

            // Determine if inside a class
            let class_name = if indent > 0 {
                current_class.as_ref().map(|(n, _)| n.clone())
            } else {
                // Top-level function — clear current class
                current_class = None;
                None
            };

            let test_id = match &class_name {
                Some(cls) => format!("{}::{}::{}", file_path, cls, func_name),
                None => format!("{}::{}", file_path, func_name),
            };

            items.push(TestItem {
                test_id,
                file_path: file_path.clone(),
                function_name: func_name,
                class_name,
            });
        }
    }

    Ok(())
}
