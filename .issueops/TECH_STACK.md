---
name: TECH_STACK.md
description: Chosen languages, runtimes, and tools; read before adding a dependency or tool.
---

# Tech Stack

Languages: Rust (crate `herdr-agents-graph`, edition 2024, `rust-version = "1.88"`; binary `agents-graph`, lib `agents_graph`) and bash (plugin scripts under `herdr-plugin/herdr/`, `scripts/`). Package manager: cargo. Toolchain: no `rust-toolchain*` file; CI uses `dtolnay/rust-toolchain@stable`, plus a job pinned to `rust-version`. Local toolchain: not pinned; the author's machine ran `rustc 1.97.0` / `cargo 1.97.0` on 2026-10-08 (observed with `rustc --version`), which is above the 1.88 floor.

Features: `default = ["native"]`; `native` enables tokio, crossterm, futures, anyhow, `ratatui/crossterm`, `rataflow/crossterm`. `--no-default-features` is the portable core (model, timeline, graph projection, UI, providers), checked in CI. Benches: `timeline`, `memory` (criterion, `harness = false`).

## Dependencies (Cargo.toml requirement → Cargo.lock resolved)

| Crate | Requirement | Locked | Role | Confidence |
|---|---|---|---|---|
| ratatui | 0.30 (no default features) | 0.30.2 | TUI rendering | High (manifest+lock) |
| rataflow | 0.1.0 | 0.1.0 | flow-graph layout/widget | High |
| crossterm | 0.29 (optional) | 0.29.0 | terminal backend | High |
| tokio | 1 (optional; `rt-multi-thread` load-bearing per manifest comment) | 1.53.1 | async runtime, tailing | High |
| futures | 0.3 (optional) | 0.3.34 | streams | High |
| anyhow | 1 (optional) | 1.0.104 | errors | High |
| serde / serde_json | 1 / 1 | 1.0.229 / 1.0.151 | transcript parsing | High |
| chrono | 0.4 | 0.4.45 | timestamps | High |
| web-time | 1 | 1.1.0 | portable time | High |
| unicode-width | 0.2 | 0.2.2 | text width | High |
| imbl | 7.0.1 | 7.0.1 | persistent data structures | High |
| terminal-colorsaurus | 1.0 (optional, `native`) | 1.0.3 | terminal background query (OSC 11) for light/dark theme | High |
| criterion | 0.8.2 | 0.8.2 | benchmarks | High (dev-dependency, html_reports) |
| yaml-rust2 | 0.13 | 0.13.0 | parses vendored SEED YAML in the token generator test | High (dev-dependency) |

Roles are brief inferences from crate purpose and manifest comments; dependency table lives in [Cargo.toml](../Cargo.toml).

## Release targets (`.github/workflows/cd.yml`)

`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` (via `cross`), `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Plugin installer supports the first four only.

## Tooling

- CI (`ci.yml`): rustfmt, clippy `-D warnings`, `cargo test --locked --all-features` on ubuntu/macos/windows, rustdoc `-D warnings`, `cargo publish --dry-run`, MSRV check, `crate-ci/typos` (`typos.toml`), commit lint (`committed.toml`).
- Changelog: `cliff.toml` (git-cliff; conventional commits only, per ci.yml comment).
- herdr plugin manifest `herdr-plugin/herdr-plugin.toml`, min herdr 0.9.3.
- Origin: fork of furkankly/zoetrope (MIT); see [NOTICE](../NOTICE), [docs/DESIGN.md](../docs/DESIGN.md), [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).
- Design tokens: SEED Design rootage (daangn/seed-design@22b68ce0, `@seed-design/rootage-artifacts` 3.0.2, Apache-2.0) vendored in `design/seed/` and compiled into `src/ui/seed/tokens.rs`; no YAML at runtime. See [conventions/overview.md](conventions/overview.md) "UI and SEED tokens".

## Adding a dependency

Keep it optional under `native` if it needs IO/runtime; the portable core must still build with `--no-default-features`.
