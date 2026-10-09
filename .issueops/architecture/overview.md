---
name: overview
description: Family module overview: dependency direction and runtime topology.
---

# Architecture — Overview

Canonical index: [ARCHITECTURE.md](../ARCHITECTURE.md)

Detail lives in repo-authored docs: [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md) (invariants), [docs/DISCOVERY.md](../../docs/DISCOVERY.md) (target → files → sessions), [docs/HERDR-PLUGIN.md](../../docs/HERDR-PLUGIN.md), [docs/DESIGN.md](../../docs/DESIGN.md) (v1 module map). This page is the map; current code wins over all of them.

## Purpose

`herdr-agents-graph` is one Rust 2024 crate: library `agents_graph` (`src/lib.rs`) plus binary `agents-graph` (`src/main.rs`, `required-features = ["native"]`, `Cargo.toml:84-88`). It reads Claude Code / Codex / omp session transcripts from local disk and renders them as a live flow graph TUI (ratatui + rataflow). A herdr plugin under `herdr-plugin/` launches it. No network IO in the binary (docs/DESIGN.md hard constraint 1).

## Observed style

A **layered one-way pipeline** with a provider/feeder split: `provider → fact → state → ui → tui`. It is **not hexagonal / ports-and-adapters**: there are no port traits with swappable adapters. `Provider` is a closed `enum` (`src/provider/mod.rs:87`) and `Stream` a closed `enum` (`:121`); adding a provider means a new directory plus one arm in each `match` (`src/provider/mod.rs` module doc, "Adding a provider"). `state` is the domain core and also owns presentation state (`App`), so it is not a pure domain layer either.

## Crate layout

| Path | Role |
| --- | --- |
| `src/main.rs` | CLI parse (`parse_cli`, `enum Cli { View, Inspect }`), `run_tui`, `run_inspect`, `parse_session_fully`, `resolve_target`; `herdr` subcommand is dispatched before CLI parsing (`main`, line ~328). |
| `src/herdr.rs` | `mod herdr` of the binary only (not in lib): `agents-graph herdr resolve \| toggle [overlay\|split\|tab\|zoomed]`. |
| `src/lib.rs` | Exports `fact`, `provider`, `state`, `tailer`, `ui`; `handler` and `tui` only under `feature = "native"`. Crate docs include `README.md`. |
| `src/fact.rs` | Vocabulary every provider reduces to: `Fact`, `FactKind`, `Statement`, `Outcome`, `AgentKind`, `AgentStatus`. |
| `src/provider/` | `mod.rs` (`Provider`, `Stream`, `SessionFile`, `Session`, `Scope`, `Target`, `open`/`sweep`/`assemble`/`provider_of`), `claude/ codex/ omp/` (each `mod.rs` + `wire.rs` + `discovery.rs`), `summary.rs` (tool-summary helpers), `harness.rs` (test-only golden/conformance check). Fixtures: `assets/{claude,codex,omp}/`. |
| `src/tailer/` | Feeders: `mod.rs` (`TailRequest`, `UiEvent`, `run`), `live.rs` (poll loop), `replay.rs` (up-front assembly), `bytes.rs` (incremental line reader), `item.rs` (`ReplayItem`, `Timing`, `Bundle`; IO-free). |
| `src/state/` | `mod.rs` (`App`), `session.rs` (`SessionModel`, `MAIN_ID`), `timeline.rs` (`Timeline` playhead), `graph.rs` (`AgentFlow`: model → rataflow `Flow`), `frame.rs` (redraw gate), `info.rs` (`SessionInfo`), `render.rs` (text report used by `inspect` and golden tests). |
| `src/ui/` | `mod.rs` (`draw`), `nodes.rs` (`AgentNode`), `edges.rs` (`AgentEdge`), `chips.rs` (`ChipTray`), `panel.rs` (detail panel). |
| `src/tui.rs`, `src/handler.rs` | Native terminal loop (16 ms tick, draw only when stale) and input routing. |
| `herdr-plugin/` | `herdr-plugin.toml` manifest, `herdr/{install,open,pane,keys}.sh`, `README.md`. |
| `tests/real_sessions.rs`, `benches/{timeline,memory}.rs`, `scripts/` | Integration test, criterion benches, helper scripts. |

## Pipeline and dependency direction

```
transcript files ──► provider/<fmt>  ──► Statement{Fact…} ──► state::SessionModel / SessionInfo / Timeline ──► state::AgentFlow (rataflow Flow) ──► ui::draw ──► tui
                      (wire+discovery)    (src/fact.rs)         (App owns all)                                  (graph.rs)                      (ratatui)
```

- Providers import `state::session::MAIN_ID` (`provider/{claude,codex,omp}/mod.rs`) and `SessionModel` in `provider/harness.rs` (test harness). `tailer/item.rs` and `tailer/replay.rs` import `MAIN_ID` only in test code (`item.rs:355`, `replay.rs:157`, inside test modules).
- `ui` reads `state` freely (`App`, `SessionModel`, `AgentFlow`, `Camera`, …).
- **Existing exception, `state → ui`:** `state/graph.rs:13-14` imports `ui::edges::AgentEdge` and `ui::nodes::{AgentNode, MAIN_NODE_DIMS, SUB_NODE_DIMS}`; `state/mod.rs:24` re-exports `ui::chips::ChipTray`, and `App` holds `crate::ui::ScrubberTally` (`:204`) and `crate::ui::panel::EraCache` (`:208`). The graph projection must build ui node/edge content, so `state` and `ui` are mutually dependent. Do not add more state → ui imports without a reason.
- `state → handler` appears only in tests (`state/mod.rs:1273` after `mod tests` at `:1054`; `state/frame.rs:216-233` in its test module).
- Feeders know where bytes come from, not what they mean; they reach providers only through `provider::{open, sweep, provider_of}` and `Provider` methods (`provider/mod.rs` module doc).

