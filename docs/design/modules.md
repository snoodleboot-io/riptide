# Module Design

## `main.rs` — Entry Point & Orchestration

The top-level binary entry point. Owns the CLI definition (via `clap`) and orchestrates the full run pipeline. All other modules are called from here in sequence.

**Key responsibilities:**
- Parse CLI args and subcommands (`run`, `collect`, `clear`, `coverage`)
- Resolve test paths from args or defaults
- Call collector → hasher → impact → runner → reporter in order
- Propagate exit codes (non-zero on test failure)

**Design principle:** `main.rs` is intentionally thin — it holds the glue, not the logic. Each step is delegated to its module.

---

## `collector.rs` — Test Discovery

Walks the file tree and extracts test function names from Python files using regex scanning — no Python interpreter required.

**Algorithm:**
1. Walk directories matching the file pattern (e.g. `test_*.py`)
2. For each file, scan lines for:
   - `class Test*:` → enter class context
   - `def test_*(...):` → emit a `TestItem`
3. Build pytest-compatible node IDs: `path::Class::function` or `path::function`

**Key type:**
```rust
pub struct TestItem {
    pub test_id: String,       // full pytest node ID
    pub file_path: String,     // source file
    pub function_name: String,
    pub class_name: Option<String>,
}
```

**Limitations:** Regex-based — cannot handle dynamically generated test functions or unusual indentation. In practice, covers 99% of real-world pytest code.

---

## `hasher.rs` — File Fingerprinting

Computes SHA-256 hashes of Python source files and compares against stored values to detect changes.

**Key functions:**
- `hash_file(path)` → SHA-256 hex string of file contents
- `hash_all_python_files(root)` → `HashMap<path, hash>` for entire tree
- `find_changed_files(current, db)` → files whose hash differs from DB
- `save_hashes(hashes, db)` → persist new hashes after a run

**Design:** Excludes `.git/`, `__pycache__/`, `.venv/`, `venv/` automatically during traversal.

---

## `db.rs` — Persistence Layer

Wraps a `rusqlite::Connection` and provides typed read/write access to the state database. All SQL is inline — no ORM.

**Key methods:**
- `save_file_hash(path, hash)` / `get_file_hash(path)`
- `save_test_result(result)` / `get_last_result(test_id)`
- `save_test_deps(test_id, deps)` / `get_test_deps(test_id)`
- `save_coverage(run_id, coverage_map)`

**Design:** Uses `INSERT OR REPLACE` for all writes — idempotent and safe to re-run.

---

## `impact.rs` — Affected Test Selection

Given a set of changed files and the stored dep graph, determines which tests need to re-run.

**Key type:**
```rust
pub struct ImpactAnalyzer<'a> {
    db: &'a Database,
    changed_files: Vec<String>,
}
```

**Decision logic (in order):**
1. Test file itself changed → run
2. Any dep file changed → run
3. No deps stored (first run) → run
4. Last result was failed/error → run
5. Otherwise → skip

Returns `(to_run: Vec<TestItem>, skipped: Vec<TestItem>)`.

---

## `runner.rs` — Parallel Execution

Executes tests using Rayon's parallel iterator. Each test spawns a subprocess and waits for completion.

**Key type:**
```rust
pub struct Runner {
    pub workers: usize,
    pub python_bin: String,
    pub with_coverage: bool,
    pub coverage_dir: PathBuf,
}
```

**Flow per test:**
1. Build `Command`: `python -m pytest <nodeid>` (or `coverage run ... -m pytest ...`)
2. `.output()` — blocks until subprocess exits
3. Parse stdout/stderr for pass/fail status
4. If coverage: extract file list from coverage JSON
5. Return `TestResult`

**Coverage output:** Each test writes `.coverage.<id>` to `.riptide-coverage/`. After all tests, `merge_coverage()` runs `coverage combine` and parses the JSON report.

---

## `reporter.rs` — Terminal Output

Handles all user-visible output. Deliberately separated from runner logic.

**Functions:**
- `print_header(total, skipped, workers, coverage)` — pre-run summary line
- `print_progress(n, total, result)` — per-test line printed as tests complete
- `print_summary(results, skipped_tests, elapsed, coverage)` — final table
- `print_coverage_report(coverage_map)` — coloured bar chart

**Design:** Uses the `colored` crate for ANSI colour. All colour output respects `NO_COLOR` env var via the crate's defaults.
