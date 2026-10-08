# Issue #1 implementation report (draft)

Status: in progress (implement done; ai-slop-clean, docs, verify, PR pending)
Lifecycle: io-4b513b64dc69
Mode/host/model: direct / omp / anthropic/claude-opus-5-5 (high)
Worktree/branch: /Users/m16khb/Workspace/herdr-agents-graph.worktrees/1-agents-graph-mvp / 1-agents-graph-mvp
Lease generation: 2 (taken over from generation 1 via replace + claim after the handoff)
Issue/plan digests: plan.md 474d3f0b6c8c6f8e0c1e05ea3103cd0215f2f7b7c7dc615dda86916a7037e4d5 and handoff.md 08b7fe92… matched before claim

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

Every intermediate commit from bdd5f36 to 3d32d33 was compiled on its own
(`cargo test --locked --all-targets --no-run` in a clean export): all rc=0.

## Acceptance evidence (gates.md, 12 of 12 met)

| Gate | Result | Evidence |
| --- | --- | --- |
| G1 | `cargo test --locked`: 237 lib + 13 bin + 1 integration (a no-op without AG_REAL_SESSIONS) + 0 doc tests, 0 failed | gates.md G1 |
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
