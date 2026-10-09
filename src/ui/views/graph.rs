//! The graph view: the spawn tree on rataflow's pannable canvas, with agent
//! cards and parent edges drawn from SEED tokens, and a minimap.
//!
//! The canvas and its background pattern take their colors from the
//! rataflow palette, which this view rebuilds from the theme on every
//! frame; cards and edges recover the full theme from that palette
//! ([`Theme::from_palette`]) so they can use every status tone.

use rataflow::{
    Background, EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, NodeContent,
    NodeRenderContext, Path,
};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget};

use crate::state::App;
use crate::state::graph::{AgentEdge, AgentNode};
use crate::state::session::{AgentStatus, status_word};
use crate::state::view::fold_card_id;
use crate::ui::seed::theme::{Depth, Mode, Theme, Tone, status_tone};
use crate::ui::seed::tokens::{brand, fg, stroke};
use crate::ui::text::{fmt_tokens, fmt_tool_count, truncate, width};

/// Below this on-screen size a card has no room for text: it renders as a
/// cell of its status tone instead (semantic zoom).
pub const CELL_MIN_WIDTH: u16 = 10;
/// See [`CELL_MIN_WIDTH`].
pub const CELL_MIN_HEIGHT: u16 = 3;

/// The theme a card or edge is drawn with, recovered from the palette the
/// view handed rataflow.
fn theme_of(palette: &rataflow::Palette) -> Theme {
    Theme::from_palette(palette).unwrap_or(Theme::new(Mode::Dark, Depth::TrueColor))
}

/// Render the graph into `area`.
pub(crate) fn render(frame: &mut Frame, area: Rect, app: &mut App, theme: &Theme) {
    if area.is_empty() {
        return;
    }
    app.hit.canvas = Some(area);
    app.flow.theme = rataflow::Theme::Custom(theme.palette());

    // The flow keeps its own selection; make it show the app's, a fold by
    // its card.
    let want = match (&app.selection.agent, &app.selection.folded) {
        (Some(id), _) => Some(id.clone()),
        (None, Some(parent)) => Some(fold_card_id(parent)),
        (None, None) => None,
    };
    let have = app.flow.selected_nodes().next().map(|n| n.id.clone());
    if want != have {
        match want.as_deref() {
            Some(id) if app.flow.node(id).is_some() => app.flow.select_node(id),
            Some(_) => {}
            None => app.flow.clear_selection(),
        }
    }

    // Background and canvas: the pattern reads the flow, the canvas renders
    // through `&mut Flow`; the borrows must not overlap.
    frame.render_widget(Background::new(&app.flow), area);
    frame.render_widget(&mut app.flow, area);
    // A selection made elsewhere is centered here, after the canvas has its
    // size: `center_on` probes the last-rendered viewport.
    if let Some(id) = app.pending_center.take() {
        app.center_node(&id, false);
    }
    let selected = app.flow.selected_nodes().next().map(|n| n.id.clone());
    super::minimap::render(
        frame.buffer_mut(),
        area,
        &app.flow,
        selected.as_deref(),
        theme,
    );
}

impl NodeContent for AgentNode {
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
        let area = ctx.area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = theme_of(&ctx.theme.palette());
        let tone = theme.tone(status_tone(self.status));

        // Cell level: a bordered card would carry nothing, so paint a cell of
        // the status tone (brand when selected).
        if area.width < CELL_MIN_WIDTH || area.height < CELL_MIN_HEIGHT {
            // Still a glyph, not colour alone: the status mark sits in the
            // cell's first column on the tone's solid fill.
            let solid = if ctx.selected {
                theme.tone(Tone::Brand)
            } else {
                tone
            };
            let fill = Style::default().bg(solid.solid_bg).fg(solid.on_solid);
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    buf[(x, y)].set_char(' ').set_style(fill);
                }
            }
            buf[(area.x, area.y)].set_char(self.status.glyph());
            return;
        }

        let surface = theme.surface();
        let border = if ctx.selected {
            theme.fg(brand::FG)
        } else {
            theme.fg(stroke::NEUTRAL_WEAK)
        };
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(surface.patch(border))
            .style(surface);
        if area.width >= 4 {
            block = block.padding(Padding::horizontal(1));
        }
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let w = inner.width as usize;

        // Alive agents breathe on the shared animation clock.
        let glyph = if self.status == AgentStatus::Running && (ctx.animation_phase / 4) % 2 == 1 {
            '○'
        } else {
            self.status.glyph()
        };
        let mut title_style = surface
            .patch(theme.fg(fg::NEUTRAL))
            .add_modifier(Modifier::BOLD);
        if ctx.selected {
            title_style = title_style.patch(theme.fg(brand::FG));
        }
        let mut lines = vec![Line::from(vec![
            Span::styled(format!("{glyph} "), surface.fg(tone.fg)),
            Span::styled(truncate(&self.title, w.saturating_sub(2)), title_style),
        ])];
        if let Some(desc) = self.description.as_deref().filter(|d| !d.is_empty()) {
            lines.push(Line::from(Span::styled(
                truncate(desc, w),
                surface.patch(theme.fg(fg::NEUTRAL_MUTED)),
            )));
        }
        let count = fmt_tool_count(self.tool_count);
        let tools = match self.last_tool.as_deref() {
            Some(last) => {
                let prefix = format!("{count} · ");
                format!(
                    "{prefix}{}",
                    truncate(last, w.saturating_sub(width(&prefix)))
                )
            }
            None => count,
        };
        lines.push(Line::from(Span::styled(
            tools,
            surface.patch(theme.fg(fg::NEUTRAL_SUBTLE)),
        )));
        let word = status_word(self.status, self.interactive);
        let mut footer = vec![Span::styled(word, surface.fg(tone.fg))];
        if self.output_tokens > 0 {
            footer.push(Span::styled(
                format!("  {} tok", fmt_tokens(self.output_tokens)),
                surface.patch(theme.fg(fg::NEUTRAL_SUBTLE)),
            ));
        }
        lines.push(Line::from(footer));

        for (i, line) in lines.into_iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            Paragraph::new(line).style(surface).render(row, buf);
        }
    }
}

