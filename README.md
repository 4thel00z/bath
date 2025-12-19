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

<p align="center">
  <img src="assets/bath.gif" alt="bath demo" />
</p>

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

- **TUI navigation (k9s-style)**

- **Views**: single active view with an always-visible bottom **Details** pane.
- **Global keys**
  - **`:`**: command palette (jump views / run commands)
  - **`/`**: filter current view (live while typing, `Esc` cancels/clears)
  - **`j`/`k`** or **Arrow keys**: move selection
  - **`g`/`End`**: jump to bottom
  - **`G`/`Home`**: jump to top
  - **`q`**: quit

- **Common `:` commands**
  - **`:profiles` `:vars` `:parts` `:items` `:defs` `:preview` `:export` `:help`**
  - **`:use <profile>`**
  - **`:themes`** (list available theme presets)
  - **`:theme <name>`** (switch theme; also persists to config)
  - **`:new-var`** (create a custom env var definition)
  - **`:new-item`** (create an item)
  - **`:quit`**

- **Theming**
  - Config file: **`~/.config/bath/config.toml`** (or `$XDG_CONFIG_HOME/bath/config.toml`)
  - Example:

```toml
[theme]
preset = "dracula"

# Optional overrides (accepts oklch(...) like DaisyUI, or #RRGGBB)
primary = "oklch(75% 0.18 346)"
base_100 = "#0b0f19"
```

- **Asciinema demo**

The repository includes a full recording that demonstrates:

- switching views via `:`
- filtering via `/`
- creating a profile, custom var definition, and items
- picking/dropping items and editing parts
- preview/export views
- switching themes

```bash
asciinema play assets/bath-demo.cast
```

Optionally upload it to asciinema.org to share a link:

```bash
asciinema upload assets/bath-demo.cast
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

- **Export help**:

```bash
bath export --help
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

### Releasing (crates.io)

Releases are automated with `release-plz` (it opens a release PR and publishes to crates.io after merge).

- **Required GitHub setting**: in `Settings → Actions → General`, set workflow permissions to allow GitHub Actions to create and approve pull requests. See the official quickstart: https://release-plz.dev/docs/github/quickstart
- **Required secret**: `CARGO_REGISTRY_TOKEN` (crates.io token with scopes `publish-new` and `publish-update`), used by `.github/workflows/release-plz.yml`.

## License

GPL-3.0. See `LICENSE`.

