//! The graph's minimap: the whole graph at one fixed scale in the canvas's
//! top-right corner, with a frame around the part the canvas shows.
//!
//! The scale comes from the nodes alone. rataflow's own minimap also fits the
//! viewport into it, so zooming out or panning rescaled the map: cards ran
//! together into half blocks and slid off its edge. Here zoom and pan move
//! only the frame.

use rataflow::types::Rect as WorldRect;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::state::AgentFlow;
use crate::ui::seed::theme::Theme;
use crate::ui::seed::tokens::{brand, fg, stroke};

/// The map's size in cells, border included.
pub(crate) const MAP_WIDTH: u16 = 26;
/// See [`MAP_WIDTH`].
pub(crate) const MAP_HEIGHT: u16 = 10;
/// Below this canvas size the map would cover the graph it maps.
const MIN_CANVAS: (u16, u16) = (60, 20);

/// Draw the map of `flow` over the top-right corner of `canvas`, one cell in
/// from its edges. `selected` is the node drawn in the brand color.
pub(crate) fn render(
    buf: &mut Buffer,
    canvas: Rect,
    flow: &AgentFlow,
    selected: Option<&str>,
    theme: &Theme,
) {
    if canvas.width < MIN_CANVAS.0 || canvas.height < MIN_CANVAS.1 {
        return;
    }
    let area = Rect::new(
        canvas.right() - MAP_WIDTH - 1,
        canvas.y + 1,
        MAP_WIDTH,
        MAP_HEIGHT,
    );
    let surface = theme.surface();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(surface.patch(theme.fg(stroke::NEUTRAL_WEAK)))
        .style(surface)
        .title_top(Line::from(" map ").style(surface.patch(theme.fg(fg::NEUTRAL_SUBTLE))));
    let inner = block.inner(area);
    buf.set_style(area, surface);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].set_char(' ');
        }
    }
    block.render(area, buf);

    let Some(bounds) = node_bounds(flow) else {
        return;
    };
    let scale = Scale::new(bounds, inner);

    // What the canvas shows, in world units: drawn first, so the cards
    // stay whole where the frame crosses them.
    let size = flow.canvas_size();
    let zoom = flow.viewport.zoom;
    if size.width > 0.0 && size.height > 0.0 && zoom > 0.0 {
        let left = -flow.viewport.x / zoom;
        let top = -flow.viewport.y / zoom;
        let (right, bottom) = (left + size.width / zoom, top + size.height / zoom);
        let overlaps = left < bounds.right
            && right > bounds.left
            && top < bounds.bottom
            && bottom > bounds.top;
        if overlaps {
            let (x0, x1) = scale.span_x(left, right);
            let (y0, y1) = scale.span_y(top, bottom);
            frame(buf, (x0, y0, x1, y1), theme);
        }
    }

    for node in flow.nodes() {
        let b = node.bounds();
        let (x0, x1) = scale.span_x(b.position.x, b.right());
        let (y0, y1) = scale.span_y(b.position.y, b.bottom());
        let token = if selected == Some(node.id.as_str()) {
            brand::FG
        } else {
            stroke::NEUTRAL_SOLID
        };
        let style = surface.patch(theme.fg(token));
        for y in y0..=y1 {
            for x in x0..=x1 {
                buf[(x, y)].set_char('█').set_style(style);
            }
        }
    }
}

/// The world-space box around every node.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

fn node_bounds(flow: &AgentFlow) -> Option<Bounds> {
    flow.nodes()
        .map(|n| n.bounds())
        .fold(None, |acc, b: WorldRect| {
            let next = Bounds {
                left: b.position.x,
                top: b.position.y,
                right: b.right(),
                bottom: b.bottom(),
            };
            Some(match acc {
                None => next,
                Some(a) => Bounds {
                    left: a.left.min(next.left),
                    top: a.top.min(next.top),
                    right: a.right.max(next.right),
                    bottom: a.bottom.max(next.bottom),
                },
            })
        })
}

/// World units to map cells. World units are terminal cells at zoom 1, so
/// one scale for both axes keeps the graph's proportions; the node bounds
/// fit the map's inside and sit centered in it.
struct Scale {
    /// Cells per world unit.
    per: f64,
    /// The world point at the inside's top-left corner.
    origin: (f64, f64),
    inner: Rect,
}

impl Scale {
    fn new(bounds: Bounds, inner: Rect) -> Self {
        let w = (bounds.right - bounds.left).max(1.0);
        let h = (bounds.bottom - bounds.top).max(1.0);
        let (cols, rows) = (f64::from(inner.width), f64::from(inner.height));
        let per = (cols / w).min(rows / h);
        let pad = ((cols - w * per) / 2.0, (rows - h * per) / 2.0);
        Scale {
            per,
            origin: (bounds.left - pad.0 / per, bounds.top - pad.1 / per),
            inner,
        }
    }

