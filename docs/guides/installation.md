# Installation

## Prerequisites

- Python 3.8+ with `pip`
- `pytest` and `coverage` Python packages
- Linux (x86_64 or ARM64) — macOS support coming in v2

## Binary (Recommended)

Download the pre-built binary from the [releases page](https://github.com/your-org/riptide/releases/latest):

```bash
# Linux x86_64
curl -sSfL https://github.com/your-org/riptide/releases/latest/download/riptide-linux-x86_64 \
  -o /usr/local/bin/riptide && chmod +x /usr/local/bin/riptide

# Linux ARM64
curl -sSfL https://github.com/your-org/riptide/releases/latest/download/riptide-linux-arm64 \
  -o /usr/local/bin/riptide && chmod +x /usr/local/bin/riptide

# Verify
riptide --version
```

## From Source (Cargo)

Requires [Rust toolchain](https://rustup.rs) 1.75+:

```bash
cargo install riptide
```

## Python Dependencies

riptide shells out to `pytest` and optionally `coverage`. Install them in your project environment:

```bash
pip install pytest coverage
# or
uv add --dev pytest coverage
```

## CI / GitHub Actions

The [official workflow](.github/workflows/ci.yml) handles installation automatically. For custom setups:

```yaml
- name: Install riptide
  run: |
    curl -sSfL https://github.com/your-org/riptide/releases/latest/download/riptide-linux-x86_64 \
      -o /usr/local/bin/riptide && chmod +x /usr/local/bin/riptide

- name: Run tests
  run: riptide tests/ --coverage -n 4
```

## Add to .gitignore

```gitignore
# riptide state — machine-local, do not commit
.riptide.db
.riptide-coverage/
```
