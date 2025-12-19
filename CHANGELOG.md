# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/4thel00z/bath/compare/v0.3.0...v0.3.1) - 2025-12-19

### Fixed

- *(tui)* preserve env var type selection in editor
- *(tui)* prevent deleting the last profile
- *(db)* rename profiles without duplicating rows
- *(export)* shell-quote values and terminate statements
- *(config)* correct separators for include path vars

### Other

- automate releases with release-plz
- *(README)* small update to the title
- satisfy clippy items-after-test-module
- fix clippy warnings
- refresh README; add CI and pre-commit
