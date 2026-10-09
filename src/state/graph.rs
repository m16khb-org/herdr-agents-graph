//! Incremental projection of [`SessionModel`] onto a rataflow `Flow`.
//!
//! Never rebuilds: per agent, either mutate the existing node content in place
//! via `node_content_mut`, or `add_node` + `add_edge` (duplicate-id `Err` is an
//! idempotent no-op). Nodes are added before their edges. A structural change
//! (node/edge added or removed) marks layout dirty; at sync end we run
//! Sugiyama. Selection survives because node ids are stable and we never
//! clear-and-re-add. Folded agents leave the canvas for one card per fold,
//! and come back where they were.

use std::collections::HashMap;

use rataflow::types::Position;
use rataflow::{Edge, Flow, Handle, HandlePosition, Node, Reconnectable, StepEdge, Sugiyama};

use super::session::{AgentInfo, AgentKind, AgentStatus, SessionModel};
use super::view::{FoldSet, fold_card_id, fold_of_card};

/// Fixed card dimensions for main / workflow nodes (world units).
pub const MAIN_NODE_DIMS: (f64, f64) = (30.0, 7.0);
/// Fixed card dimensions for subagent nodes (world units).
pub const SUB_NODE_DIMS: (f64, f64) = (26.0, 6.0);

/// What an agent card shows, mirrored from the model on every sync.
///
/// Plain data: the graph layer reads the fields back and mutates them in
/// place, and the ui layer renders it (`NodeContent` is implemented in
/// `ui::views::graph`), so this module never depends on how a card looks.
#[derive(Debug, Clone)]
pub struct AgentNode {
    /// Title line — the agent type, which for the main agent is the provider's
    /// own name (`claude`, `codex`).
    pub title: String,
    /// Truncated description shown under the title.
    pub description: Option<String>,
    pub status: AgentStatus,
    /// Number of tool calls.
    pub tool_count: usize,
    /// Name of the most recent tool call, if any.
    pub last_tool: Option<String>,
    pub output_tokens: u64,
    /// Interactive agents (main, forks) word `Running` as "active": we know
    /// there are recent entries, not that a task is executing.
    pub interactive: bool,
}

/// A step-routed parent edge; `running` mirrors the target agent's status so
/// the renderer can mark liveness on the structure itself.
#[derive(Debug, Default, Clone)]
pub struct AgentEdge {
    pub(crate) inner: StepEdge,
    pub running: bool,
}

/// The concrete `Flow` type the app uses: agent-card nodes, step-routed parent
/// edges (no labels — liveness reads from color alone).
pub type AgentFlow = Flow<AgentNode, AgentEdge>;

/// Build an empty, fully-configured `Flow` for the app.
///
/// Config: `with_deselect_on_pane_click(false)`, `deselect_on_drag = false`
/// (detail panel persists), `with_min_zoom(0.1)` (Sugiyama trees outgrow the
/// default fit-view limit). Hidden source/target handles for a clean look.
/// The palette is the ui's: it replaces `flow.theme` with the SEED palette
/// before every canvas render.
pub fn new_flow() -> AgentFlow {
    let mut flow = Flow::new()
        .with_deselect_on_pane_click(false)
        // We drive the camera on selection ourselves (a center-glide via
        // `pending_center`), so suppress the library's instant ensure-visible pan
        // — otherwise the two stack into a jump-then-glide on off-screen nodes.
        .with_selection_reveal(rataflow::SelectionReveal::None)
        .with_min_zoom(0.1);
    flow.deselect_on_drag = false;
    flow
}

/// Fixed card dimensions for a node kind.
fn node_dims(kind: AgentKind) -> (f64, f64) {
    match kind {
        AgentKind::Main | AgentKind::Group => MAIN_NODE_DIMS,
        AgentKind::Subagent => SUB_NODE_DIMS,
    }
}

