# Issue #1 implementation report

~~~text
Status: completed (draft PR published and verified; completion recorded with this report)
Lifecycle: io-4b513b64dc69
Mode/host/model: direct / omp / anthropic/claude-opus-5-5 (high)
Worktree/branch/final HEAD: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp / 1-agents-graph-mvp / the commit that adds this report (passed as --final-head)
Lease generation/completion: generation 2 (replace + claim after the generation-1 handoff); completed by `issueops execution complete`
Issue/packet digests: verified — plan.md 474d3f0b6c8c6f8e0c1e05ea3103cd0215f2f7b7c7dc615dda86916a7037e4d5 and handoff.md 08b7fe921d9ae1efa12222d6e54fbfbfb1003553f218cf6a4e24c78a231b9fc5 matched before claim
Commits: listed below (base 526801f, first parent)
Changed files: git diff 526801f..HEAD (fork import of zoetrope plus the files named in each commit below)
Acceptance evidence: G1-G12 12/12 met (.issueops/issues/1/gates.md); T10 three-agent herdr QA (evidence/task-10-herdr-qa*.txt); intent check below
Verification: cargo test --locked 239+13+1 PASS; clippy -D warnings PASS; cargo doc -D warnings PASS; --no-default-features PASS; real_sessions 3153/0 failed PASS; inspect over 3150 files 0 abnormal exits PASS; gates 12/12 PASS; implementation review round 2 PASS; cd.yml not run (needs a v* tag) SKIP
AI-slop clean: removed the recording-only autopilot (476 lines), narrowed EXIT_UNRESOLVED, corrected the README release claim
Draft PR/MR: https://github.com/m16khb-org/herdr-agents-graph/pull/2 (draft, open, head 1-agents-graph-mvp, base main, label enhancement, assignee m16khb, closes #1; verified with remote verify-artifact)
Deviations: see "Deviations from the plan" below
Blockers: none
~~~
## Commits (first parent, base 526801f)

