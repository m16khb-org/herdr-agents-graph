---
name: 2026-10-12-auto-arrange-is-on-by-default-and-users-can-collapse-any-sub
description: Accepted decision record with rationale, alternatives, and consequences.
---

# Auto-arrange is on by default and users can collapse any subtree

- Date: 2026-10-12
- Kind: `adr`
- Source: cli
- Summary: The graph re-runs Sugiyama on every structural change while App.auto_layout is on (default; a toggles it, a node drag turns it off), and x/X collapse any agent's subtree through the same FoldSet that owns the done folds.
- Context: Users asked for the graph to arrange itself and for subtrees to fold and unfold. Layout had been strictly user-driven (r only) because a relayout on every new node read as jumpy in a growing live session; folding covered only two or more done siblings.
- Decision: App.auto_layout (default true) is passed to graph::sync as relayout; a toggles it and tidies at once when turned on; FlowEvent::NodeDragged turns it off so a hand-placed card stays put; r still relayouts once. User collapses live in App.collapsed; state::view::fold_set adds FoldSet.hidden_by (descendant -> outermost collapsed agent) and FoldSet.collapsed (hidden count), so rows, graph sync, selection normalisation, Follow and center_node read one owner. x toggles the selected subtree (unfolds a selected done fold, refolds a childless agent into its done fold), X collapses every parent or clears all collapses; Lanes ignore both.
- Consequences: Graph goldens are laid out by Sugiyama. A live session's cards move whenever an agent arrives unless the user turns auto-arrange off; parked positions only matter with it off. Collapses are cleared on a session switch; auto_layout is not.
