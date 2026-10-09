---
name: TESTING.md
description: Verification standards and required checks; read before writing tests or claiming work is verified.
---

# Testing

Test strategy and verification gates. This root is the family index; focused detail lives under [testing/](testing/).

- [Testing overview](testing/overview.md) — CI jobs and exact commands, golden-fixture harness, opt-in real-session test, benches, perf scripts and thresholds, platform pitfalls, test examples.

## Quick reference

- Run locally what CI runs on every PR: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --locked --all-features`. Full list: [overview](testing/overview.md#ci-jobs-and-commands).
- Parser/provider changes: goldens under `assets/{claude,codex,omp}/`; regenerate only with `UPDATE_GOLDEN=1` and review the diff.
- Test CI runs on ubuntu, macos **and windows**: no hard-coded `/` path expectations.
- Related repo docs: [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md), [docs/DISCOVERY.md](../docs/DISCOVERY.md), [.issueops/issues/1/gates.md](issues/1/gates.md) (measured gates).

## Appending knowledge

- Append new dated records with MCP `project_docs_append` or `issueops project append`.
- Records are written as one file per record inside the module directory, so this index stays small.
- Revise this index only to add links to new curated modules; keep it within the manifest line budget.