    /// The cells `from..to` covers along x, clamped to the map's inside.
    fn span_x(&self, from: f64, to: f64) -> (u16, u16) {
        let at = |v: f64| (v - self.origin.0) * self.per;
        span(at(from), at(to), self.inner.x, self.inner.width)
    }

    /// See [`span_x`](Self::span_x).
    fn span_y(&self, from: f64, to: f64) -> (u16, u16) {
        let at = |v: f64| (v - self.origin.1) * self.per;
        span(at(from), at(to), self.inner.y, self.inner.height)
    }
}

/// First and last of `cells` cells (from `origin`) that the span `from..to`,
/// in cells, rounds to — at least one, clamped to the inside.
fn span(from: f64, to: f64, origin: u16, cells: u16) -> (u16, u16) {
    let last = f64::from(cells.saturating_sub(1));
    let first = from.round().clamp(0.0, last);
    let end = (to.round() - 1.0).clamp(first, last);
    (origin + first as u16, origin + end as u16)
}

/// The viewport's outline, `┌─┐│└┘`.
fn frame(buf: &mut Buffer, (x0, y0, x1, y1): (u16, u16, u16, u16), theme: &Theme) {
    let style = theme.surface().patch(theme.fg(stroke::NEUTRAL_CONTRAST));
    let mut put = |x: u16, y: u16, c: char| {
        buf[(x, y)].set_char(c).set_style(style);
    };
    for x in x0..=x1 {
        put(x, y0, '─');
        put(x, y1, '─');
    }
    for y in y0..=y1 {
        put(x0, y, '│');
        put(x1, y, '│');
    }
    put(x0, y0, '┌');
    put(x1, y0, '┐');
    put(x0, y1, '└');
    put(x1, y1, '┘');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::graph::{AgentNode, new_flow};
    use crate::state::session::AgentStatus;
    use crate::ui::seed::theme::{Depth, Mode};
    use rataflow::Node;

    fn card(id: &str, x: f64, y: f64) -> Node<AgentNode> {
        let content = AgentNode {
            title: id.into(),
            description: None,
            status: AgentStatus::Done,
            tool_count: 0,
            last_tool: None,
            output_tokens: 0,
            interactive: false,
        };
        Node::new(id.to_string(), (x, y), (26.0, 6.0), content)
    }

    /// The map's cells as text.
    fn map_cells(buf: &Buffer, canvas: Rect) -> Vec<String> {
        let left = canvas.right() - MAP_WIDTH - 1;
        (canvas.y + 1..canvas.y + 1 + MAP_HEIGHT)
            .map(|y| {
                (left..left + MAP_WIDTH)
                    .map(|x| buf[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    /// Zoom and pan move the viewport frame and nothing else: the cards keep
    /// their cells, and the frame stays inside the map.
    #[test]
    fn minimap_shape_ignores_zoom_and_pan() {
        let theme = Theme::new(Mode::Dark, Depth::TrueColor);
        let mut flow = new_flow();
        for (id, x, y) in [
            ("main", 40.0, 0.0),
            ("a", 0.0, 12.0),
            ("b", 30.0, 12.0),
            ("c", 60.0, 12.0),
        ] {
            flow.add_node(card(id, x, y)).unwrap();
        }
        let canvas = Rect::new(0, 0, 100, 30);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let mut maps = Vec::new();
        for (zoom, pan) in [
            (1.0, (0.0, 0.0)),
            (0.3, (0.0, 0.0)),
            (0.3, (25.0, 18.0)),
            (2.5, (-40.0, -10.0)),
        ] {
            terminal
                .draw(|f| {
                    f.render_widget(&mut flow, canvas);
                    flow.zoom_to(zoom);
                    flow.viewport.set_offset(pan.0, pan.1);
                    render(f.buffer_mut(), canvas, &flow, Some("a"), &theme);
                })
                .unwrap();
            maps.push(map_cells(terminal.backend().buffer(), canvas));
        }
        let cards = |map: &[String]| -> Vec<String> {
            map.iter()
                .map(|row| {
                    row.chars()
                        .map(|c| if c == '█' { c } else { ' ' })
                        .collect()
                })
                .collect()
        };
        for map in &maps[1..] {
            assert_eq!(cards(map), cards(&maps[0]), "{map:#?}");
        }
        assert!(maps[0].concat().contains('█'));
        // Zoomed out, the canvas shows the whole graph: every card lies
        // inside the frame.
        let at = |map: &[String], c: char| {
            map.iter()
                .enumerate()
                .find_map(|(y, row)| row.chars().position(|x| x == c).map(|x| (x, y)))
        };
        let (left, top) = at(&maps[1], '┌').expect("a frame");
        let (right, bottom) = at(&maps[1], '┘').expect("a frame");
        for (y, row) in maps[1].iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c == '█' {
                    assert!(
                        (left..=right).contains(&x) && (top..=bottom).contains(&y),
                        "{:#?}",
                        maps[1]
                    );
                }
            }
        }
        // Panned and zoomed in, the frame has moved.
        assert_ne!(maps[3], maps[1]);
        assert!(maps[0][0].contains(" map "));
    }
}
