# Coverage Engine

## Overview

riptide integrates with Python's `coverage.py` library for test coverage measurement. Coverage serves two purposes in riptide:

1. **Reporting** — show line coverage percentages per file after a run
2. **Dependency mapping** — build the graph of which tests import which source files (powers impact analysis)

## Per-Test Coverage

Unlike running `pytest --cov` which measures aggregate coverage, riptide measures coverage **per test**. This is what enables impact analysis to know exactly which tests depend on which files.

Each test runs as:

```bash
python -m coverage run \
  --data-file=.riptide-coverage/.coverage.<unique-test-id> \
  --source=. \
  --branch \
  -m pytest <nodeid> -x --tb=short -q
```

The `--branch` flag enables branch coverage (not just line coverage), giving more accurate dependency information.

## Merging

After all tests complete, riptide merges the individual `.coverage.*` files:

```bash
python -m coverage combine --keep .riptide-coverage/
python -m coverage json -o .riptide-coverage/combined.json
```

The JSON report is parsed to produce the terminal coverage table.

## Coverage Report Format

```
  Coverage
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  src/auth.py      [██████████] 100%  42/42
  src/models.py    [████████░░]  83%  25/30
  src/utils.py     [██████░░░░]  61%  11/18

  Overall: 87.4%
```

Progress bars are colour-coded:

- 🟢 Green: ≥80%
- 🟡 Yellow: 60–79%
- 🔴 Red: <60%

## Dependency Extraction

After each per-test coverage run, riptide extracts the list of files that were executed:

```json
{
  "files": {
    "src/auth.py": { "executed_lines": [...], "missing_lines": [...] },
    "src/models.py": { ... }
  }
}
```

This list is stored in SQLite:

```sql
INSERT INTO test_file_deps (test_id, dep_path) VALUES ('tests/test_auth.py::test_login', 'src/auth.py');
INSERT INTO test_file_deps (test_id, dep_path) VALUES ('tests/test_auth.py::test_login', 'src/models.py');
```

On subsequent runs without `--coverage`, this stored graph is used for impact analysis without re-running coverage instrumentation.

## When to Use `--coverage`

| Scenario | Recommendation |
|---|---|
| First run on a project | Always use `--coverage` to build dep graph |
| Regular development | Can omit `--coverage` — dep graph persists |
| After adding new source files | Use `--coverage` or `--all --coverage` to rebuild graph |
| CI runs | Recommended — keeps graph fresh and generates reports |

## Performance Impact

Coverage instrumentation adds ~2x overhead per test (roughly 300ms → 600ms for a typical test). This overhead is acceptable because:

- Impact analysis savings compound across runs
- Once the dep graph is built, future runs skip `--coverage` entirely
- Coverage runs in parallel too — the 2x cost is divided across all cores
