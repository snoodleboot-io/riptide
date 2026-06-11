use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS file_hashes (
                path TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS test_results (
                test_id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                status TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                stdout TEXT,
                stderr TEXT,
                ran_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS test_file_deps (
                test_id TEXT NOT NULL,
                dep_path TEXT NOT NULL,
                PRIMARY KEY (test_id, dep_path)
            );

            CREATE TABLE IF NOT EXISTS coverage_data (
                run_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                lines_covered TEXT NOT NULL,
                lines_total INTEGER NOT NULL,
                ran_at TEXT NOT NULL,
                PRIMARY KEY (run_id, file_path)
            );
        ")?;
        Ok(())
    }

    /// Store file hashes after a run
    pub fn save_file_hash(&self, path: &str, hash: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO file_hashes (path, hash, updated_at) VALUES (?1, ?2, ?3)",
            params![path, hash, now],
        )?;
        Ok(())
    }

    /// Get stored hash for a file
    pub fn get_file_hash(&self, path: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT hash FROM file_hashes WHERE path = ?1",
            params![path],
            |row| row.get(0),
        );
        match result {
            Ok(h) => Ok(Some(h)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Record which source files a test depends on (from coverage data)
    pub fn save_test_deps(&self, test_id: &str, deps: &[String]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM test_file_deps WHERE test_id = ?1",
            params![test_id],
        )?;
        for dep in deps {
            self.conn.execute(
                "INSERT OR IGNORE INTO test_file_deps (test_id, dep_path) VALUES (?1, ?2)",
                params![test_id, dep],
            )?;
        }
        Ok(())
    }

    /// Get all deps for a test
    pub fn get_test_deps(&self, test_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT dep_path FROM test_file_deps WHERE test_id = ?1"
        )?;
        let deps = stmt.query_map(params![test_id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(deps)
    }

    /// Save test result
    pub fn save_test_result(&self, result: &crate::runner::TestResult) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO test_results 
             (test_id, file_path, status, duration_ms, stdout, stderr, ran_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                result.test_id,
                result.file_path,
                result.status.as_str(),
                result.duration_ms,
                result.stdout,
                result.stderr,
                now
            ],
        )?;
        Ok(())
    }

    /// Get last result for a test
    pub fn get_last_result(&self, test_id: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT status FROM test_results WHERE test_id = ?1",
            params![test_id],
            |row| row.get(0),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save coverage data for a run
    pub fn save_coverage(&self, run_id: &str, coverage: &HashMap<String, (Vec<u32>, u32)>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        for (file, (covered_lines, total)) in coverage {
            let lines_json = serde_json::to_string(covered_lines)?;
            self.conn.execute(
                "INSERT OR REPLACE INTO coverage_data 
                 (run_id, file_path, lines_covered, lines_total, ran_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![run_id, file, lines_json, total, now],
            )?;
        }
        Ok(())
    }

    /// Get all stored test IDs and their file paths
    pub fn get_all_tests(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT test_id, file_path FROM test_results"
        )?;
        let tests = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tests)
    }
}
