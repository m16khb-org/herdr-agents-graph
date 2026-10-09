---
name: ARCHITECTURE.md
description: System structure and component boundaries; read before adding a component or moving a responsibility.
---

# Architecture

Dependency direction and runtime topology. This root is the family index; focused detail lives under [architecture/](architecture/).

- [Architecture overview](architecture/overview.md): crate layout, the provider → fact → state → ui pipeline, feeders (live / replay / inspect), the herdr bridge, observed style (layered pipeline, not hexagonal).

## Authoritative repo docs (read these for detail; this index does not copy them)

- [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md): invariants (order-independence, content vs presentation clocks, derived-state heuristics).
- [docs/DISCOVERY.md](../docs/DISCOVERY.md): how a feeder turns a target into files, then sessions (`src/provider/mod.rs`).
- [docs/HERDR-PLUGIN.md](../docs/HERDR-PLUGIN.md): the herdr plugin contract.
- [docs/DESIGN.md](../docs/DESIGN.md): zoetrope-origin v1 structural spec and module map; the map matches `src/` except it omits `src/provider/summary.rs` (checked against `find src`).
- Known hazard: [cautions/2026-10-09-a-herdr-plugin-pane-s-own-context-names-the-focus-at-pane-st.md](cautions/2026-10-09-a-herdr-plugin-pane-s-own-context-names-the-focus-at-pane-st.md) (`AGENTS_GRAPH_PANE` handoff).

## Boundaries to keep (summary)

- Providers state facts; nothing past `src/fact.rs` knows a transcript format.
- Feeders (tailer, inspect) reach providers only via `provider::{open, sweep, provider_of}` and the `Provider` primitives.
- The `native` cargo feature gates tokio/crossterm/filesystem code (`tui`, `handler`, most of `tailer`); the rest is IO-free.

## Appending knowledge

- Append new dated records with MCP `project_docs_append` or `issueops project append`.
- Records are written as one file per record inside the module directory, so this index stays small.
- Revise this index only to add links to new curated modules; keep it within the manifest line budget.
