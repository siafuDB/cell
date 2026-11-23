# Contributing to .cell

Thank you for helping shape the .cell. The spec is a draft, so a good argument or a cell that
breaks the validator is worth as much as code.

## The fastest useful contribution: break it

Write a cell that the validator accepts but should refuse, or refuses but should accept. Open
an issue with the cell and what you expected. If you can, add it as a test in
[`tests/stress.rs`](tests/stress.rs). Every stress case so far has turned into a decision the
spec has to make.

## Set up

You need Rust 1.85 or later (the crate uses the 2024 edition). Nothing else.

```sh
git clone https://github.com/siafuDB/cell
cd cell
cargo test
cargo run -- help
```

## Before you open a pull request

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass. CI runs the same commands, and nothing merges unless every check is green.

## Rules

- **Open an issue first** for anything beyond a typo, and reference it in the pull request
  (`Refs #12` or `Closes #12`).
- **Target `main`.** The repo is trunk-based: branch from `main` and open a pull request back into it. There is no staging branch.
- **Rust only** in code, tests and CI.
- **Tests with every change.** A new behaviour needs a test; a bug fix needs a test that fails
  without the fix.
- **Keep the CLI, the skill and the README in step.** If you add or change a command, update
  [`plugin/skills/cell/SKILL.md`](plugin/skills/cell/SKILL.md) and paste the new `cell help`
  output into the README. `tests/skill.rs` checks both.
- **Spec changes are proposals.** Change [SPEC.md](SPEC.md) only through an issue; the owner
  approves the scope of the draft.
- **Conventional Commits** for pull request titles: `feat:`, `fix:`, `docs:`, `test:`, `ci:`,
  `chore:`.
- **Sign off your commits** (`git commit -s`), certifying the
  [Developer Certificate of Origin](https://developercertificate.org/).

## Licence of contributions

Code you contribute is licensed under Apache 2.0. Text you contribute to the spec is licensed
under CC BY 4.0, and any schema or JSON-LD context under CC0 1.0.

## Conduct

Everyone here follows the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues go through
[SECURITY.md](SECURITY.md), never a public issue.
