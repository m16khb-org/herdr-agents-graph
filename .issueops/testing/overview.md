---
name: overview
description: Family module overview: test strategy and verification gates.
---

# Testing — Overview

Canonical index: [TESTING.md](../TESTING.md)

## Purpose

Read before writing or modifying tests. Records the real verification commands (from CI and repo scripts), the fixture/golden mechanics, and measured performance gates.

## When to read

- Writing tests for a feature or bug fix
- A test fails or is flaky, especially only on one OS
- Proving behavior preservation after a parser/provider refactor
- Deciding which verification to run before completion

## CI jobs and commands

Source: `.github/workflows/ci.yml` (runs on `pull_request` and push to `main`).

| Job | Command |
|---|---|
| Format | `cargo fmt --all --check` |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test (matrix `ubuntu-latest`, `macos-latest`, `windows-latest`, `fail-fast: false`) | `cargo test --locked --all-features` |
| Check portable core | `cargo check --lib --locked --no-default-features` |
| Docs (`RUSTDOCFLAGS=-D warnings`) | `cargo doc --locked --all-features --no-deps` |
| Package | `cargo publish --dry-run --locked` |
| MSRV | `cargo check --lib --locked --all-features` on the `rust-version` read from `cargo metadata` (package `herdr-agents-graph`) |
| Typos | `crate-ci/typos@master` |
| Lint commits (PR only) | `crate-ci/committed@v1.1.11 -vv` over the PR range; if the base lacks upstream tip `b1f31dd2…`, lint from that tip |

Notes:
- `core` exists because `default-features = false` (no tokio/crossterm/fs) is the portable core a browser frontend depends on (comment in ci.yml; features in `Cargo.toml`). Code behind `#[cfg(feature = "native")]` (e.g. `draw_every_tick_while_auto_panning` in `src/state/frame.rs`) is not compiled there.
- `cargo package`-style builds exclude `assets/` (`Cargo.toml` `include = ["/src/**/*.rs", "/README.md", "/LICENSE", "/NOTICE"]`), so fixture tests must skip, not fail, outside a checkout (see `fixture_dir`).
- Commit messages are checked; see [COMMIT_POLICY.md](../COMMIT_POLICY.md). A non-conventional commit is dropped from the changelog (`cliff.toml`, per ci.yml comment).
- Release builds are in `.github/workflows/cd.yml`; it builds with `cargo build --release --locked --target …`. Whether cd.yml runs tests: Unknown / not confirmed — read `cd.yml` to verify.

Full local pre-push set: the first three rows plus `cargo check --lib --locked --no-default-features` and `RUSTDOCFLAGS="-D warnings" cargo doc --locked --all-features --no-deps`. No `cargo test --no-default-features` job exists; Unknown whether it passes.

## Golden-fixture conformance harness

`src/provider/harness.rs`, `conform(provider, fixture, streams)`, called from `provider/claude/mod.rs` (`demo`), `provider/codex/mod.rs` (`captures_conform`, one per fixture dir), `provider/omp/mod.rs` (`demo`). Each call checks:

1. **Order invariance** — `assert_order_invariant` folds the per-file fact streams in 40 deterministic LCG interleavings (seeds 0–39) and requires an identical model snapshot (`"order-dependent state at seed {seed}"`).
2. **Model golden** — `assets/<provider>/<fixture>.model.txt`.
3. **Timeline golden** — `assets/<provider>/<fixture>.timeline.txt`.

Fixtures present: `assets/claude/demo`, `assets/codex/{cli-0.149.1,cli-0.153.4,desktop-0.150.0,usage-records-only}`, `assets/omp/` (`demo` + one dated session dir). Each has `.jsonl`/dir plus `.model.txt` and `.timeline.txt`.