/// Whether a node's content already mirrors the agent — allocation-free
/// comparison so unchanged agents skip [`build_content`]'s String clones on
/// every sync (the steady state for almost all agents on almost all ticks).
fn content_matches(info: &AgentInfo, node: &AgentNode) -> bool {
    node.title == info.display_name()
        && node.description.as_deref() == info.description.as_deref()
        && node.status == info.status
        && node.tool_count == info.tool_calls.len()
        && node.last_tool.as_deref() == info.last_tool()
        && node.output_tokens == info.output_tokens
        && node.interactive == info.is_interactive()
}

/// Build the [`AgentNode`] content mirrored from an [`AgentInfo`].
fn build_content(info: &AgentInfo) -> AgentNode {
    AgentNode {
        title: info.display_name().to_string(),
        description: info.description.clone(),
        status: info.status,
        tool_count: info.tool_calls.len(),
        last_tool: info.last_tool().map(str::to_string),
        output_tokens: info.output_tokens,
        interactive: info.is_interactive(),
    }
}

/// The card a fold shows: how many are done and who, and their calls and
/// tokens summed.
fn fold_content(model: &SessionModel, members: &[String]) -> AgentNode {
    let agents = || members.iter().filter_map(|id| model.agent(id));
    let names: Vec<&str> = agents().map(AgentInfo::display_name).collect();
    AgentNode {
        title: format!("{} done", members.len()),
        description: Some(names.join(", ")),
        status: AgentStatus::Done,
        tool_count: agents().map(|a| a.tool_calls.len()).sum(),
        last_tool: None,
        output_tokens: agents().fold(0u64, |sum, a| sum.saturating_add(a.output_tokens)),
        interactive: false,
    }
}

/// A read-only card: selectable (detail panel) and draggable (manual
/// arrangement), but never deletable and never a connection source.
/// Enforced at the DTO level, not just the key whitelist, so no input path
/// can mutate the graph.
fn card(id: String, pos: (f64, f64), dims: (f64, f64), content: AgentNode) -> Node<AgentNode> {
    Node::new(id, pos, dims, content)
        .with_deletable(false)
        .with_connectable(false)
        .with_handles(vec![
            Handle::source(HandlePosition::Bottom).with_hidden(true),
            Handle::target(HandlePosition::Top).with_hidden(true),
        ])
}

/// Horizontal gap between locally-placed siblings (world units).
const LOCAL_H_GAP: f64 = 4.0;
/// Vertical gap below a parent for locally-placed children (world units).
const LOCAL_V_GAP: f64 = 5.0;

/// Local placement for a new `w`-wide card of agent `id`: below its parent,
/// fanned past the siblings spawned before it; `(0, 0)` when the parent is not
/// on the canvas. The sibling index is computed only for the rare new node, so
/// the no-new-nodes steady state skips it entirely.
fn local_position(flow: &AgentFlow, model: &SessionModel, id: &str, w: f64) -> (f64, f64) {
    let Some(parent) = model.agent(id).and_then(|a| a.parent.as_deref()) else {
        return (0.0, 0.0);
    };
    let Some(node) = flow.node(parent) else {
        return (0.0, 0.0);
    };
    let siblings = model
        .spawn_order
        .iter()
        .take_while(|x| x.as_str() != id)
        .filter(|x| model.agent(x).and_then(|a| a.parent.as_deref()) == Some(parent))
        .count();
    (
        node.position.x + siblings as f64 * (w + LOCAL_H_GAP),
        node.position.y + node.height + LOCAL_V_GAP,
    )
}

