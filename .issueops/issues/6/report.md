# Issue #6 implementation report (draft)

~~~text
Status: implement and ai-slop-clean done; docs, verify, commit/push and draft PR pending
Lifecycle: io-4b513b64dc69
Mode/host/model: direct / omp / anthropic/claude-opus-5-5
Worktree/branch: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/6-seed-ui-redesign / 6-seed-ui-redesign
Lease generation: 2 (replace + claim after the generation-1 handoff; plan deebc075… and handoff ceb542fd… digests matched before claim)
Base: 1c744d24931a6634f8dea129c218f1823775f1c0 (main; sync-base preview: merge not needed)
Acceptance evidence: .issueops/issues/6/gates.md (G1-G12, G15, G16); .issueops/evidence/task-*.txt
Blockers: none
~~~

## What changed

| Area | Files | Plan task |
| --- | --- | --- |
| SEED rootage tokens pinned (daangn/seed-design@22b68ce0, rootage-artifacts 3.0.2) with LICENSE, NOTICE, SOURCE.md; generator test and generated token module; sync script | `design/seed/**`, `src/ui/seed/{mod,gen,tokens}.rs`, `scripts/sync-seed.sh`, `NOTICE`, `Cargo.toml` (yaml-rust2 dev, include) | T1 |
| Theme (light/dark × truecolor/256, OSC 11 detection, `AG_THEME`/`AG_COLOR`/`AG_BG`) and SEED widgets | `src/ui/seed/theme.rs`, `src/ui/seed/widgets/*`, `Cargo.toml` (terminal-colorsaurus, native only) | T2 |
| Tool-call intent and recorded cost in the fact vocabulary and model | `src/fact.rs`, `src/provider/{omp,claude,codex}/*`, `src/state/session.rs` | T3 |
| View state (view, shared selection, snackbar, flash, hit map, display offset, loop clock) in `App`; state no longer imports ui; start-ordered tool calls; tree order, intent line, in-flight queries | `src/state/{mod,view,graph,session}.rs` | T4 |
| Now view, Lanes view, Graph view and detail panel | `src/ui/views/{now,lanes,graph,detail,mod}.rs`, `src/ui/text.rs` | T5-T7 |
| Shell (top bar, hint bar, overlays), keymap, mouse, redraw predicates, slow idle tick, input drain after the theme query | `src/ui/{mod,chrome}.rs`, `src/handler.rs`, `src/state/frame.rs`, `src/tui.rs` | T8 |
| `--pending-tool` demo copy for the idle measurement | `scripts/make-running-demo.sh` | T9 |
| 12 snapshot goldens named by token; README screen, keys, colours, measurements; release archives carry the SEED license | `src/ui/{snapshots.rs,seed/snapshot.rs}`, `assets/ui/*`, `README.md`, `.github/workflows/cd.yml` | T10 |

Removed: `src/ui/{chips,panel,nodes,edges}.rs` (replaced by the views).

## Deviations from the plan

- **UI copy is English** (tabs `Now · Lanes · Graph`, `Enlarge the window`, `No tool calls recorded`): the README, `inspect` and the 0.1.0 UI are English; the plan's Korean mockups describe meaning. Status words stay `AgentInfo::status_word()` (single source).
- **The theme is owned by the frontend loop, not by `App`**: `ui::draw(frame, app, theme)`. Putting a `ui::seed::Theme` in `App` would have made `state` import `ui`, which T4 forbids; the theme is still not a global.
- **SEED commit**: the plan's `22b68ce26…` does not resolve; `22b68ce014c894edf286a2328a96250a36ab33a2` (the npm 3.0.2 gitHead `22b68ce`) is vendored.
- **`gen` module**: `gen` is reserved in edition 2024; the file stays `gen.rs` and is mounted as `generator` (the `ui::seed::gen` test filter still matches).
- **No `CHANGELOG.md`**: release-plz writes it from commits (`cliff.toml`); the keymap change table is in the README and goes in the commit body.
- **Idle tick**: the loop ticks at 16 ms only while the picture moves (playback, glide, marching ants, a flash, a drag) and at 200 ms otherwise. Needed for G10: on this machine 0.1.0 itself measured 0.63-0.67 % idle on 2026-10-09; with the slow tick the redesign measures 0.33 % / 0.17 % / 0.17 %.
- **Lanes cursor**: the axis ends at `now_reference()` (the playhead when replaying or scrubbed back), as the plan specifies, so the cursor sits at the right edge and `←`/`→` re-scale the axis by one column per press.
- **Tree connectors** in the Now view are indentation only (expanded calls use `├`/`└`).

## Side effects

- New runtime dependency `terminal-colorsaurus` (native feature only); new dev-dependency `yaml-rust2`.
- On start, one OSC 11 + DA1 query to the terminal (skipped with `AG_THEME`, `TERM=dumb`, or a non-tty), waiting up to 1 s for a terminal that never answers; leftover input is drained after raw mode.
- Release archives gain `LICENSE-APACHE-SEED` and `NOTICE-SEED`; `install.sh` extracts only the binary, so installs are unaffected.
- No on-disk writes, network access, schema or provider-rule changes; provider goldens unchanged (G12).
- The loop wakes 5×/s instead of 60×/s while nothing on screen moves by itself (`App::in_motion`).

## Performance (2026-10-09, this machine)

