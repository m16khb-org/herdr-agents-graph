//! What the screen is showing, as state: the active view, the selection
//! every view shares, the done subagents folded away, transient notices, and
//! where the last frame put the things a mouse can hit.
//!
//! Plain data with no rendering in it. The ui reads it to draw and writes the
//! hit map back; the input handler reads the hit map to route clicks.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use web_time::Instant;

use super::session::{AgentKind, AgentStatus, SessionModel};

/// The three ways of looking at a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum View {
    /// Who is doing what right now: the agent tree with intents (default).
    #[default]
    Now,
    /// Each agent as a lane of tool calls along time.
    Lanes,
    /// The spawn graph on a pannable canvas.
    Graph,
}

impl View {
    /// Tab order.
    pub const ALL: [View; 3] = [View::Now, View::Lanes, View::Graph];

    /// The tab label.
    pub const fn label(self) -> &'static str {
        match self {
            View::Now => "Now",
            View::Lanes => "Lanes",
            View::Graph => "Graph",
        }
    }

    /// The view `step` tabs away, wrapping.
    pub fn cycle(self, step: isize) -> View {
        let at = View::ALL.iter().position(|v| *v == self).unwrap_or(0) as isize;
        let n = View::ALL.len() as isize;
        View::ALL[(at + step).rem_euclid(n) as usize]
    }
}

/// The agent the user is looking at, shared by every view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Selected agent id. The graph's own selection mirrors it.
    pub agent: Option<String>,
    /// The Now view shows the selected row's most recent tool calls.
    pub expanded: bool,
    /// The detail panel fills the body.
    pub detail: bool,
    /// The selected fold, by the id of the parent it hangs under. Never
    /// `Some` together with `agent`: a fold is selected instead of an agent.
    pub folded: Option<String>,
    /// Detail tool-list scroll offset. The renderer clamps it to the real
    /// maximum and writes it back, so it always names the top line shown.
    pub scroll: u16,
    /// Whether the detail list tails the newest call. Scrolling up detaches
    /// it; scrolling back to the bottom re-attaches.
    pub follow: bool,
}

impl Default for Selection {
    fn default() -> Self {
        Selection {
            agent: None,
            expanded: false,
            detail: false,
            folded: None,
            scroll: 0,
            follow: true,
        }
    }
}

impl Selection {
    /// Point the detail list back at its newest call.
    pub fn reset_scroll(&mut self) {
        self.scroll = 0;
        self.follow = true;
    }
}

/// A short notice at the bottom of the screen ("Back to live").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snack {
    pub message: String,
    pub until: Instant,
}

/// Agents whose status changed a moment ago, drawn emphasised until `until`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flash {
    pub ids: Vec<String>,
    pub until: Instant,
}

/// The lane view's time axis as last drawn: the body it covers and the time
/// at the left edge of every column, so a click or an arrow press can move
/// the cursor by whole columns even where idle gaps are folded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneAxis {
    pub area: Rect,
    pub columns: Vec<DateTime<Utc>>,
    /// Time one column spans on the unfolded axis.
    pub ms_per_col: i64,
}

impl LaneAxis {
    /// The column whose span holds `t` (clamped to the axis).
    pub fn column_of(&self, t: DateTime<Utc>) -> usize {
        self.columns.partition_point(|c| *c <= t).saturating_sub(1)
    }
}

/// Where the last frame put the things a mouse can hit. Rewritten by every
/// draw, read by the input handler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HitMap {
    pub tabs: Vec<(Rect, View)>,
    /// Clickable agent rows (Now and Lanes).
    pub rows: Vec<(Rect, String)>,
    pub lane_axis: Option<LaneAxis>,
    /// The graph canvas.
    pub canvas: Option<Rect>,
}

impl HitMap {
    /// The tab under a cell, if any.
    pub fn tab_at(&self, x: u16, y: u16) -> Option<View> {
        self.tabs
            .iter()
            .find(|(r, _)| contains(*r, x, y))
            .map(|(_, v)| *v)
    }

    /// The agent row under a cell, if any.
    pub fn row_at(&self, x: u16, y: u16) -> Option<&str> {
        self.rows
            .iter()
            .find(|(r, _)| contains(*r, x, y))
            .map(|(_, id)| id.as_str())
    }
}

/// Whether `r` covers the cell at (`x`, `y`).
pub fn contains(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x.saturating_add(r.width) && y >= r.y && y < r.y.saturating_add(r.height)
}

/// Separates the parts of the ids folds add to the graph and the hit map. No
/// agent id contains a control character, so these never collide with one.
const FOLD_SEP: char = '\u{1f}';

/// The graph node id of the fold under `parent`.
pub fn fold_card_id(parent: &str) -> String {
    format!("{parent}{FOLD_SEP}done")
}

