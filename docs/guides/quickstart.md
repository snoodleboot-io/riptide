# Quick Start

## Your First Run

Point riptide at your test directory. It will discover all `test_*.py` and `*_test.py` files automatically:

```bash
riptide tests/
```

On the first run, riptide:

1. Collects all tests via fast regex-based scanning
2. Hashes every `.py` file in the project
3. Runs all tests in parallel
4. Stores results and file hashes in `.riptide.db`

```
  ✓ collected 47 tests
  ⚡ no previous state — running all tests

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    riptide ⚡ Rust-powered test engine
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    tests: 47   skipped (unchanged): 0   workers: 8   coverage: off

  ✓ [1/47] tests/test_auth.py::test_login 312ms
  ✓ [2/47] tests/test_auth.py::test_logout 289ms
  ...
```

## Second Run (Impact Analysis)

Run again without changing anything:

```bash
riptide tests/
```

```
  ✓ collected 47 tests
  ⚡ no files changed
  ⚡ All tests skipped — no changes detected!
```

**Zero tests run. Instant feedback.**

## After Changing a File

Edit a source file, then run again:

```bash
riptide tests/
```

```
  ✓ collected 47 tests
  ⚡ 1 file(s) changed:
    src/auth.py

  tests: 8   skipped (unchanged): 39   workers: 8
```

Only the 8 tests that import `auth.py` are re-run.

## Enable Coverage

```bash
riptide tests/ --coverage
```

After the run, a coverage report is printed and the dependency graph is stored — this makes future impact analysis smarter.

```
  Coverage
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  src/auth.py      [██████████] 100%  42/42
  src/models.py    [████████░░]  83%  25/30
  src/utils.py     [██████░░░░]  61%  11/18

  Overall: 87.4%
```

## Common Commands

```bash
# Run with 8 parallel workers
riptide tests/ -n 8

# Force run all tests regardless of changes
riptide tests/ --all

# Collect and list all tests without running
riptide collect tests/

# Reset state (next run will re-run everything)
riptide clear

# Use a specific Python binary
riptide tests/ --python .venv/bin/python
```

## Next Steps

- [Configuration](configuration.md) — customize patterns, workers, DB path
- [How impact analysis works](../design/impact-analysis.md)
- [CI/CD setup](../guides/releases.md)