/// Incrementally sync `flow` to `model`, with `folds` folded away.
///
/// For each agent in spawn order: mutate the existing node content in place, or
/// add the node (then its parent edge). Updates edge `animated` from target
/// status. New nodes get LOCAL placement (below their parent, offset past
/// siblings) so they land somewhere sensible even without a relayout.
///
/// A folded agent's node leaves the canvas, its position kept in `parked`,
/// and each fold gets one card in its first member's place. An agent that
/// comes back out of a fold returns to its parked position, so folding and
/// seeking never undo where the user dragged a card.
///
/// When `relayout` is true, any structural change ends with a full
/// `Sugiyama::vertical()` pass (which overwrites the local placements). When
/// false — Manual camera: the user owns the view — nothing existing moves;
/// the caller tracks dirtiness and relayouts when the camera re-engages.
/// Returns `true` if structure changed.
pub fn sync(
    flow: &mut AgentFlow,
    model: &SessionModel,
    folds: &FoldSet,
    parked: &mut HashMap<String, Position>,
    relayout: bool,
) -> bool {
    let mut structural = false;

    // Take folded agents and stale fold cards off the canvas first, parking
    // the agents' positions: a new fold card takes its first member's place.
    let leaving = |id: &str| match fold_of_card(id) {
        Some(parent) => !folds.by_parent.contains_key(parent),
        None => folds.member_of.contains_key(id),
    };
    if flow.nodes().any(|n| leaving(&n.id)) {
        flow.retain_nodes(|n| {
            if !leaving(&n.id) {
                return true;
            }
            if fold_of_card(&n.id).is_none() {
                parked.insert(n.id.clone(), n.position);
            }
            false
        });
        structural = true;
    }

    // First pass: nodes (must exist before their edges).
    for id in &model.spawn_order {
        if folds.member_of.contains_key(id) {
            continue;
        }
        let Some(info) = model.agent(id) else {
            continue;
        };
        if let Some(existing) = flow.node_content_mut(id) {
            // Steady state: only rebuild (String clones) when something
            // visible changed — the per-second status tick and per-batch
            // syncs walk every agent, and most are unchanged.
            if !content_matches(info, existing) {
                *existing = build_content(info);
            }
        } else {
            let (w, h) = node_dims(info.kind);
            // Back where it was before a fold took it; else local placement:
            // below the parent, fanned past prior siblings. Overwritten by
            // Sugiyama when `relayout` runs; kept verbatim in Manual so
            // existing nodes never move underneath the user.
            let pos = match parked.remove(id) {
                Some(p) => (p.x, p.y),
                None => local_position(flow, model, id, w),
            };
            // Duplicate-id is an idempotent no-op; a genuine add is structural.
            if flow
                .add_node(card(id.clone(), pos, (w, h), build_content(info)))
                .is_ok()
            {
                structural = true;
            }
        }
    }
    for (parent, members) in &folds.by_parent {
        let id = fold_card_id(parent);
        let content = fold_content(model, members);
        if let Some(existing) = flow.node_content_mut(&id) {
            *existing = content;
        } else {
            // The first member's parked place; a fold formed before any
            // member reached the canvas (a live session's first read) takes
            // the place the first member would have had.
            let pos = members.first().map_or((0.0, 0.0), |m| {
                parked.get(m).map_or_else(
                    || local_position(flow, model, m, SUB_NODE_DIMS.0),
                    |p| (p.x, p.y),
                )
            });
            if flow.add_node(card(id, pos, SUB_NODE_DIMS, content)).is_ok() {
                structural = true;
            }
        }
    }

    // Second pass: edges from each shown agent, and each fold card, to its
    // parent.
    for id in &model.spawn_order {
        if folds.member_of.contains_key(id) {
            continue;
        }
        let Some(info) = model.agent(id) else {
            continue;
        };
        let Some(parent) = &info.parent else {
            continue;
        };
        structural |= upsert_edge(flow, id, parent, info.status == AgentStatus::Running);
    }
    for parent in folds.by_parent.keys() {
        structural |= upsert_edge(flow, &fold_card_id(parent), parent, false);
    }

    if structural && relayout {
        self::relayout(flow);
    }
    structural
}

/// Add `child`'s parent edge, or refresh its animation when it exists.
/// Returns whether an edge was added.
fn upsert_edge(flow: &mut AgentFlow, child: &str, parent: &str, animated: bool) -> bool {
    let edge_id = edge_id(child);
    // Edge already present (the steady state on every sync): just refresh
    // animation — probing via `edge_content_mut` first avoids building a
    // throwaway Edge (three String clones) per agent per sync only for
    // `add_edge` to reject it as a duplicate. Edges carry no selectable
    // meaning here (no edge panel), and a stray edge click would pin
    // Follow mode while closing the node panel — a dead state. Fully inert:
    // not selectable, deletable, or reconnectable. Liveness shows as the
    // running color + marching ants, NOT a label — the current tool already
    // shows in the child's chips and detail panel.
    if let Some(content) = flow.edge_content_mut(&edge_id) {
        content.running = animated;
        flow.set_edge_animated(&edge_id, animated);
        return false;
    }
    let edge = Edge::new(edge_id.clone(), parent.to_string(), child.to_string())
        .with_animated(animated)
        .with_selectable(false)
        .with_deletable(false)
        .with_reconnectable(Reconnectable::None);
    let added = flow.add_edge(edge).is_ok();
    if let Some(content) = flow.edge_content_mut(&edge_id) {
        content.running = animated;
    }
    added
}