/// The parent of the fold a graph node id names, if it names one.
pub fn fold_of_card(id: &str) -> Option<&str> {
    id.strip_suffix("done")?.strip_suffix(FOLD_SEP)
}

/// The hit-map key of the Now view's row for the fold under `parent`.
pub fn fold_row_key(parent: &str) -> String {
    format!("{FOLD_SEP}fold:{parent}")
}

/// The parent of the fold a hit-map row key names, if it names one.
pub fn fold_of_row_key(key: &str) -> Option<&str> {
    key.strip_prefix(FOLD_SEP)?.strip_prefix("fold:")
}

/// Which agents the Now and Graph views fold away, from [`fold_set`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldSet {
    /// Each fold, by the parent it hangs under: its members in tree order.
    pub by_parent: HashMap<String, Vec<String>>,
    /// Every agent a fold hides, members and their descendants alike, with
    /// the parent of the fold that shows in its place.
    pub member_of: HashMap<String, String>,
}

/// Fold the done subagents of each parent that has two or more of them.
///
/// A subagent can fold once it is `Done` for good: a reliable completion
/// signal pinned it (`is_terminal`), or the replay has ended (`settled`). A
/// live agent judged done only because it went quiet can come back, and
/// folding it would make it blink in and out. A parent in `expanded` keeps
/// its children apart. The selection plays no part, so every view agrees.
///
/// Folds form in tree order, so an agent inside a hidden subtree is hidden
/// with it and never starts a fold of its own.
pub fn fold_set(session: &SessionModel, expanded: &HashSet<String>, settled: bool) -> FoldSet {
    let foldable = |id: &str| {
        session.agent(id).is_some_and(|a| {
            a.kind == AgentKind::Subagent
                && a.status == AgentStatus::Done
                && (a.is_terminal() || settled)
        })
    };
    let order = session.tree_order();
    // Each row's parent in the tree (the last row one level up), and how
    // many foldable children each parent has.
    let mut parents = Vec::with_capacity(order.len());
    let mut path: Vec<&str> = Vec::new();
    let mut candidates: HashMap<&str, usize> = HashMap::new();
    for &(id, depth) in &order {
        path.truncate(depth);
        let parent = path.last().copied();
        if let Some(p) = parent
            && foldable(id)
        {
            *candidates.entry(p).or_default() += 1;
        }
        parents.push(parent);
        path.push(id);
    }
    let mut folds = FoldSet::default();
    for (&(id, _), parent) in order.iter().zip(parents) {
        let Some(parent) = parent else {
            continue;
        };
        if let Some(fold) = folds.member_of.get(parent).cloned() {
            // Under a hidden agent: hidden with it.
            folds.member_of.insert(id.to_string(), fold);
        } else if foldable(id)
            && candidates.get(parent).is_some_and(|n| *n >= 2)
            && !expanded.contains(parent)
        {
            folds
                .by_parent
                .entry(parent.to_string())
                .or_default()
                .push(id.to_string());
            folds.member_of.insert(id.to_string(), parent.to_string());
        }
    }
    folds
}

/// One line of the Now view, or one card on the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row<'a> {
    Agent {
        id: &'a str,
        depth: usize,
    },
    /// A fold, in the place of its first member.
    Folded {
        parent: &'a str,
        members: &'a [String],
        depth: usize,
    },
}

/// The rows the Now view lists: tree order, with each fold standing in for
/// its members and everything under them.
pub fn rows<'a>(session: &'a SessionModel, folds: &'a FoldSet) -> Vec<Row<'a>> {
    session
        .tree_order()
        .into_iter()
        .filter_map(|(id, depth)| match folds.member_of.get(id) {
            None => Some(Row::Agent { id, depth }),
            Some(parent) => {
                let members = folds.by_parent.get(parent)?;
                (members.first().map(String::as_str) == Some(id)).then_some(Row::Folded {
                    parent,
                    members,
                    depth,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_cycle_both_ways() {
        assert_eq!(View::Now.cycle(1), View::Lanes);
        assert_eq!(View::Graph.cycle(1), View::Now);
        assert_eq!(View::Now.cycle(-1), View::Graph);
    }

    #[test]
    fn lane_axis_maps_time_to_its_column() {
        let t = |s: i64| DateTime::from_timestamp(s, 0).unwrap();
        let axis = LaneAxis {
            area: Rect::new(0, 0, 3, 1),
            columns: vec![t(0), t(10), t(20)],
            ms_per_col: 10_000,
        };
        assert_eq!(axis.column_of(t(-5)), 0);
        assert_eq!(axis.column_of(t(10)), 1);
        assert_eq!(axis.column_of(t(19)), 1);
        assert_eq!(axis.column_of(t(99)), 2);
    }
}
