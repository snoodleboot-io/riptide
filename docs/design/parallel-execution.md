# Parallel Execution

## Overview

riptide runs tests concurrently using [Rayon](https://github.com/rayon-rs/rayon), Rust's data parallelism library. Each test executes in its own OS subprocess, fully isolated from other tests.

## Worker Pool

Rayon manages a thread pool sized to `available_parallelism()` by default — the number of logical CPU cores. Each thread picks a test from the queue, spawns a subprocess, waits for it to complete, and picks the next test.

```
Thread 1: [test_login] [test_search] [test_export] ...
Thread 2: [test_logout] [test_filter] [test_import] ...
Thread 3: [test_signup] [test_profile] ...
Thread 4: [test_reset] [test_verify] ...
```

Override with `-n`:

```bash
riptide tests/ -n 16   # 16 concurrent tests
riptide tests/ -n 1    # sequential (useful for debugging)
```

## Process Isolation

Each test runs as:

```bash
python -m pytest path/to/test_file.py::test_function -x --tb=short -q --no-header
```

This means:

- **No shared state** between tests — fresh Python interpreter per test
- **Full pytest compatibility** — fixtures, plugins, and conftest.py all work
- **Predictable** — tests cannot interfere with each other's imports or globals

## Coverage Isolation

When `--coverage` is enabled, each test writes to a unique data file:

```
.riptide-coverage/
  .coverage.tests_test_auth_py__test_login
  .coverage.tests_test_auth_py__test_logout
  .coverage.tests_test_models_py__test_create_user
  ...
```

After all tests complete, `coverage combine --keep` merges them into a unified report. This avoids write conflicts when tests run concurrently.

## Output Ordering

Because tests run in parallel, output is printed as each test completes — not in source order. The `[N/total]` counter reflects completion order, not discovery order.

```
  ✓ [3/47] tests/test_utils.py::test_format_date 89ms    ← fast test finishes first
  ✓ [1/47] tests/test_auth.py::test_login 312ms
  ✓ [2/47] tests/test_auth.py::test_logout 289ms
```

## Failure Behaviour

By default, riptide continues running all tests even after a failure — unlike pytest's `-x` mode. The failing test's output is collected and printed in the summary.

This is intentional for parallel runs: stopping the pool when one test fails wastes the work already in flight on other threads.

## Performance Characteristics

| Scenario | Expected Speedup |
|---|---|
| 100% I/O-bound tests (DB, network) | Near-linear with core count |
| 100% CPU-bound tests | Near-linear with core count |
| Mixed workloads | Significant; depends on distribution |
| Very fast tests (<50ms each) | Limited by subprocess startup (~250ms) |

For very fast test suites, the subprocess startup cost dominates. In these cases, the impact analysis savings (skipping tests entirely) matter more than parallelism.