## Data flow per mode

All modes end in the same `App` and `Timeline`; live and replay are one time-shifted timeline (`state/timeline.rs` doc), not two engines.

- **Live follow** (`agents-graph`, `<dir>`, or `<file> --follow`): `run_tui` builds `Target::Here` (or `Path/Id` with `Mode::Live`), spawns `tailer::run(..., replay=false, ...)` → `run_live` poll loop; it emits `UiEvent::Batch{statements}` over a bounded mpsc (cap 32, `main.rs` `CHANNEL_CAP`), and may switch to a newer session of the watched dir. Polling interval: 200 ms, backing off after 30 s quiet (`tailer/live.rs`, docs/DESIGN.md:194).
- **Replay** (`<file>` or `<id>`, optional `--speed`, default 8.0): `tailer::run(..., replay=true, ...)` → `run_replay` parses every file up front, merges by timestamp, sends one `UiEvent::ReplayLoaded{items, speed, info}`; the `App` then owns pacing/seek and the tailer keeps tailing appends.
- **Inspect** (`agents-graph inspect <file|id|dir>`): headless, no tailer task, no TUI. `parse_session_fully` calls `provider::open`, streams each file with `tailer::read_lines` + `Provider::stream_for`, folds statements into `SessionModel`/`SessionInfo`, then `state::render::report` prints it (`main.rs:169-240`).
- **TUI loop** (`tui::run`): single task; owns `App`; drains `UiEvent` and crossterm input (`handler::handle_event`), ticks the camera/timeline, draws via `ui::draw` only when `RedrawGate` says the frame is stale. State is single-owner; background tasks talk over channels (docs/DESIGN.md hard constraint 3).
- `tui::run` holds the `TailRequest` sender only to keep the channel open; closing it makes the tailer exit (`tui.rs:34-45` comment).

## herdr bridge

- Manifest `herdr-plugin/herdr-plugin.toml`: id `m16khb.herdr-agents-graph`, `min_herdr_version = "0.9.3"`, platforms macos/linux, build step `bash herdr/install.sh` (downloads the release binary for manifest `version` into `herdr-plugin/bin/`, checks SHA256SUMS), pane `graph` (overlay, command `herdr/open.sh`), actions `open`/`open-split`/`open-tab` (→ `herdr/pane.sh <placement>`), `setup-keys`/`remove-keys` (→ `herdr/keys.sh`).
- `pane.sh` is an `exec` of `agents-graph herdr toggle <placement>`. `toggle` (`src/herdr.rs:188`) records the opened pane id in `$HERDR_PLUGIN_STATE_DIR/open-pane`, closes it on the next press, else runs `herdr plugin pane open --entrypoint graph --placement …`, passing `AGENTS_GRAPH_PANE=<focused_pane_id>` (and `--target-pane` only for `split`).
- `open.sh` runs `agents-graph herdr resolve` → `<provider> <id-or-path>`, then `agents-graph --provider <p> --follow <target>`; resolve failures exit 2 and the script holds the message until enter.
- `resolve` reads `AGENTS_GRAPH_PANE` before `HERDR_PLUGIN_CONTEXT_JSON` (`src/herdr.rs:32,173`). Reason and test: [caution](../cautions/2026-10-09-a-herdr-plugin-pane-s-own-context-names-the-focus-at-pane-st.md), test `the_named_pane_outranks_the_pane_commands_own_focus`.
- All herdr JSON handling is in Rust (`herdr.rs`); the shell scripts need only bash. The binary shells out to `herdr` (`HERDR_BIN_PATH` or `herdr`), not a network call.

## Verified staleness / traps

- Code comments mention a **browser/wasm frontend** that is not in this repo (the fork removed `web/`): `src/provider/mod.rs:16,74,415`, `src/tailer/item.rs:4,194` (`Bundle`: "the browser's feeder"), `src/tailer/mod.rs:36,47`, `Cargo.toml:37,51,79` (wasm deps). `tailer::Bundle` (`item.rs:204`, re-exported at `tailer/mod.rs:38`) is still consumed by the benches (`benches/common/mod.rs:148`), so it is live code; only the browser wording is stale.
- `README.md:67-95` and `docs/DESIGN.md:3` name zoetrope: fork attribution and a benchmark comparison, not stale names. `grep -in zoe` found no zoe names in `docs/ARCHITECTURE.md`, `docs/DISCOVERY.md`, `docs/HERDR-PLUGIN.md`.
- `docs/DESIGN.md` is the "v1 structural spec"; its header (line 5) defers to `docs/ARCHITECTURE.md` for invariants. Its module map (`:68-117`) matches the files under `src/` today except `src/provider/summary.rs` (used by `provider/{claude,codex,omp}/mod.rs`), which is not listed.
- `src/ui/` is being replaced by a SEED design-token layer in issue #6 (https://github.com/m16khb-org/herdr-agents-graph/issues/6). This page describes `main` as it is now; re-check the ui row and the `state → ui` exception after that lands.

## Guidance

- Before large design changes, read the entrypoints above and `docs/ARCHITECTURE.md` invariants (fold order-independence is enforced by `provider/harness.rs` shuffled-order tests).
- New provider: one directory + one arm per `match` (compiler lists them) + fixture under `assets/<name>/` + one `harness::conform` call. Nothing above `provider/` changes.
- Keep `fact`, `state`, `ui` IO-free; IO belongs to `tailer`, `tui`, `handler`, `main`, `herdr`.
