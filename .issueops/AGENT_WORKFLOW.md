---
name: AGENT_WORKFLOW.md
description: Agent start, execution, verification, and completion flow; read when starting or handing off a task.
---

# Agent Workflow

## Start

1. Read [AGENTS.md](../AGENTS.md) first.
2. At session start, treat [CONSTITUTION.md](CONSTITUTION.md) as the baseline principle document.
3. If MCP is available, send the current task to `project_docs_route` and read only the documents it returns.
4. Verify inferred doc claims against current files and command output.

## MCP usage rule

- At session start the installed SessionStart hook (`issueops hook session-start`) injects the project-doc catalog. It is context for judgment, not an auto-execution command.
- Use MCP when the task needs current state, repo-specific doc routing, policy decisions, state checkpoints, or durable records that the model should not rely on from memory. The shared issueops MCP server needs an authority file from `issueops mcp authorize --workspace-root <abs path> ...` for workspace tools.
- Do not use MCP for simple reasoning or summarizing already opened files.
- Narrowly use route/read/revise/append tools that match the task.
- Do not trust tool output blindly; check paths, exists flags, warnings, and verification evidence.

## How work has run in this repo

Feature work goes through IssueOps cycles (issue #1 → PR #2; issue #6 in progress). Observed shape, from `.issueops/issues/1/` and the issue/PR history:

- One GitHub issue per cycle, branch `<issue>-<slug>` from `main`, and a canonical worktree at `../herdr-agents-graph.worktrees/<branch>`. The source checkout stays clean while a cycle runs; edits happen only in the worktree.
- Plans pass an independent adversarial review before implementation; plan, gates, and report live under `.issueops/issues/<n>/`.
- When the preparing session runs inside herdr, implementation is handed to a new omp session in a herdr pane opened on the worktree.
- Draft PR, CI (`.github/workflows/ci.yml`) green on every OS in the matrix, merge commit into `main`, then a `v*` tag for a release (`.github/workflows/cd.yml`), then a real `herdr plugin install` QA.
- Unrelated docs or repo work during an active cycle uses its own branch and worktree, so it never dirties the source checkout the cycle checks.

## Work

Use the Simplicity First and Surgical Changes principles from AGENTS.md, plus:

- Do not overwrite existing user changes.
- Add dependencies, deploy, tag a release, or perform destructive actions only with explicit instruction or strong evidence.
- Follow the invariants in [CONSTITUTION.md](CONSTITUTION.md): read-only on agent sessions, no session content in the repo, never drive another herdr pane.
- If docs diverge from current code or user consensus, use `project_docs_read` for the current SHA and `project_docs_revise` to change one document at a time.
- When a problem occurred and was resolved, record it with `project_docs_append(kind=caution)` as a dated file under [cautions/](cautions/).
- When a structural decision or rejected alternative matters, record it with `project_docs_append(kind=adr)` as a dated file under [adr/](adr/).

## Verify

Use the Goal-Driven Execution principle from AGENTS.md, plus:

- Before writing or modifying tests, read [TESTING.md](TESTING.md).
- Provider or fact changes: run the golden harness and confirm `assets/**/*.txt` did not change unless intended.
- TUI or loop changes: re-measure idle CPU with `scripts/idle-cpu.sh` (see [TESTING.md](TESTING.md)).
- herdr bridge changes (`herdr-plugin/`, `src/herdr.rs`): exercise the real plugin path on a pane this work opened.
- Completion reports include test/build/static-check results and the reason for any skipped verification.

## Finish

- If a commit is needed, follow [COMMIT_POLICY.md](COMMIT_POLICY.md).
- Record resolved false cases or structural decisions with `project_docs_append` when useful.
