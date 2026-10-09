---
name: overview
description: Family module overview: implementation and interface conventions.
---

# Conventions — Overview

Canonical index: [CONVENTIONS.md](../CONVENTIONS.md)

Commit rules: [COMMIT_POLICY.md](../COMMIT_POLICY.md). Architecture and boundaries: [ARCHITECTURE.md](../ARCHITECTURE.md), [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md), [docs/DISCOVERY.md](../../docs/DISCOVERY.md).

Legend: **Enforced** = a CI job or config fails on it. **Observed** = visible in the code or history, nothing fails if broken.

## Toolchain gates (enforced, `.github/workflows/ci.yml`)

| Gate | Command / config | Notes |
| --- | --- | --- |
| Format | `cargo fmt --all --check` (stable) | No `rustfmt.toml` / `.rustfmt.toml`: rustfmt defaults. |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | Warnings fail. No `clippy.toml`. |
| Tests | `cargo test --locked --all-features` on ubuntu, macos, windows | `--locked`: `Cargo.lock` is committed and must be current. |
| Portable core | `cargo check --lib --locked --no-default-features` | Must build without tokio, crossterm or filesystem code. |
| Docs | `cargo doc --locked --all-features --no-deps` with `RUSTDOCFLAGS=-D warnings` | Broken intra-doc links fail CI. |
| Package | `cargo publish --dry-run --locked` | `include` in `Cargo.toml` is an allow-list (`src/**/*.rs`, README, LICENSE, NOTICE). |
| MSRV | `cargo check --lib --locked --all-features` on the toolchain read from `rust-version` | `rust-version = "1.88"`, the floor inherited from ratatui 0.30 (comment in `Cargo.toml`). |
| Typos | `crate-ci/typos@master`, config `typos.toml` | See below. |
| Commits | `crate-ci/committed@v1.1.11` | See [COMMIT_POLICY.md](../COMMIT_POLICY.md). |

- Edition 2024 (`Cargo.toml`). No `rust-toolchain` file: CI uses `dtolnay/rust-toolchain@stable`, except the MSRV job.
- Lint relaxations in source are local and rare: `#[allow(clippy::too_many_arguments)]` on three render helpers (`src/ui/views/detail.rs:314`, `src/ui/views/lanes.rs:386,535`) and `#[allow(deprecated)]` (`src/provider/claude/discovery.rs:29`). No crate-level `#![deny/forbid/warn]` attributes were found.
- The test job asserts nothing about clippy on its own; a prior fix `style(test): satisfy clippy needless_borrows_for_generic_args` (`febfd7f`) shows test code is held to the same `-D warnings` bar via `--all-targets`.
- `typos.toml` ignores: ellipsis-truncated string fragments (`"hel…"`), `mis-` prefixes, and `assets/codex/**/*.jsonl` (real captures). Allowed words: `ratatui`, `seeked`, `reviewr`. Add a word there with a comment saying why, as the existing entries do.

## Cargo features (enforced by the `core` and test jobs)

- `default = ["native"]`. `native` gates tokio, crossterm, futures, anyhow and the `tailer` live/terminal parts via `#[cfg(feature = "native")]`.
- Library code that must run without `native` (model, timeline, graph, UI rendering, providers) MUST NOT import those crates. `[[bin]] agents-graph` has `required-features = ["native"]`.
- `rataflow` is resolved from crates.io, not by path; local overrides go in a per-machine `.cargo/config.toml` `[patch.crates-io]`, never in `Cargo.toml` (comment in `Cargo.toml`).
- Dependency additions carry a short comment explaining why (observed throughout `Cargo.toml`, e.g. `rt-multi-thread` is "load-bearing", `imbl` backs O(1) snapshot clones).

## Module and doc-comment style (observed)

- Files open with a `//!` header that states the module's job, its vocabulary and the rule it protects, not a list of items. See `src/provider/mod.rs` (Core / Provider / Feeder vocabulary, the two boundaries, "Adding a provider") and `src/fact.rs` (the provider-boundary rule).
- Design decisions are written next to the code as the reason, including the rejected option. Example: `Provider` is "An enum, not a trait: providers arrive by pull request, and an exhaustive `match` makes the compiler list every site a new one must touch" (`src/provider/mod.rs`).
- `cargo doc -D warnings` makes intra-doc links (`[`Fact`](crate::fact::Fact)`) load-bearing; keep them valid when renaming.
- `src/lib.rs` embeds `README.md` as the crate front page (`#![doc = include_str!("../README.md")]`); README edits are therefore doc-build inputs.
- Visibility is deliberate: provider submodules are `pub(crate)`; only `Provider`, `Stream`, `open`, `sweep`, `provider_of` and the `fact` vocabulary are the surface feeders use. `src/lib.rs` marks the native frontend modules as "no stability promise".
- Section dividers inside a file use `// ----` banner comments (e.g. "The uniform surface: one stream per file" in `src/provider/mod.rs`).

## Provider extension rule (documented in `src/provider/mod.rs`; compiler-assisted)

A new transcript format is one directory `src/provider/<name>/` plus:

