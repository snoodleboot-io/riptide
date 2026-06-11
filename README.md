<div align="center">

<img src="docs/assets/logo.svg" width="64" height="64" alt="riptide logo">

# riptide ⚡

**Rust-powered Python test engine**  
Parallel execution · Impact analysis · Coverage · Zero config

[![CI](https://github.com/your-org/riptide/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/riptide/actions/workflows/ci.yml)
[![Release](https://github.com/your-org/riptide/actions/workflows/release.yml/badge.svg)](https://github.com/your-org/riptide/actions/workflows/release.yml)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache_2.0-C75B39.svg)](LICENSE)

</div>

---

## What is riptide?

riptide is a compiled Rust binary that orchestrates your Python test suite faster than pure-Python tools can. It runs tests in parallel and — crucially — **only re-runs tests affected by files you actually changed.**

```
$ riptide tests/

  ✓ collected 200 tests
  ⚡ 1 file changed: src/auth.py

  tests: 8   skipped (unchanged): 192   workers: 8   coverage: on

  ✓ [1/8] tests/test_auth.py::test_login              312ms
  ✓ [2/8] tests/test_auth.py::test_logout             289ms
  ✓ [3/8] tests/test_auth.py::test_session_expire     301ms
  ...

  ✓ passed: 8
  ⚡ skipped (unchanged): 192 (impact analysis)
  time: 0.71s
```

## Features

| | riptide | pytest | pytest-xdist | pytest-testmon |
|---|:---:|:---:|:---:|:---:|
| Parallel execution | ✅ Rust/Rayon | ❌ | ✅ Python | ❌ |
| Impact analysis | ✅ | ❌ | ❌ | ✅ Python |
| Coverage | ✅ | via plugin | via plugin | via plugin |
| Written in | 🦀 Rust | 🐍 Python | 🐍 Python | 🐍 Python |
| Subprocess overhead | ~250ms/test | shared | shared | shared |
| State persistence | SQLite | none | none | `.testmondata` |

## Install

```bash
# Linux x86_64
curl -sSfL https://github.com/your-org/riptide/releases/latest/download/riptide-linux-x86_64 \
  -o /usr/local/bin/riptide && chmod +x /usr/local/bin/riptide

# From source
cargo install riptide
```

## Quick Start

```bash
# First run — builds the dependency graph
riptide tests/ --all --coverage

# All subsequent runs — only changed tests
riptide tests/

# CI
riptide tests/ -n 8 --coverage --python .venv/bin/python
```

## How It Works

1. **Collect** — Scan `test_*.py` files with Rust regex (no Python startup)
2. **Hash** — SHA-256 fingerprint every `.py` file in the tree
3. **Diff** — Compare against hashes stored in `.riptide.db`
4. **Impact** — Map changed files to affected tests via stored coverage dep graph
5. **Run** — Rayon parallel pool; each test is an isolated `pytest` subprocess
6. **Persist** — Store new hashes, results, and coverage dep graph

## Add to .gitignore

```gitignore
.riptide.db
.riptide-coverage/
```

## Documentation

Full documentation at **[riptide-test.dev](https://riptide-test.dev)**:

- [Quick Start](https://riptide-test.dev/guides/quickstart/)
- [Architecture](https://riptide-test.dev/design/architecture/)
- [Impact Analysis Deep Dive](https://riptide-test.dev/design/impact-analysis/)
- [CLI Reference](https://riptide-test.dev/api/cli/)

## License

Apache 2.0 — see [LICENSE](LICENSE)
