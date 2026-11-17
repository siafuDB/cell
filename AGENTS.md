# AGENTS.md

Rules for AI coding agents working in `siafuDB/cell`. The enterprise rules in
[`nyuchi/.github/AGENTS.md`](https://github.com/nyuchi/.github/blob/main/AGENTS.md) and the
org rules in [`siafuDB/.github/AGENTS.md`](https://github.com/siafuDB/.github/blob/main/AGENTS.md)
apply in full. This file adds what is specific to this repository and wins where they conflict.

## What this repository is

The .cell spec ([SPEC.md](SPEC.md), a proposed 0.1.0 draft) and its prototype validator: the
Rust crate `siafudb-cell`, with the library `siafudb_cell` and the `cell` command. The .cell is
a Bundu Labs project; it lives in the SiafuDB organisation because a cell is what SiafuDB
stores. A change to the spec is checked against SiafuDB, and the other way round.

## Rules

- **Rust only.** No Node, JavaScript or Python in the code, the tests or CI. If a tool has no
  Rust equivalent, use Deno or Vite+ (`vp`) and say why in the pull request.
- **No agent-local files.** No `CLAUDE.md` and no `.claude/` directory. The skill agents use is
  served from [`plugin/skills/cell/SKILL.md`](plugin/skills/cell/SKILL.md) through the plugin
  marketplace in `.claude-plugin/marketplace.json`.
- **The spec does not change in a code PR.** Spec changes need an issue and the owner's
  approval; the validator follows the spec, not the other way round.
- **The skill and the README move with the CLI.** `tests/skill.rs` fails if a command is
  missing from the skill, if the plugin version is not the crate version, or if the README does
  not show the current `cell help`. Update all three in the same pull request.
- **Every claim in the README is checked.** Counts, outputs and links come from a real run.
  `examples/quickstart.rs` is the README's Rust example; keep them identical.
- **Nothing merges while anything is red or orange.** Every check and status on the head commit
  must be green; the `All checks green` job enforces it.
- **Every pull request references an issue** and targets `staging`. Releases go from `staging`
  to `main` through a release pull request.

## Before you push

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --example quickstart
```