- 195dbb8 chore(fork): import furkankly/zoetrope@b1f31dd
- ef49e61 chore(identity): rename crate to herdr-agents-graph and drop zoetrope-only assets
- 4ddfee4 feat(provider): read omp and pi sessions, subagent graph included (PR #26 by bl-dev0, cherry-picked; subject made conventional)
- bdd5f36 chore(identity): rename the demo variable and format renamed imports
- bc05b18 feat(provider): read omp sessions and pin provider detection (T2)
- a6385f7 perf(tailer): stream initial load and reattach after rename (T3)
- 3584fcc feat(provider): read Codex token_usage_record as a fallback token source (T5)
- 890a70a perf(ui): redraw only when dirty and back off idle polling (T4)
- febfd7f style(test): satisfy clippy needless_borrows_for_generic_args
- 60b58e4 test(provider): parse every local session file without panicking (T6)
- 3d32d33 feat(herdr): resolve path sessions and toggle by pane id without jq (T7)
- 2807a63 build(release): tag-triggered checksummed binaries and download-only installer (T8)
- 17f3422 docs: describe the plugin, install path, and measured baseline (T9)
- 419a8ee chore(issueops): track the cycle plan, gates, and handoff for issue #1
- 00815ad fix(herdr): draw the pane the action was invoked for (found in T10)
- 8cc2ad3 chore(issueops): aim herdr QA at its own panes through the socket
- 96bb59d chore(issueops): record G1-G12 evidence for issue #1
- e4a081c chore(issueops): draft the implementation report for issue #1
- bd7f035 refactor(ui): drop the recording-only autopilot
- 16081ca fix(tailer): keep complete lines longer than 8 MiB in chunked tail reads (review round 1)
- c0a5bf3 fix(herdr): keep the open-pane record until the pane is closed (review round 1)
- cf5f646 docs: say that no release exists before v0.1.0
- 90a927a chore(issueops): record the review round, cautions, and gate evidence

Every intermediate commit from bdd5f36 to 3d32d33 was compiled on its own
(`cargo test --locked --all-targets --no-run` in a clean export): all rc=0.

## Acceptance evidence (gates.md, 12 of 12 met)

| Gate | Result | Evidence |
| --- | --- | --- |
| G1 | `cargo test --locked`: 239 lib + 13 bin + 1 integration (a no-op without AG_REAL_SESSIONS) + 0 doc tests, 0 failed | gates.md G1 |
| G2 | omp root with 12 children auto-detected: 13 agents, 835 tool calls, same as `--provider omp` | gates.md G2, evidence/task-2-omp-inspect.txt |
| G3 | `provider_of_pins_all_four_formats` passes | gates.md G3 |
| G4 | 521 MB Codex rollout: peak RSS 36-55 MB (≤ 100 MiB) | gates.md G4, evidence/task-9-bench.txt |
| G5 | idle CPU 0.23 % finished demo, 0.17 % Running main copy, main_active=1 | gates.md G5 |
| G6 | real_sessions total=3149 failed=0 standalone=123 | gates.md G6, evidence/task-6-real-sessions*.txt |
| G7 | jq in herdr-plugin/herdr/*.sh: 0 | gates.md G7 |
| G8 | tampered SHA256SUMS: `checksum mismatch`, rc=1, no binary | gates.md G8, evidence/task-8-install*.txt |
| G9 | install --ref from GitHub with a file:// release, two presses open then close the graph (`● omp`), uninstall clean | gates.md G9 |
| G10 | `tail_reattaches_after_rename` passes | gates.md G10 |
| G11 | 96 MB Claude transcript: best 0.08 s, RSS 17 MB | gates.md G11 |
| G12 | existing codex goldens unchanged vs b1f31dd, 521 MB rollout tokens: 246129 | gates.md G12 |

T10 (three agents, real herdr 0.9.3): omp, Claude Code and Codex panes each
open the graph on the first press (main card `● omp` / `● claude` / `● codex`)
and close it on the second; an omp pane without a session shows "Herdr has no
session path for this omp pane" with `herdr integration install omp`; the
plugin is uninstalled afterwards (evidence/task-10-herdr-qa*.txt).

## Deviations from the plan

- T2: omp's parent `task` result never carries a terminal child status on this
  machine (all 34 measured were `pending`: background tasks), so the child's own
  `session_exit` (57 of 58 children) remains the usual end signal; the parent
  progress mapping (completed/failed/aborted) is implemented for synchronous
  tasks. `.tombstone` files: none exist, not read.
- T2 QA: `inspect` of an empty or title-only file now exits 1 with "no agent
  activity in this … session yet" (it used to print an empty session).
- T6: 123 files cannot join a session (114 Codex children whose root rollout
  is no longer on disk, 9 Codex Desktop internal subagents without a parent
  link). They are parsed on their own and reported as `standalone`, not failed.
- T7: live QA showed a pane command's context names the focus at pane start,
  not the action's target; toggle now passes the target as AGENTS_GRAPH_PANE
  (and `--target-pane` for split). herdr rejects `--target-pane` for overlays.
- T10: Codex holds a new session at "Hooks need review" because herdr's Codex
  hook changed and is not yet trusted; the QA skipped the prompt (hooks left
  untrusted, the user's decision) and reported the session the way the hook
  does (`herdr pane report-agent-session`). Claude Code and Codex were started
  in directories they already trust.
- Commits were made during implement (not only at commit-push) because T10
  installs the plugin from the pushed branch.

## AI-slop clean

- Dead code: removed `src/autopilot.rs` (476 lines) and its hooks in
  `src/tui.rs`, `src/main.rs`, `src/lib.rs`, and `docs/DESIGN.md`. It drove a
  scripted pointer for upstream's VHS recordings behind `AGENTS_GRAPH_DEMO`;
  the tapes and `assets/build.sh` that used it were removed in the fork.
- Weak artifacts: `herdr::EXIT_UNRESOLVED` is no longer `pub`; the RedrawGate
  doc no longer mentions the demo pointer.
- Unsupported claims: README now says no release exists until `v0.1.0` is
  tagged (the build step has nothing to download before that) and how to point
  it at local archives.
- Measured over `git diff ef49e61 -- src tests benches scripts herdr-plugin/herdr`
  (added non-blank lines split into code and `//`/`#`/`*` comment lines; an
  approximate shell count, not AST-backed): before 2446 code + 672 comment
  added, 438 removed (comment share 0.216); after 2439 code + 667 comment added,
  964 removed (comment share 0.215). Net change in that scope went from +2905 to
  +2365 lines.
- Out of scope, left as is: `cfg!(target_arch = "wasm32")` branches in
  `src/ui/mod.rs` (upstream's portable core still builds without `native`).
- Re-verified after the last pass: cargo fmt --check, cargo clippy --all-targets
  -D warnings, cargo doc -D warnings, cargo check --no-default-features,
  cargo test --locked (239 + 13 + 1), git diff --check, and all twelve gates
  re-run from unchecked (12 met).

## Implementation review

Round 1 (reviewer subagent, lenses reuse/perf/compat/side-effect): revise.

- [major] Chunked tail reads applied the 8 MiB runaway-line cap inside a poll,
  so a complete JSONL line over 8 MiB (the 521 MB rollout has eight Codex tool
  outputs of 8.5-13.3 MB) was dropped on the live path. Fixed: lines are split
  per chunk without the cap and only a line still unfinished when the poll ends
  is capped (`split_lines` + `cap_partial` in src/tailer/bytes.rs). RED first:
  `a_complete_line_longer_than_the_cap_is_kept` failed before the fix (1 of 2
  lines), passes after; `an_unfinished_line_past_the_cap_is_dropped_until_its_newline`
  pins the remaining cap.
- [minor] toggle forgot the open pane before closing it; a failed close left an
  unreachable pane. Fixed: the record is removed only after the close succeeds
  or when the pane is already gone.

## Intent check (intent.md success criteria)

| Criterion | Status |
| --- | --- |
| omp pane: key opens main, task subagents and tool calls; same key closes | met: G9, T10 (`● omp`, a done `task` subagent and tool chips drawn) |
| inspect joins `<Name>.jsonl` children; auto-detect never says Claude | met: G2, G3, omp_demo_conforms |
| Claude/Codex goldens and `cargo test --locked` pass | met: G1, G12 |
| every local session file parses: no panic, no abnormal exit | met: G6 (library sweep, 0 failed) and `agents-graph inspect` over all 3150 files: 3026 exit 0, 124 exit 1 with a message (orphaned Codex children, internal subagents, one file without agent activity), 0 panics or signals |
| 521 MB rollout ≤ 100 MB RSS; 96 MB transcript ≤ 0.3 s | met: G4 (36-55 MB), G11 (0.07-0.08 s) |
| idle CPU ≤ 0.5 % after 30 s without change | met: G5 (0.23 %, 0.17 %) |
| omp rename rewrite is followed | met at unit level: G10 `tail_reattaches_after_rename` (replay-seeded state, rename with a longer file, reset then full re-read) |
| `herdr plugin install` works without jq/cargo/brew and is listed | met with a local `file://` release (G9, T10). The GitHub release does not exist until a `v0.1.0` tag is pushed, which this cycle does not do |

## Performance (this machine, zoetrope 0.2.0 vs agents-graph 0.1.0)

| Measure | zoetrope | agents-graph |
| --- | --- | --- |
| inspect 96 MB Claude transcript: wall / peak RSS | 0.08 s / 110 MB | 0.07-0.08 s / 17 MB |
| inspect 521 MB Codex rollout: wall / peak RSS | 0.17 s / 562 MB | 0.15 s / 36-55 MB |
| idle CPU, finished demo session, 30 s | 3.4-5.0 % | 0.23-0.33 % |
| idle CPU, demo copy with a Running main | — | 0.17 % |
| omp session auto-detect | 1 agent, 0 tool calls (read as Claude) | 13 agents, 835 tool calls |

Sources: evidence/task-9-bench.txt, gates.md G4, G5, G11.

## Side effects

- Remote: branch 1-agents-graph-mvp pushed (includes upstream zoetrope history).
- Local herdr: the plugin was installed and uninstalled several times; panes
  this session created were closed. New Claude Code / Codex session files exist
  under ~/Workspace and ~/Workspace/issueops from the QA prompts.
- Files written by the product: HERDR_PLUGIN_STATE_DIR/open-pane, herdr-plugin/bin/agents-graph (ignored).

## Follow-ups

- A Claude Code session that has not written its transcript yet (no message
  sent) opens to "no session with id"; the graph could wait for it instead.
- Trust herdr's updated Codex hook (`Hooks need review` in Codex) so Codex panes
  report sessions on their own.
- Remove the furkankly.zoetrope plugin and brew zoetrope (separate approval).
