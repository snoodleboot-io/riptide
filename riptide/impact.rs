use anyhow::Result;
use std::collections::HashSet;
use crate::collector::TestItem;
use crate::db::Database;

pub struct ImpactAnalyzer<'a> {
    db: &'a Database,
    changed_files: Vec<String>,
}

impl<'a> ImpactAnalyzer<'a> {
    pub fn new(db: &'a Database, changed_files: Vec<String>) -> Self {
        ImpactAnalyzer { db, changed_files }
    }

    /// Given a list of test items, return which ones need to run
    pub fn filter_affected(&self, tests: &[TestItem]) -> Result<(Vec<TestItem>, Vec<TestItem>)> {
        let changed_set: HashSet<&String> = self.changed_files.iter().collect();
        let mut to_run = Vec::new();
        let mut skipped = Vec::new();

        for test in tests {
            if self.should_run(test, &changed_set)? {
                to_run.push(test.clone());
            } else {
                skipped.push(test.clone());
            }
        }

        Ok((to_run, skipped))
    }

    fn should_run(&self, test: &TestItem, changed: &HashSet<&String>) -> Result<bool> {
        // Always run if the test's own file changed
        if changed.contains(&test.file_path) {
            return Ok(true);
        }

        // Check if test has no recorded deps (never run before)
        let deps = self.db.get_test_deps(&test.test_id)?;
        if deps.is_empty() {
            return Ok(true);
        }

        // Run if any dependency file changed
        for dep in &deps {
            if changed.contains(dep) {
                return Ok(true);
            }
        }

        // Check if the test previously failed — always re-run failures
        if let Some(status) = self.db.get_last_result(&test.test_id)? {
            if status == "failed" || status == "error" {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
