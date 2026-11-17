---
name: cell
description: Use this skill when writing, validating, rendering, sealing or verifying .cell records, or when answering questions about the .cell format and how it maps to SiafuDB. Covers what a cell is, the five rules that keep facts and text from diverging, {{field}} references, NFC, the content hash and the proof, the version chain, one file in two homes (a .cell file and a database row), the SiafuDB mapping, and every `cell` command with an example. The spec is a proposed 0.1.0 draft; nothing in it is final.
---

# .cell

**Status: proposed, version 0.1.0 draft.** Nothing is final until the owner approves it. The
`cell:` names, the context URL `https://standards.bundu.org/cell/v0.1` (not published yet)
and the media type `application/vnd.bundu.cell+json` are all draft. The source of truth is
SPEC.md in [siafuDB/cell](https://github.com/siafuDB/cell); when this skill
and the spec disagree, the spec wins. The `cell` command is a prototype validator for
stress-testing the draft, not the reference implementation.

## What a cell is

One record that a person owns, such as a health record, a contact, a receipt or a note. It is
a JSON-LD object with schema.org meaning (`@context`, `@type`), a stable identity (`@id`),
typed fields that hold the facts, and a `text` field of CommonMark prose. A pod is the set of
cells that belong to one person.

| Field                           | Purpose                                                               |
| ------------------------------- | --------------------------------------------------------------------- |
| `@context`, `@type`             | schema.org meaning, so any system can read the cell                   |
| `@id`                           | The stable identity, and the database key                             |
| Typed fields                    | The facts, each stated once                                           |
| `text`                          | CommonMark prose; it may name a fact only as `{{field}}`              |
| `cell:version`, `cell:previous` | The version chain: each version names the hash of the one before      |
| `cell:hash`                     | SHA-256 of the cell in RFC 8785 form, without `cell:hash` and `proof` |
| `proof`                         | A W3C Data Integrity signature by the owner's key                     |

The prototype validator requires `@context`, `@type`, `@id`, `cell:version` and `cell:hash`.

## The five rules: why facts and text can't diverge

1. Each fact is stated once, in a typed field. The `text` restates none of them.
2. The `text` names a fact only as `{{field}}`, and every reference resolves to a field.
3. Readers render each `{{field}}` from the field, so changing a fact changes every sentence
   that uses it.
4. `cell:hash` covers the fields and the text together; editing either without re-hashing
   makes the cell invalid.
5. A new version names the hash of the one before. History can grow; rewriting it breaks
   the chain.

Rule 1 is a lint, not a guarantee: `cell check` warns when a fact's value appears written out
in the text, but a paraphrase ("March 2024" for `2024-03-11`) is not detectable.

## Writing `{{field}}` references

- Write `{{dateCreated}}`, never the date itself. Nested fields use a dotted path,
  `{{possibleTreatment.name}}`; a segment of digits indexes an array, `{{author.0.name}}`.
- Every reference must name a field, or `cell check` refuses the cell.
- Rendering is one pass: a field whose value looks like `{{...}}` is not expanded again.
- `text` may be one string, or a JSON-LD language map keyed by BCP 47 tags
  (`{"en": "...", "sn": "..."}`); references are checked in every language.
- Proposed, not decided: references move to JSON Pointer (RFC 6901),
  `{{/possibleTreatment/name}}`.

## NFC

The draft proposes that all text is Unicode NFC (UAX #15), because the same name in two
normal forms gives two hashes. `cell check` already refuses a `text` that is not NFC.
Normalise to NFC before you hash or seal.

## Hash and proof

- `cell:hash` is `sha256-` and the unpadded base64url SHA-256 of the cell in RFC 8785 (JCS)
  canonical form, with `cell:hash` and `proof` left out. Key order and whitespace do not
  change it.
- The draft proposes I-JSON (RFC 7493): no duplicate member names (the `cell` parser refuses
  them), and integers beyond 2^53, such as ID numbers, as strings, since JCS reads every
  number as a double and rounds them.
- `proof` is an Ed25519 signature over the canonical cell without its proof, hash included,
  in the shape of a VC Data Integrity `eddsa-jcs-2022` proof. The prototype follows that
  shape, not that algorithm. Any change after sealing makes the proof fail.
- Never edit a sealed cell by hand and keep its hash: re-seal it.

## The version chain

A new version keeps the `@id`, sets `cell:version` to the old version plus one, and sets
`cell:previous` to the old version's `cell:hash`; then it is sealed again. `cell check NEW
--previous OLD` checks all three. `--previous` reads only the old version's `@id`,
`cell:version` and `cell:hash`, so the chain stays checkable after old content is deleted.
Open: offline edits on two devices fork the chain; the proposal is that `cell:previous`
becomes a list and a merge version names both parents.

## One file, two homes

| Home     | Stored as                                                                        | Key                    |
| -------- | -------------------------------------------------------------------------------- | ---------------------- |
| File     | `name.cell`, UTF-8 JSON, media type `application/vnd.bundu.cell+json` (proposed) | The path; `@id` inside |
| Database | One row: an `@id` column and the cell in a JSON or JSONB column                  | `@id`                  |

"The same in both homes" means the same canonical form and hash, not the same bytes: a
Postgres `jsonb` column rewrites the bytes and keeps the hash. Exporting writes the JSON column
to a file unchanged; importing stores the file under its `@id`.

## SiafuDB

A cell is the record; SiafuDB is where it lives and how it moves.

- A cell is a SiafuDB node keyed by its `@id`, holding the cell unchanged.
- Links between cells (the cell's JSON-LD references) are SiafuDB edges.
- SiafuDB on the device is the cell's first home: offline first.
- The SiafuDB Graph Sync Protocol carries cells device → cloud → pod (the person's isolated
  cloud space, then their pod in the Nyuchi Honeycomb) without reshaping them, and must carry
  each version unchanged, by its hash.
- A change to the .cell spec is checked against SiafuDB, and the other way round. Both are
  proposed or pre-release; nothing here commits SiafuDB to a release.

## The `cell` command

Build it from the repository with `cargo build --release` (or `cargo run -- COMMAND`). `FILE`
may be `-` to read standard input. Exit codes: 0 ok, 1 invalid or failed, 2 usage error.

| Command          | Example                                              | What it does                                                                                              |
| ---------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `cell check`     | `cell check v2.cell --previous v1.cell`              | Validate; `--previous` adds the version chain. Prints `warning:` and `error:` lines; exits 1 on any error |
| `cell render`    | `cell render asthma.cell --lang sn`                  | Print the text with every `{{field}}` filled in                                                           |
| `cell hash`      | `cell hash asthma.cell`                              | Print the computed hash and whether it matches `cell:hash`; exits 1 on a mismatch                         |
| `cell canonical` | `cell canonical asthma.cell`                         | Print the RFC 8785 (JCS) form                                                                             |
| `cell keygen`    | `cell keygen --out alice`                            | Write `alice.key` (secret, mode 0600 on Unix, never printed) and `alice.pub`; refuses to overwrite        |
| `cell seal`      | `cell seal draft.cell --key alice.key > asthma.cell` | Set `cell:hash`, sign, print the sealed cell; refuses a cell with errors                                  |
| `cell verify`    | `cell verify asthma.cell --pub alice.pub`            | Check the proof; exits 0 if valid, 1 if not                                                               |
| `cell help`      | `cell help seal`                                     | List every command, or the details of one (`cell seal --help` too)                                        |
| `cell version`   | `cell version`                                       | The crate version and the spec version it implements                                                      |

A typical round: write the draft without `cell:hash` or `proof`, run `cell seal` to hash and
sign it, then `cell check` and `cell verify`. To make a new version, copy the sealed cell,
change the facts, bump `cell:version`, set `cell:previous` to the old `cell:hash`, seal, and
`cell check NEW --previous OLD`.

## A minimal valid cell

This passes `cell check` with no warnings. It has no proof; `cell seal` adds one.

```json
{
  "@context": "https://schema.org",
  "@type": "MedicalCondition",
  "@id": "urn:cell:01JD3Q7Z8K2V5M",
  "name": "Asthma",
  "dateCreated": "2024-03-11",
  "text": "Diagnosed on {{dateCreated}}.",
  "cell:version": 1,
  "cell:hash": "sha256-AoJcwI8-7BtItG7iIeiAzrEFwMxAQwu8gv7Bx0RL0mk"
}
```

## Still open

Size and bundling (is a scan one cell, or a cell that points at a file?), encryption at rest
(JWE around the whole cell or only sensitive fields?), when to register the media type, the
merge rule for forked chains, and JSON Pointer references. Don't present any of these as
decided.
