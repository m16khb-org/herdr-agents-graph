---
name: 2026-10-09-folded-agents-leave-the-rataflow-graph-and-the-minimap-is-ou
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Folded agents leave the rataflow graph, and the minimap is our own

- Date: 2026-10-09
- Kind: `adr`
- Source: cli
- Summary: Issue #9. Done subagents (two or more under one parent, done for good: terminal or a settled replay) fold into one card and one Now row. Folded nodes are removed with retain_nodes and their positions parked in App.parked_positions, not hidden with set_node_hidden: rataflow 0.1.0's Sugiyama apply_layout ignores hidden and keeps their slots. One owner, state::view::fold_set, decides the folds for every view; the selection takes no part, and App::reconcile_folds re-syncs the graph once per loop turn when they change. The minimap is drawn by ui::views::minimap from Flow::nodes, Node::bounds, the viewport and canvas_size, with its scale from the node bounds only; rataflow's MiniMap fits the viewport into its scale (calculate_bounds), so zoom and pan rescaled it. Patching rataflow was rejected: it is a crates.io dependency, Cargo.toml forbids path overrides, and an upstream release date is unknown.