/// Stable id for the (single) parent edge of `child`.
///
/// Keyed by the child alone: every agent has exactly one parent edge, so the
/// id must never change once created. Keying on `spawned_by` or the parent
/// would orphan a stale edge if either field were filled in after the edge
/// existed (latent today, armed by any future meta re-emission). An edge
/// leaves only with its child's node.
fn edge_id(child: &str) -> String {
    format!("e-{child}")
}

/// Remove several agents from the canvas in ONE pass.
///
/// Bulk, not a loop of single removals: `Flow::remove_node` costs O(nodes plus
/// edges) every time — it shifts every lookup index and rebuilds the edge
/// lookup per call — so removing k of them is quadratic. `retain_nodes` makes a
/// single pass over each vec and drops the connected edges itself.
pub fn remove_agents(flow: &mut AgentFlow, agents: &[String]) {
    if agents.is_empty() {
        return;
    }
    let doomed: std::collections::HashSet<&str> = agents.iter().map(String::as_str).collect();
    flow.retain_nodes(|n| !doomed.contains(n.id.as_str()));
}

/// Apply the Sugiyama vertical layout to `flow`.
///
/// Split out so it can be called explicitly and unit-tested independently of
/// the per-agent diffing in [`sync`].
pub fn relayout(flow: &mut AgentFlow) {
    flow.apply_layout(Sugiyama::vertical());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::claude::wire::SubagentMeta;

    /// `sync` with nothing folded.
    fn sync_all(flow: &mut AgentFlow, model: &SessionModel, relayout: bool) -> bool {
        sync(
            flow,
            model,
            &FoldSet::default(),
            &mut HashMap::new(),
            relayout,
        )
    }

    /// A model with main + one direct subagent (running).
    fn model_with_subagent() -> SessionModel {
        let mut m = SessionModel::new("s1".into());
        // The root's name is the provider's to state.
        m.apply_fact(&crate::fact::Fact {
            agent: Some(super::super::session::MAIN_ID.to_string()),
            ts: None,
            kind: crate::fact::FactKind::Agent {
                kind: AgentKind::Main,
                parent: None,
                agent_type: Some("claude".into()),
                description: None,
                spawned_by: None,
                interactive: true,
            },
        });
        let meta = SubagentMeta {
            agent_type: Some("guide".into()),
            description: Some("research".into()),
            tool_use_id: Some("ag1".into()),
            stopped_by_user: None,
        };
        m.apply_meta("abc123", None, &meta);
        m
    }

    #[test]
    fn sync_creates_nodes_and_edge() {
        let model = model_with_subagent();
        let mut flow = new_flow();
        let structural = sync_all(&mut flow, &model, true);
        assert!(structural);
        assert!(flow.node_content_mut("main").is_some());
        assert!(flow.node_content_mut("abc123").is_some());
        // One edge main -> abc123.
        assert_eq!(flow.edges().len(), 1);
    }

    #[test]
    fn sync_idempotent() {
        let model = model_with_subagent();
        let mut flow = new_flow();
        let first = sync_all(&mut flow, &model, true);
        assert!(first);
        let node_count = flow.nodes().count();
        let edge_count = flow.edges().len();

        // Applying the same model again adds nothing structural.
        let second = sync_all(&mut flow, &model, true);
        assert!(!second);
        assert_eq!(flow.nodes().count(), node_count);
        assert_eq!(flow.edges().len(), edge_count);
    }

    #[test]
    fn sync_preserves_selection() {
        let model = model_with_subagent();
        let mut flow = new_flow();
        sync_all(&mut flow, &model, true);
        flow.select_node("abc123");
        assert_eq!(
            flow.selected_nodes().next().map(|n| n.id.clone()),
            Some("abc123".to_string())
        );

        // Re-sync after a non-structural change (e.g. a tool call added).
        let mut model2 = model;
        if let Some(a) = model2.agents.get_mut("abc123") {
            a.output_tokens += 100;
        }
        sync_all(&mut flow, &model2, true);
        assert_eq!(
            flow.selected_nodes().next().map(|n| n.id.clone()),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn edge_animation_follows_status() {
        let mut model = model_with_subagent();
        let mut flow = new_flow();
        sync_all(&mut flow, &model, true);
        // Running subagent -> animated edge.
        let edge_id = edge_id("abc123");
        let animated = flow
            .edges()
            .iter()
            .find(|e| e.id == edge_id)
            .map(|e| e.animated);
        assert_eq!(animated, Some(true));

        // The edge content mirrors running-ness (drives the distinct color).
        assert!(flow.edge_content_mut(&edge_id).unwrap().running);

        // Mark done, re-sync -> no longer animated, color back to default.
        if let Some(a) = model.agents.get_mut("abc123") {
            a.status = AgentStatus::Done;
        }
        sync_all(&mut flow, &model, true);
        let animated = flow
            .edges()
            .iter()
            .find(|e| e.id == edge_id)
            .map(|e| e.animated);
        assert_eq!(animated, Some(false));
        assert!(!flow.edge_content_mut(&edge_id).unwrap().running);
    }

    #[test]
    fn graph_is_structurally_read_only() {
        use rataflow::Reconnectable;

        let model = model_with_subagent();
        let mut flow = new_flow();
        sync_all(&mut flow, &model, true);

        for node in flow.nodes() {
            assert!(node.selectable, "nodes stay selectable (detail panel)");
            assert!(node.draggable, "nodes stay draggable (manual arranging)");
            assert!(!node.deletable, "nodes must not be deletable");
            assert!(!node.connectable, "nodes must not start connections");
        }
        for edge in flow.edges() {
            assert!(!edge.selectable, "edges carry no selectable meaning");
            assert!(!edge.deletable);
            assert_eq!(edge.reconnectable, Reconnectable::None);
        }
    }

    #[test]
    fn manual_mode_local_placement_moves_nothing_existing() {
        let mut model = model_with_subagent();
        let mut flow = new_flow();
        // Initial layout (camera engaged).
        sync_all(&mut flow, &model, true);
        let main_pos = flow.node("main").unwrap().position;
        let first_sub = flow.node("abc123").unwrap().position;

        // Camera now Manual: a second subagent arrives, relayout deferred.
        let meta2 = SubagentMeta {
            agent_type: Some("guide".into()),
            description: None,
            tool_use_id: Some("ag2".into()),
            stopped_by_user: None,
        };
        model.apply_meta("def456", None, &meta2);
        let structural = sync_all(&mut flow, &model, false);
        assert!(structural);

        // Nothing existing moved...
        assert_eq!(flow.node("main").unwrap().position, main_pos);
        assert_eq!(flow.node("abc123").unwrap().position, first_sub);
        // ...and the newcomer landed below its parent, not at the origin.
        let new_pos = flow.node("def456").unwrap().position;
        assert!(new_pos.y > main_pos.y, "child placed below parent");
        assert_ne!((new_pos.x, new_pos.y), (0.0, 0.0));
    }

    #[test]
    fn cards_render_into_buffer() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        use ratatui::widgets::Widget;

        let model = model_with_subagent();
        let mut flow = new_flow();
        sync_all(&mut flow, &model, true);
        flow.request_fit_view();

        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        (&mut flow).render(area, &mut buf);

        let mut text = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        assert!(
            text.contains("claude"),
            "main card title missing from render:\n{text}"
        );
        assert!(
            text.contains("guide"),
            "subagent card title missing from render:\n{text}"
        );
    }
}
