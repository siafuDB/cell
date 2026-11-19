# Changelog

All notable changes to this repository are documented here. This file follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/). The spec has its own version, in [SPEC.md](SPEC.md).

## [Unreleased]

## [0.1.4] - 2026-11-19

### Changed

- `main` no longer carries GitHub's starter `rust.yml` either: it had been committed straight
  to `main`, so removing it from `staging` (0.1.3) did not reach the release. Hotfix
  [#24](https://github.com/siafuDB/cell/pull/24) removed it there
  ([#22](https://github.com/siafuDB/cell/issues/22)).

## [0.1.3] - 2026-11-17

### Changed

- Releases are tagged from the tags, not the version files ([#22](https://github.com/siafuDB/cell/issues/22)).
  `staging-version.yml` tags every merge into `staging` as the next patch, and
  `main-release.yml` tags every release to `main` as the next minor and publishes it. The
  version files hold the version of the last merge into `staging`, so the release after 0.1.3
  is v0.2.0. (v0.1.2 was tagged on a merge that left the files at 0.1.1.)
- Removed `.github/workflows/rust.yml`, GitHub's starter Rust workflow
  ([#22](https://github.com/siafuDB/cell/issues/22)). `ci.yml` already builds and tests the
  crate, with fmt and clippy, pinned actions and a `permissions:` block.

## [0.1.1] - 2026-11-17

### Added

- The README, rewritten for developers: the problem, a 30-second CLI walkthrough, the Rust API,
  the test suite and the open questions, every output taken from a real run.
- `examples/quickstart.rs`, the README's Rust example, built and run by CI.
- Repository docs ([#3](https://github.com/siafuDB/cell/issues/3)): `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`,
  `CODEOWNERS`, a pull request template, issue templates and Dependabot for Cargo and Actions
  (with a 7-day cooldown on newly published versions).
- `examples/profile/`: a person, their family and their organisation as three linked cells,
  built from the founder's real profile with every personal detail replaced by a sample.
  `tests/examples.rs` checks that each validates with no warnings and that every link resolves.

## [0.0.1] - 2026-11-17

### Added

- The .cell spec, version 0.1.0 draft, moved here from `bundu-labs/bundu-standards`.
- `siafudb-cell`, a Rust crate: the `siafudb_cell` library (`parse`, `canonicalize`, `hash`,
  `resolve`, `texts`, `render`, `validate`, `seal`, `verify_proof`, `keys`) and the `cell`
  command with `help` for every command.
- Stress tests, RFC 8785 test vectors, CLI tests and an optional Postgres round trip.
- The `cell` skill, served from this repository as a Claude Code plugin marketplace.
