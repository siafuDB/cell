# Changelog

All notable changes to this repository are documented here. This file follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/). The spec has its own version, in [SPEC.md](SPEC.md).

## [Unreleased]

### Added

- The README, rewritten for developers: the problem, a 30-second CLI walkthrough, the Rust API,
  the test suite and the open questions, every output taken from a real run.
- `examples/quickstart.rs`, the README's Rust example, built and run by CI.
- Repository docs ([#3](https://github.com/siafuDB/cell/issues/3)): `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`,
  `CODEOWNERS`, a pull request template, issue templates and Dependabot for Cargo and Actions.

## [0.0.1] - 2026-11-17

### Added

- The .cell spec, version 0.1.0 draft, moved here from `bundu-labs/bundu-standards`.
- `siafudb-cell`, a Rust crate: the `siafudb_cell` library (`parse`, `canonicalize`, `hash`,
  `resolve`, `texts`, `render`, `validate`, `seal`, `verify_proof`, `keys`) and the `cell`
  command with `help` for every command.
- Stress tests, RFC 8785 test vectors, CLI tests and an optional Postgres round trip.
- The `cell` skill, served from this repository as a Claude Code plugin marketplace.
