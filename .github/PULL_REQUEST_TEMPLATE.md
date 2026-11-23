<!--
  Title: Conventional Commits, e.g. `feat: add a --json flag to cell check`.
  Target branch: main.
-->

## What and why

<!-- One or two sentences. -->

Refs #

## How it was tested

<!-- The commands you ran and what they printed. -->

## Checklist

- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass.
- [ ] Tests cover the change, or I explained why none apply.
- [ ] If the CLI changed: the skill and the README's `cell help` block are updated.
- [ ] If this touches [SPEC.md](../blob/main/SPEC.md): it follows an issue the owner approved.
- [ ] `CHANGELOG.md` has an entry under Unreleased.
- [ ] Commits are signed off (`git commit -s`).
