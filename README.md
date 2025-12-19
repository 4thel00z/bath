<h1 align="center">bath 🚿</h1>

<p align="center">
  <a href="https://github.com/4thel00z/bath/actions/workflows/ci.yml">
    <img alt="CI" src="https://github.com/4thel00z/bath/actions/workflows/ci.yml/badge.svg" />
  </a>
  <a href="https://crates.io/crates/bath">
    <img alt="Crates.io" src="https://img.shields.io/crates/v/bath.svg" />
  </a>
  <a href="https://github.com/4thel00z/bath/blob/master/LICENSE">
    <img alt="License" src="https://img.shields.io/github/license/4thel00z/bath" />
  </a>
</p>

`bath` is a terminal UI (TUI) tool for managing environment-variable profiles (e.g. `PATH`, compiler flags, and linker flags) backed by SQLite.

## Features

- **Profiles in SQLite**: store multiple named profiles and switch/export them consistently
- **Interactive TUI**: edit variables with live preview of the resulting export output
- **Export for shell eval**: print `export ...` statements for the selected profile
- **Modes**: prepend/append/replace behavior per variable

## Installation

```bash
cargo install bath
```

## Usage

- **TUI mode**:

```bash
bath
```

- **Export a profile**:

```bash
bath export my_profile
```

- **Eval in your shell**:

```bash
eval "$(bath export my_profile)"
```

- **Choose export mode** (`prepend` is default):

```bash
bath export my_profile --mode append
```

## Data storage

Bath stores profiles in a SQLite database at:

- `~/.bath.db`

## Development

### pre-commit hooks

This repository includes a `.pre-commit-config.yaml` to run basic checks locally (formatting, clippy, tests).

```bash
pipx install pre-commit
pre-commit install
pre-commit run -a
```

### CI

GitHub Actions runs the following on every push and pull request:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

## License

GPL-3.0. See `LICENSE`.