Rules:
- Regenerate with `UPDATE_GOLDEN=1 cargo test --locked <name>`; a mismatch prints the first differing line and says to rerun with it only "if the change is intended". Explain every golden diff in the commit (broad snapshot updates need stated intent).
- Golden comparison tolerates CRLF checkouts (comment in `golden()`); keep that if touching it.
- Missing `assets/<provider>/` prints `skipping: no fixtures …` and the test passes — a green run in a non-checkout proves nothing about parsers.
- Fixtures are synthetic/redacted demos; `assets/` is not shipped. Do not add real transcripts (the real-session test deliberately reports paths and failure kinds only, never content).

## Real-session test (opt-in)

`tests/real_sessions.rs`, test `every_local_session_file_parses`:

```text
AG_REAL_SESSIONS=1 cargo test --release --test real_sessions -- --nocapture
```

- Without `AG_REAL_SESSIONS` it prints `real_sessions: skipped (set AG_REAL_SESSIONS=1)` and **passes**.
- Roots: `$HOME/.claude/projects`, `$HOME/.codex/sessions`, `$HOME/.omp/agent/sessions`; `AG_REAL_SESSIONS_ROOT=<dir>` replaces all three. Uses `HOME`, so it is not Windows-portable as written (Unknown whether it is run on Windows; CI never sets the flag).
- A file fails when no provider recognizes it, parsing panics, or a non-empty file yields no records. Unjoinable files (archived Codex root, subagent without parent) parse alone and count as `standalone`.
- Summary line: `real_sessions: total=N failed=M standalone=K`; failures are listed as `failed: <path> (<why>)`. Gate G6 evidence (issue #1): `total=3153 failed=0 standalone=123` (one machine, 2026-10-09).

## Benches

Both in `Cargo.toml` as `harness = false`, over synthetic sessions from `benches/common` (never real transcripts).

- `cargo bench --bench timeline` — criterion; groups `parse`, `load`, `seek` (`seek_back_*` = O(k) `App::rebuild_to`) at three scales.
- `cargo bench --bench memory` — not criterion; prints a table of live heap per loaded session using a counting global allocator (`Counting`, `LIVE` bytes), not RSS. Purpose: check snapshot-ladder rungs are worth their retained memory.
- Exact `cargo bench` invocations are inferred from the `[[bench]]` names; no CI job runs them. Stored baseline numbers: Unknown / not confirmed.

## Performance scripts and gates

- `scripts/idle-cpu.sh <session.jsonl>` — runs `agents-graph <file> --follow` in a 200x60 pty (`script`), samples CPU time at 5 s and 35 s, prints `idle_cpu_percent=<v>`. Needs a release binary (`cargo build --release`, or `AG_BIN=…`). Uses `pgrep`/`ps`/`script`; written for macOS/Linux shells.
- `scripts/make-running-demo.sh <out.jsonl>` — copies `assets/claude/demo.jsonl` and appends an assistant line dated 40 s ago so the main agent is `Running` (an interactive main stays Running 120 s after last activity). Regenerate right before each measurement; do not reuse the file.
- Thresholds from `.issueops/issues/1/gate.py` (ledger: [issues/1/gates.md](../issues/1/gates.md)):
  - G4: `agents-graph inspect --provider codex <largest rollout>` max RSS ≤ 104857600 B (100 MB); baseline file 521 MB, summary `1 agent(s), 813 tool call(s)`.
  - G5: idle CPU ≤ 0.5 % for both `assets/claude/demo.jsonl` and a make-running-demo copy, and the copy's `inspect` reports the main as active.
  - G11: `inspect` of the 96 MB Claude session ≤ 0.30 s real (best of 3 recorded 0.07 s, RSS ~16.8 MB).
  - G12: codex goldens unchanged and the 521 MB rollout reports tokens 246129.
- `gate.py` reads session files from the author's machine (largest under `~/.codex/sessions`, etc.); it cannot be reproduced without equivalent local sessions.

## TestBackend and UI tests

- `src/state/frame.rs` tests drive `RedrawGate` (draw-skip logic) with a simulated clock (`Instant` offsets and `TICK`), not sleeps. `draw_every_tick_while_auto_panning` renders once via `ratatui::Terminal::new(TestBackend::new(80, 24))` + `crate::ui::draw(f, &mut app, &theme)` with a fixed dark truecolor `Theme`, then feeds mouse events through `crate::handler::handle_event`.
- **View goldens** (`src/ui/snapshots.rs`, native only): the omp fixture replayed to its end, rendered for each view (Now, Lanes, Graph) × theme (light, dark) × size (80x24, 120x40), 12 files `assets/ui/<view>-<theme>-<WxH>.txt`. Each holds the frame's text and its style runs named by SEED token (`seed::snapshot`), not RGB, so a token value change does not churn them. Clock times print in UTC. Regenerate with `UPDATE_GOLDEN=1` and review the diff like the provider goldens.
- **Token generator** (`src/ui/seed/gen.rs`, `seed_tokens_match_vendored_yaml`): regenerates `tokens.rs` from `design/seed/*.yaml` and fails on drift; `UPDATE_SEED=1` rewrites it. Outside a git checkout (no `design/seed/color.yaml`, as in the published crate) it prints `skipping:` and passes. It also pins the brand swap (`brand_maps_to_purple_not_carrot`).
- Theme selection is tested without a terminal: `decide()` takes the environment snapshot and the OSC 11 query as a closure (`decide_prefers_ag_theme_without_querying`, `decide_uses_query_then_colorfgbg_then_dark` in `src/ui/seed/theme.rs`).
- Idle CPU after the redesign (issue #6 gate G10, one machine): finished demo 0.30 %, Running copy 0.17 %, pending-tool copy 0.13 % (`scripts/make-running-demo.sh --pending-tool` makes the last one).

## Platform pitfalls observed

- **Windows path separator** (commit `d58ae7b`): `Path::join` yields `/home/me\.omp/...` on Windows while the test expected `/`. Fixed in `resolve_accepts_only_known_kinds` (`src/herdr.rs`) by building the expectation with `Path::new("/home/me").join(...).display().to_string()`. Build expected paths the way production does.
- **CRLF goldens**: Windows checkouts may carry CRLF; `golden()` compares by content (harness.rs).
- **Local green ≠ CI green**: that commit was verified on macOS (`239 + 13 + 1 passed`) and left Windows to CI. Tests touching paths, line endings, or HOME need thought for all three matrix OSes.

## Well-structured tests

- Directly verify changed behavior through public contracts/observable behavior; deterministic, independent, no sleeps/network.
- Example: `draw_skipped_when_clean` and `running_without_pending_draws_at_most_once_per_second` (`src/state/frame.rs`) — fake clock, one stated behavior each, assertion messages like `"the first frame draws"`; the second carries a doc comment explaining why the bound holds.
- Example: `assert_order_invariant` (`src/provider/harness.rs`) — a seeded LCG makes the order-dependent failure reproducible (`"… at seed {seed}"`), and it returns the baseline so callers assert semantic anchors.
- Regression tests encode the recurring input and expected result; reuse existing helpers (`finished()`, `live()`, `run()` in frame.rs; `harness::conform`).

## Poorly-structured tests (and traps in this repo)

- Asserting platform-specific literals, as `resolve_accepts_only_known_kinds` did before `d58ae7b` (hard-coded forward-slash expected path; failed on windows-latest).
- Skip-and-pass tests that hide missing coverage: `fixture_dir` and `every_local_session_file_parses` pass when fixtures or the opt-in flag are absent. Check the `skipping:` / `skipped` output before claiming a parser was verified.
- Locking only implementation structure, assertions not tied to a bug or requirement, sleeps, local machine state, huge or vague snapshots, or weakening production code to pass.
- Gate evidence that proves nothing: in `.issueops/issues/1/gates.md` G1 records `0 passed; 0 failed` (an empty test binary) as PASS evidence, and G3/G10 evidence lines only show `Running …` headers. When claiming verification, quote the `test result: ok. N passed` line for the named test, not just exit code.

## Rule

Before running, prefer the commands in `.github/workflows/ci.yml` over guesses; they are what gates a PR. Apply the well/poorly-structured criteria above to new tests.