impl EdgeContent for AgentEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        self.inner.compute_path(ctx)
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        // Edges are built non-selectable, so `ctx.selected` never applies.
        // A running child's edge takes the informative stroke: liveness reads
        // on the structure at any zoom.
        let style = if self.running {
            let theme = theme_of(&ctx.theme.palette());
            EdgeStyle::default().with_stroke_style(theme.fg(stroke::INFORMATIVE_SOLID))
        } else {
            EdgeStyle::default()
        };
        ctx.render_path(&style, None, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rataflow::types::Position;

    fn render_into(area: Rect, theme: Theme, selected: bool) -> Buffer {
        let node = AgentNode {
            title: "claude".into(),
            description: Some("research the plugin".into()),
            status: AgentStatus::Done,
            tool_count: 3,
            last_tool: Some("Bash".into()),
            output_tokens: 1200,
            interactive: false,
        };
        let ctx = NodeRenderContext {
            id: "main",
            area,
            selected,
            dragging: false,
            position_absolute: Position::new(0.0, 0.0),
            theme: rataflow::Theme::Custom(theme.palette()),
            animation_phase: 0,
        };
        let mut buf = Buffer::empty(area);
        node.render(&ctx, &mut buf);
        buf
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    #[test]
    fn card_shows_title_status_and_tokens_in_tokens_colors() {
        for theme in [
            Theme::new(Mode::Light, Depth::TrueColor),
            Theme::new(Mode::Dark, Depth::Ansi256),
        ] {
            let area = Rect::new(0, 0, 28, 6);
            let buf = render_into(area, theme, false);
            assert!(row(&buf, 1).contains("✓ claude"), "{}", row(&buf, 1));
            assert!(row(&buf, 4).contains("done"), "{}", row(&buf, 4));
            assert!(row(&buf, 4).contains("1.2k tok"));
            // The glyph takes the Done tone; the card face is the surface.
            let glyph = (0..area.width)
                .find(|&x| buf[(x, 1)].symbol() == "✓")
                .unwrap();
            assert_eq!(buf[(glyph, 1)].fg, theme.tone(Tone::Positive).fg);
            assert_eq!(
                buf[(glyph, 1)].bg,
                theme.color(crate::ui::seed::tokens::bg::LAYER_DEFAULT)
            );
            // Rounded corners: SEED radius r2+ on a box.
            assert_eq!(buf[(0, 0)].symbol(), "╭");
        }
    }

    #[test]
    fn selected_card_takes_the_brand_border() {
        let theme = Theme::new(Mode::Dark, Depth::TrueColor);
        let buf = render_into(Rect::new(0, 0, 28, 6), theme, true);
        assert_eq!(buf[(0, 0)].fg, theme.color(brand::FG));
    }

    /// Zoomed out, a card is a cell of its status tone that still carries its
    /// glyph, so the status never rests on colour alone.
    #[test]
    fn cell_level_fills_with_the_status_tone_and_glyph() {
        let theme = Theme::new(Mode::Light, Depth::TrueColor);
        let area = Rect::new(0, 0, 6, 2);
        let buf = render_into(area, theme, false);
        let done = theme.tone(status_tone(AgentStatus::Done));
        for y in 0..area.height {
            for x in 0..area.width {
                let want = if (x, y) == (0, 0) { "✓" } else { " " };
                assert_eq!(buf[(x, y)].symbol(), want);
                assert_eq!(buf[(x, y)].bg, done.solid_bg);
            }
        }
        assert_eq!(buf[(0, 0)].fg, done.on_solid);
    }
}
