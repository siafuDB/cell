# Security Policy

The .cell is a format for records people own: their health, their money, their contacts. A
weakness in how a cell is hashed, signed, parsed or verified matters. Thank you for looking.

## Reporting a vulnerability

**Do not open a public issue, pull request or discussion for a suspected vulnerability.**

Use either private channel:

1. **GitHub private vulnerability reporting:** the repository's **Security** tab, then
   **Report a vulnerability**.
2. **Email:** <security@nyuchi.com>. Say if you want to encrypt your report and we will arrange
   it.

Please include the version or commit, a cell or command that shows the problem, what you
expected, and what happened.

## What is in scope

- The `siafudb_cell` library and the `cell` command in this repository.
- The spec in [SPEC.md](SPEC.md), where the format itself allows forgery, tampering that
  validates, or two different cells with the same hash.

Weaknesses in dependencies (`ed25519-dalek`, `sha2`, `serde_json` and the rest) go to their
upstream projects. If one has a concrete effect on .cell, we want to hear about that effect.

## Status

The crate is a **prototype** and the spec is a **draft**. The `proof` follows the shape of a W3C
`eddsa-jcs-2022` proof, not yet its exact algorithm. Do not rely on either to protect real
personal data yet. Reports are still welcome: they shape the spec before it is adopted.

## Our commitments

- We acknowledge a report within **3 business days**.
- We give an initial assessment within **10 business days**.
- We coordinate disclosure with you and credit you unless you ask us not to. Our default target
  is a fix and an advisory within **90 days** of confirming the issue.
- We will not take legal action against research done in good faith under this policy.
