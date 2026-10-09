//! What the screen is showing, as state: the active view, the selection
//! every view shares, transient notices, and where the last frame put the
//! things a mouse can hit.
//!
//! Plain data with no rendering in it. The ui reads it to draw and writes the
//! hit map back; the input handler reads the hit map to route clicks.

use chrono::{DateTime, Utc};
use ratatui::layout::Rect;
use web_time::Instant;

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
