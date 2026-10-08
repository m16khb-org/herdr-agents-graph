---
name: 2026-10-09-a-herdr-plugin-pane-s-own-context-names-the-focus-at-pane-st
description: Caution record for a solved false case or recurring risk.
---

# A herdr plugin pane's own context names the focus at pane start, not the action's target

- Date: 2026-10-09
- Kind: `caution`
- Source: issue #1 T10 live QA
- Summary: agents-graph herdr toggle must hand the invoked pane to the graph pane itself.
- Context: An action receives focused_pane_id for the pane it was invoked for. When it runs herdr plugin pane open, the new pane command gets its own HERDR_PLUGIN_CONTEXT_JSON whose focused_pane_id is whatever is focused when that pane starts. Invoked for an agent-less shell pane while an omp pane had focus, the graph drew the omp session. herdr 0.9.3 also rejects --target-pane for overlay and popup placements (invalid_params: overlay and popup plugin panes target the active pane).
- Resolution: toggle passes the action's focused_pane_id to the graph pane as AGENTS_GRAPH_PANE (and as --target-pane only for split); herdr resolve reads AGENTS_GRAPH_PANE before the context. Test: the_named_pane_outranks_the_pane_commands_own_focus (src/herdr.rs).