1. `wire.rs`: serde model, defensive. Unknown record types, missing fields and malformed lines parse to something skippable, never a panic.
2. `discovery.rs`: where the format keeps sessions, how to tell what a file is. Pure path logic, directory scans, at most a head read. Never assembles sessions, resolves ids, diffs rescans or tails.
3. `mod.rs`: the per-file `Stream` whose `push(line)` yields a `Statement`.
4. One new arm per `match` over `Provider` / `Stream` / `SessionFile` in `src/provider/mod.rs`; add the variant to `Provider::ALL`. The exhaustive `match` lists the sites; nothing above `provider/` should change.
5. Fixtures in `assets/<name>/`, shaped like the real layout so `discovery` finds them as it would a live session (see `assets/omp/`, `assets/codex/`). Fixtures are test-only: `include` in `Cargo.toml` keeps them out of the package.
6. A conformance test that calls `harness::conform(name, fixture, streams)` (`src/provider/harness.rs`, `#[cfg(test)]`). It checks order-independent folding, `assets/<name>/<fixture>.model.txt`, and `assets/<name>/<fixture>.timeline.txt`.
7. Regenerate goldens with `UPDATE_GOLDEN=1 cargo test ...`, then read the diff: the goldens are the human-readable record of what the provider extracts. Without the variable a missing or differing golden fails with that hint.
8. Pin provider detection (`provider_of`) against all formats when a new first-line shape is added (history: `bc05b18`, `4ddfee4`).

Where the repo shows a provider-specific shape (Claude single inherited timestamp, Codex learns file ownership from line one, omp joins children by name), `src/provider/mod.rs` says these are expected, not deviations.

## Facts (observed, `src/fact.rs`)

- A provider states what its records contain; the core decides what it means when nothing was recorded. Providers never conclude (no inferring "done" from a tool result; no invented outcome for a record without one: emit no `ToolEnd`, the call stays pending).
- Fact kinds are "named after what the record *is*, not what it implies" (doc comment on `FactKind`). Name new variants for the record, not the inference.
- Every fact carries the envelope `agent` + `ts`; absence reasoning lives in `SessionModel` only.

## Error handling (observed)

- `anyhow` is an optional dependency enabled by `native`. It is used in the binary and native frontend: `src/main.rs`, `src/herdr.rs` (`use anyhow::{Context, Result, anyhow, bail}`), `src/tui.rs`, `src/tailer/mod.rs`. `thiserror` is not a dependency.
- Portable-core parsing returns `Option` (e.g. `Stream::push -> Option<Statement>`, `Provider::parse -> Option<Provider>`); it does not return errors.
- User-facing CLI errors append the usage text (`anyhow!("--provider requires a name\n\n{USAGE}")` in `src/main.rs`) and name the missing prerequisite (`src/herdr.rs`: "HERDR_PLUGIN_STATE_DIR is not set; run this from a herdr action").
- Ordering of side effects matters for recoverability: close the pane first, forget its record after (`c0a5bf3`).
- No-`unwrap`/`expect` policy: Unknown / not confirmed. `grep -rn "unwrap()\|expect(" src` finds ~338 hits, mixed with test code; no lint forbids them. Verify with `cargo clippy -- -W clippy::unwrap_used` before asserting a rule.
- Test harness failures `panic!` with a remedy ("run with UPDATE_GOLDEN=1").

## Tests (observed)

- Unit tests are inline `#[cfg(test)] mod tests` (63 `cfg(test)` sites under `src/`, plus 3 `cfg(all(test, …))`); integration test: `tests/real_sessions.rs` (parses local real session files without panicking; see `60b58e4`). Benches: `benches/` (`timeline`, `memory`, criterion, `harness = false`).
- Tests must pass on Windows: build expected paths with `Path::join`/`display()` instead of hard-coded `/` (`d58ae7b`).
- A fix adds a regression test named for the behavior (e.g. `a_complete_line_longer_than_the_cap_is_kept`, `16081ca`). Details: [TESTING.md](../TESTING.md).

## UI and SEED tokens (observed; checked by issue #6 gates, not by CI)

- Every color comes from `ui::seed::theme::Theme`, which resolves SEED rootage tokens. No `Color::` literal in `src/ui` outside `src/ui/seed`, and none in `src/state` (gate G3 in `.issueops/issues/6/gates.md`, run by `gate.py`, not by CI).
- A new widget or view takes `&Theme`. View state (selection, view, overlays, hit map) is plain data in `src/state/view.rs`; `src/state` never imports `src/ui` outside tests.
- `src/ui/seed/tokens.rs` is generated: never edit it by hand. Re-vendor with `scripts/sync-seed.sh <full-sha>`, update `design/seed/SOURCE.md`, then `UPDATE_SEED=1 cargo test --locked seed_tokens`; view goldens may then need `UPDATE_GOLDEN=1`.
- Brand rule: SEED's brand role is drawn from the purple palette at the steps SEED uses carrot. No Daangn logo, name, character, or carrot accent; `design/seed/LICENSE` and `NOTICE` ship in release archives as `LICENSE-APACHE-SEED` / `NOTICE-SEED`. Decision: [SEED UI ADR](../adr/2026-10-09-the-terminal-ui-draws-only-from-pinned-seed-tokens-and-state.md).
- UI copy is English, like the README and `inspect` output.

## Other

- `docs/DESIGN.md` is zoetrope-origin and may use `zoe`/`zoetrope` names; prefer `docs/ARCHITECTURE.md` when they disagree.
- Upstream attribution (MIT, `LICENSE`, `NOTICE`) stays as-is (`195dbb8` constraint).
- Repo-root `AGENTS.md` carries general behavioral guidelines (simplicity, surgical changes, goal-driven verification); they apply here.

## Editing rules

- Follow existing style first; `cargo fmt --all` is the only formatter (defaults).
- Do not run repo-wide formatting beyond `cargo fmt` unless asked.
- Add a dependency only with a comment in `Cargo.toml` stating why, and check the `core` (no-default-features) and MSRV jobs still hold.
- Apply SOLID/YAGNI/KISS pragmatically: the repo's own precedent is an enum plus exhaustive `match` instead of a trait/registry for providers. Introduce a new abstraction only with a real variation point; record adoption in [ADR.md](../ADR.md).
