# Issue #9 handoff material

- Lifecycle ID: `io-4b513b64dc69`
- Issue: https://github.com/m16khb-org/herdr-agents-graph/issues/9
- Branch: `9-ui-polish-graph-default`
- Source root: `/Users/m16khb/Workspace/herdr-agents-graph` (stays clean on `main`; never edit it)
- Canonical worktree: `/Users/m16khb/Workspace/herdr-agents-graph.worktrees/9-ui-polish-graph-default`
- Base head: `5825116416ca9d306cc35ac2964901aa27a17dc9` (main). Full head at handoff: the same commit. Diff at handoff: none except this file and the materialized artifacts under `.issueops/issues/9/artifact/`.
- Plan: `.issueops/issues/9/artifact/plan.md`, sha256 `96bd6e45239bba7ff209fe4f419869626c78cd28e4a05f346bd0f62d129cecf8`.

## Purpose, non-goals, endpoint

Purpose: the seven items of issue #9 exactly as the plan defines them. In short:

1. Graph switch keeps every card in view.
2. Distinct transport words and glyphs (`quiet ◦`, `live ◉`).
3. omp subagent description from `session_init.task`: the first line that is not blank, does not start with `#`, and does not end with `:`.
4. Detail facts line truncation and `1 tool` / `N tools`.
5. Fold done subagents in the Now and Graph views, with a single `fold_set` owner and `reconcile_folds`.
6. The default view is Graph.
7. A self-drawn minimap whose scale comes from the nodes only.

Non-goals (from the plan): folding in the Lanes view, Ctrl+wheel, clicking the minimap, filling the detail panel's empty space (removed from the issue as a contract change), `inspect` format changes, new CLI flags.

Approved endpoint for the implementing session: implement → ai-slop-clean → project docs → verify → commit and push to `9-ui-polish-graph-default` → draft PR publication → `execution complete`. The preparing session then does CI (G13), merge, the `v0.3.0` release, real install QA (G14) and cleanup. Do not merge, tag or release.

## Verification so far

- 2026-10-09, macOS arm64, herdr 0.9.3, omp 18.8.6, rustc 1.97.0.
- Plan adversarial review: round 1 revise (6 findings), round 2 revise (4), round 3 revise (1), round 4 pass. Rounds 3 and 4 ran at the escalated reviewer effort. All findings were applied to the plan. The test names in the plan's G10, G10b, G10c and G11 come from those findings, so keep the names.
- Facts measured for the plan (counts only, no session content): 108 real omp `session_init.task` values. Skipping only blank and `#` lines gives 1 distinct line. Also skipping lines that end in `:` gives 95 distinct lines, 0 empty, median length 361. rataflow `apply_layout` ignores `hidden`, so folds remove nodes (`retain_nodes`) instead of hiding them.
- Failures: none. No code has been written or built for this issue yet.

## Remaining work and where results go

- All of the plan's implementation work. Results go in commits on the branch, the gate ledger `.issueops/issues/9/gates.md` (create it from the plan's G1-G12 when entering implement; G13 and G14 belong to the preparing session) and `.issueops/evidence/` (local, ignored).
- The README (screen, key table, a "Changed from 0.2.0" table), the UI sections of `docs/DESIGN.md`, and `.issueops` docs where the change touches them (conventions "UI and SEED tokens", testing view goldens, architecture state/ui rows).
- G6 idle CPU must be measured with the new default Graph view: `scripts/idle-cpu.sh` plus `scripts/make-running-demo.sh`.

## Rules that are easy to miss

- Never send keys or text to herdr panes you did not open. Never restart the herdr server. The preparing session does the real install QA.
- Never copy real session content (`~/.omp`, `~/.claude`, `~/.codex`) into fixtures, tests, docs, commits or PR text. Synthetic tasks only (G11 uses a synthetic preamble, heading and body).
- Provider goldens: only `assets/omp/demo.model.txt` may change (G4). View goldens in `assets/ui/` are regenerated with `UPDATE_GOLDEN=1` and reviewed.
- No automatic relayout on folds. Keep the "relayout only on `r`" rule (`docs/DESIGN.md:307`).
- Commit messages: Conventional Commits with a `Lore:` body (`.issueops/COMMIT_POLICY.md`). PRs are merged with a merge commit by the preparing session.

## Lifecycle state at handoff

`plan.handoff` is complete. The execution is direct, generation 1, and the holder (the preparing omp session) releases before the new session opens. The session-choice decision is recorded in `status.decisions` (title "실행 방식 선택", created 2026-10-09T08:13:50.864018Z).

## Resume and read-only check commands

```bash
cd /Users/m16khb/Workspace/herdr-agents-graph.worktrees/9-ui-polish-graph-default
issueops execution whoami --json
issueops next --id io-4b513b64dc69 --json        # follow its exact next_command chain (direct released → replace preview → claim)
issueops execution status --id io-4b513b64dc69 --json
issueops status --id io-4b513b64dc69 --json
git rev-parse HEAD                                 # expect 5825116416ca9d306cc35ac2964901aa27a17dc9 until you commit
shasum -a 256 .issueops/issues/9/artifact/plan.md  # expect 96bd6e45…cecf8
```
