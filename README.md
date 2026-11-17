# .cell

A cell is one record that an entity owns: a person, a household or a business. It is JSON-LD
with Markdown inside, the same in a `.cell` file or a database row, with its facts and its words
kept from drifting apart.

**Status: proposed, version 0.1.0 draft.** Nothing here is final until the owner approves the
scope. The spec is in [SPEC.md](SPEC.md).

## Why it lives with SiafuDB

The .cell maps directly to [SiafuDB](https://github.com/siafuDB/siafudb). A cell is the record;
SiafuDB is where it lives and how it moves:

- On the device and at the edge, a cell is a SiafuDB node keyed by its `@id`.
- Links between cells are SiafuDB edges.
- The [SiafuDB Graph Sync Protocol](https://github.com/siafuDB/siafudb-gsp) carries cells from
  the device to the person's isolated cloud space and on to their pod in the Nyuchi Honeycomb,
  without reshaping them.

A change to the .cell spec is checked against SiafuDB, and the other way round.

## Try the prototype

The prototype is a Rust crate, `siafudb-cell`, with a library and a `cell` command:

```sh
cargo run -- check FILE.cell    # validate one cell; exits 1 if it has errors
cargo run -- render FILE.cell   # print its text with facts filled in
cargo test                      # stress tests, RFC 8785 vectors, the CLI and the skill
CELL_PG="psql -h HOST -U postgres" cargo test -- --ignored   # adds the Postgres round trip
```

The validator is a prototype for stress-testing the draft, not the reference implementation.
The library holds the logic behind the command: `parse` (I-JSON, refusing duplicate
members), `canonicalize` (RFC 8785), `hash`, `resolve`, `texts`, `render`, `validate`, `seal`
and `verify_proof` (an Ed25519 proof in the shape of VC Data Integrity).

## CLI

`cargo build --release` builds the `cell` command at `target/release/cell` (or run it as
`cargo run -- COMMAND`). It uses the standard library only for its argument handling.
`cell help` prints:

```text
cell 0.0.1: validate, render, hash, seal and verify .cell records
Implements the .cell spec 0.1.0 draft, proposed.

Usage: cell COMMAND [ARGS]
       cell help COMMAND

Commands:
  check      Validate a cell, and its version chain with --previous
  render     Print a cell's text with every {{field}} filled in
  hash       Print the content hash and say whether it matches cell:hash
  canonical  Print the cell in RFC 8785 (JCS) canonical form
  keygen     Make an Ed25519 key pair: NAME.key (secret) and NAME.pub
  seal       Set cell:hash and sign it; prints the sealed cell
  verify     Check a cell's proof against a public key
  help       Show this list, or the details of one command
  version    Print the crate version and the spec version it implements

FILE may be - to read the cell from standard input.

Exit codes:
  0  ok
  1  the cell is invalid, or a check, a read or a write failed
  2  usage error: an unknown command or flag, or a missing argument

Examples:
  cell check asthma.cell
  cell check v2.cell --previous v1.cell
  cell render asthma.cell --lang sn
  cell keygen --out alice
  cell seal draft.cell --key alice.key > asthma.cell
  cell verify asthma.cell --pub alice.pub
  cat asthma.cell | cell hash -

Run `cell help COMMAND` for the details of one command.
```

`cell help COMMAND` or `cell COMMAND --help` gives the details of one command.

## Install the skill

This repository is also a Claude Code plugin marketplace. The `cell` plugin carries one skill,
for writing, validating, rendering, sealing and verifying cells and for questions about the
format and SiafuDB. It is Markdown only: no scripts and no hooks.

```text
/plugin marketplace add siafuDB/cell
/plugin install cell@siafudb-cell
```

The skill is `plugin/skills/cell/SKILL.md`, served from this repository through the plugin
marketplace; agents working here install it the same way (`.claude/` stays out of the
repository, by the org rule). The tests check that every command `cell help` lists appears in the skill and that the plugin
version is the crate version.

## Licence

The spec text is CC BY 4.0; any schema and JSON-LD context is CC0 1.0; the code is Apache 2.0
([LICENSE](LICENSE)). Stewarded by the Bundu Foundation.