| Measure | 0.1.0 (base binary) | this branch |
| --- | --- | --- |
| 521 MB Codex rollout `inspect`, peak RSS | 55 MB (issue #1) | 54,968,320 B (G9) |
| idle CPU, finished demo `--follow` 200×60 | 0.63-0.67 % | 0.30 % |
| idle CPU, Running main copy | — | 0.17 % |
| idle CPU, pending-tool copy (timer ticking) | — | 0.13 % |
| first frame, `AG_THEME` set / terminal answers OSC 11 | 3 ms | 3 ms / 3 ms |
| first frame, terminal never answers | — | 1,015 ms (median of 5) |
| real session files parsed (AG_REAL_SESSIONS) | — | 3,172, failed 0 (G11) |
| release binary | 4,084,880 B | 4,280,512 B |

Evidence: `.issueops/evidence/task-9-perf.txt`, `task-9-startup.txt`, gates.md.

## ai-slop-clean

Independent reviewer pass over the diff (25 findings), then fixes:

- **dead-code**: scrubber log-line model (`LogEvent`, `LogKind`, `latest_event_at` + 2 tests); scrubber-only `Timeline` helpers (`has_span`, `progress`, `bar_fraction_for_index`, `gap_markers`, `prompt_markers`, `GAP_MARKER_SECS` + 3 tests); `ProgressDots`, `KeyHint::key_width`, `Tone::ALL`, `ToneColors::stroke`; unused `help` parameter; test debug prints; `App::transport` made test-only.
- **duplication**: widget-local `truncate` → `ui::text::truncate`; `agent_name`/`node_title` → `AgentInfo::display_name`; two `said` helpers → `session::said`; slow-tool count → `SessionModel::slow_tools` (stamp and hint bar share it, `SLOW_TOOL` lives in the model); tab labels from `View::label`, tab width from `TabList::width`; lanes bar colours via `tool_tone`; gen.rs reuses `theme::xterm_rgb`; detail era walk once per frame; lanes agent list once per frame; omp intent shaping in `stated_intent`; three identical glide literals in tests → one const.
- **boundary-violation**: lanes' running rule now matches `SessionModel::in_flight` (pending and agent not terminal).
- **weak-artifact**: stale chips/scrubber/panel/status-bar comments in `state/mod.rs`, `tui.rs`; docs/DESIGN.md and docs/ARCHITECTURE.md rewritten where they described the removed UI; Divider widget now draws the detail's rule.
- **unsupported-claim**: "a frame is a function of App and Theme alone" narrowed (live edge reads the system clock via `now_reference`); README "nothing depends on telling colours apart" made true instead (failed lane bars end in `✗`, cell-level graph cards keep their glyph) and worded to what the code does; README "latest reasoning" → "the thought that led to its spawn"; intent_line doc no longer claims a first-line cut.

Heuristic metrics (`/tmp/ag-qa/metrics.py`: comments = noise, `use`/attribute lines = boilerplate, branch tokens per fn; whole changed files, so pre-existing provider fns dominate entropy): SNR 0.901 → 0.895; fns > 12 branches 22 → 23 of 612 → 657 (file set grew by timeline.rs; new-code maxima are the flat keymap `handle_key` and `graph::render`'s file tail); same-file duplicate 7-line blocks 10 → 9 (the 9 left are pre-existing transport tests in state/mod.rs); files > 50 % boilerplate 1 → 1 (`src/ui/seed/mod.rs`, module declarations only). Snapshots in `.issueops/evidence/code-quality-metrics/`.

Verification after the last pass: `cargo fmt --check`, `cargo clippy --all-targets --locked -D warnings`, `cargo test --locked` (304 + 13 + 1 + 0 doc), `cargo check --lib --no-default-features`, `RUSTDOCFLAGS=-D warnings cargo doc --all-features --no-deps`, `cargo publish --dry-run --locked --allow-dirty`, `git diff --check`: all pass. gate.py G1-G8, G12, G15, G16: PASS.

## Implementation review

Round 1 (independent reviewer, plan + diff + plan-review claims; lenses reuse, perf, compat, side-effect, accessibility, responsive, motion): **revise**. All seven carried-over plan claims were kept. Fixed:

| Finding | Fix | Proof |
| --- | --- | --- |
| Lanes → stepped backward and ← two columns: the axis ends at the cursor off the edge | `lane_cursor` steps to the previous column start before the cursor, or forward to the next column start / one column of time; past the head it goes live | `lane_arrows_move_the_cursor_by_columns` now redraws between presses |
| `tokens.rs` compared byte-for-byte; CRLF checkout on windows-latest fails | normalize `\r\n` before comparing | — (Windows CI) |
| `insert_call` walked all prior calls on every append (imbl `skip` is O(n)) | indexed re-index of the shifted calls only | `tool_calls_are_start_ordered` |
| SEED generator tests panic in the published tarball | skip loudly when `design/seed/color.yaml` is absent | lib tests from `cargo package` tarball: 307 passed |
| Lanes `skip(range.start)` defeated the binary search | indexed access over the visible range | lanes tests |
| A late OSC reply arrives as Alt+`]` and steps the timeline | keys with Alt but not Ctrl are ignored (Ctrl+Alt is Windows AltGr, which types `[` `]` on many layouts; delta review) | `alt_modified_keys_are_ignored_but_altgr_keys_work` |
| Detail "tool calls" label in a stroke token (~1.3:1); brand fg as text ~2.9:1 in light | label `fg.neutral-subtle`; text uses `brand fg-contrast` (lane name, prompt label) | goldens |
| Now list < 60 cols at 100-104 columns with the side detail open | list `Min(NARROW)`, detail ≤ 40 % | `side_detail_keeps_the_list_full_width_from_100_columns` |

Snapshot serialization now names a colour by the token most fitting its role (text → `fg.*`, background → `bg.layer-*`), so goldens read `bg=bg.layer-default` rather than an alias that shares the value.

Delta review: the eight fixes verified; one regression found in the first Alt guard (it dropped AltGr `[` `]` on Windows) and fixed as above.
