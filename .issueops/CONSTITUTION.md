---
name: CONSTITUTION.md
description: Instruction priority, safety, and accuracy principles; read when rules conflict or an action is risky.
---

# Constitution

## SessionStart contract

Read this document at session start. Follow the general LLM coding behavior guidelines at the top of [AGENTS.md](../AGENTS.md); this document adds the invariants that are specific to herdr-agents-graph. Treat it as the baseline principle document for MCP routing.

## Source of truth

1. Latest explicit user/system instructions
2. Current repo AGENTS.md or a nearer nested AGENTS.md
3. `.issueops/*.md`
4. Current files and command output

Repo-authored design docs under [docs/](../docs/) (`ARCHITECTURE.md`, `DISCOVERY.md`, `HERDR-PLUGIN.md`, `DESIGN.md`) carry detail that `.issueops` links to rather than repeats. They were inherited from the zoetrope fork; when one disagrees with current code, the code wins and the doc is the thing to fix.

## Principles

- **Read-only on agent sessions.** The program and its tests only read Claude Code, Codex, and omp transcripts. Nothing writes to `~/.claude`, `~/.codex`, or `~/.omp`.
- **No session content leaves the machine or lands in the repo.** Fixtures under `assets/` are anonymised (for example `assets/omp/2026-01-01T00-00-00-000Z_00000000-...jsonl`). Docs, test output, and issue/PR text may describe record shapes and field names, never real conversation text, prompts, personal paths, or tokens. `tests/real_sessions.rs` prints paths and error kinds only.
- **Never drive other herdr panes.** QA and automation may create, read, and close panes this work opened; they must not send keys or text to panes a person or another agent owns, restart the herdr server, or edit the user's herdr config except through the plugin's own `setup-keys`/`remove-keys` actions on request.
- **Goldens are contracts.** `assets/<provider>/*.model.txt` and `*.timeline.txt` change only when the change is intended and reviewed (`UPDATE_GOLDEN=1`); a refactor that rewrites them is a regression until shown otherwise.
- **Measured performance is a contract.** The release targets set in issue #1 hold: 521 MB Codex rollout `inspect` max RSS ≤ 100 MB, finished-session idle CPU ≤ 0.5 %, every local session file parsed without panic. Evidence and commands: [.issueops/issues/1/gates.md](issues/1/gates.md).
- **Third-party attribution stays intact.** The fork keeps zoetrope's MIT `LICENSE` and the attribution in `NOTICE`. Vendored design tokens (issue #6, SEED Design, Apache-2.0) keep their `LICENSE` and `NOTICE`, and Daangn brand resources (logo, name, characters, carrot accent) are not used.
- **Secrets never appear** in docs, logs, fixtures, or CLI output.

## Complexity, readability, and consistency

**Prefer average or amortized O(1) data access and repeated hot-path operations when practical.**

- When alternatives do not materially harm performance or optimization value, prefer readability, established project conventions, and consistent APIs and structure.
- Do not add unnecessary caches, indexes, or duplicated state merely to claim O(1). Justify optimization with measured bottlenecks and time/space complexity evidence.
- Safety and correctness remain higher priorities when O(1) goals conflict.
- Hot paths in this repo are the line-by-line transcript load, the 200 ms tail poll, and the 16 ms TUI tick; redraw only when the frame would change (see [ARCHITECTURE.md](ARCHITECTURE.md)).
