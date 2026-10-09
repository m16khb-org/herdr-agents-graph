---
name: 2026-10-09-herdr-focuses-a-new-plugin-split-even-when-focus-is-false
description: Caution record for a solved false case or recurring risk.
---

# herdr focuses a new plugin split even when focus is false

- Date: 2026-10-09
- Kind: `caution`
- Source: project-bootstrap enrichment 2026-10-09
- Summary: During release QA, opening the graph as a split next to a target pane moved the user's focus and workspace to the new pane although plugin.pane.open defaults to focus false.
- Context: QA on 2026-10-09 drove agents-graph herdr toggle split with HERDR_PLUGIN_CONTEXT_JSON naming a QA pane; herdr pane list then showed only the new graph pane focused, taking focus away from the pane the user was on in another workspace.
- Resolution: Record the user's focused pane before QA and restore it with herdr agent focus <pane> afterwards. Do not rely on focus false to keep the user's view; prefer QA in a workspace the user is not looking at and say in the report that focus moved.
- Evidence:
  - herdr pane list before/after toggle (focused wA:p1 -> wQ:p1C)
  - herdr agent focus wA:p1
  - src/herdr.rs toggle
