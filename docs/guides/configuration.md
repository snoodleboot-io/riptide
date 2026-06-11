# Configuration

riptide is configured via command-line flags and optionally a `pyproject.toml` section.

## CLI Flags

| Flag | Default | Description |
|---|---|---|
| `[PATHS]` | `tests/ test/` | Test directories or files to scan |
| `-n, --workers` | `0` (CPU count) | Parallel worker threads |
| `--python` | `python3` | Python binary to use |
| `-c, --coverage` | off | Enable per-test coverage |
| `--all` | off | Ignore impact analysis, run everything |
| `--pattern` | `test_.*\.py\|.*_test\.py` | Regex for test file discovery |
| `--db` | `.riptide.db` | Path to SQLite state database |

## pyproject.toml

Add a `[tool.riptide]` section to configure defaults:

```toml
[tool.riptide]
workers = 8
python = ".venv/bin/python"
coverage = true
pattern = "test_.*\\.py"
db = ".riptide.db"
paths = ["tests/", "integration/"]
```

!!! note
    CLI flags always take precedence over `pyproject.toml` settings.

## Environment Variables

| Variable | Description |
|---|---|
| `RIPTIDE_WORKERS` | Override worker count |
| `RIPTIDE_DB` | Override DB path |
| `RIPTIDE_PYTHON` | Override Python binary |

## Test Discovery

riptide finds tests by:

1. Walking directories matching `--pattern` for file names
2. Scanning each file for `def test_*` functions (top-level and inside `class Test*`)
3. Building node IDs in pytest format: `path/to/test_file.py::TestClass::test_name`

!!! warning "Fixtures and conftest.py"
    riptide delegates execution to `pytest` as a subprocess, so all fixtures, `conftest.py`, parametrize, and marks work as normal. riptide controls *which* tests run and *how many at once* — not how they execute.

## Recommended .gitignore

```gitignore
.riptide.db
.riptide-coverage/
```

The state database is machine-local. Each developer and each CI runner maintains their own.
