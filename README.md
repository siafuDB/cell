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

```sh
node tools/cell.mjs check FILE.cell    # validate one cell
node tools/cell.mjs render FILE.cell   # print its text with facts filled in
node --test tools/cell.test.mjs        # the stress tests
```

The validator is a prototype for stress-testing the draft, not the reference implementation.

## Licence

The spec text is CC BY 4.0; any schema and JSON-LD context is CC0 1.0; the code is Apache 2.0
([LICENSE](LICENSE)). Stewarded by the Bundu Foundation.
