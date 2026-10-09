//! SEED callout (`callout.yaml`): a tone-coloured notice of one or two rows.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::ui::seed::{
    theme::{Theme, Tone},
    tokens::fg,
};
use crate::ui::text::truncate;

/// Left accent bar.
const BAR: &str = "┃ ";
/// Cells before the title text: bar (2) + glyph and its space (2).
const INDENT: u16 = 4;

fn tone_glyph(tone: Tone) -> char {
    match tone {
        Tone::Neutral => '•',
        Tone::Brand => '◆',
        Tone::Positive => '✓',
        Tone::Critical => '✗',
        Tone::Warning => '!',
        Tone::Informative => 'i',
    }
}

#[derive(Clone, Debug)]
pub struct Callout {
    theme: Theme,
    tone: Tone,
    title: String,
    body: Option<String>,
}

impl Callout {
    pub fn new(theme: Theme, tone: Tone, title: impl Into<String>) -> Self {
        Self {
            theme,
            tone,
            title: title.into(),
            body: None,
        }
    }

    pub fn body(mut self, text: impl Into<String>) -> Self {
        self.body = Some(text.into());
        self
    }

    /// Rows this callout wants: 1, or 2 with a body.
    pub fn height(&self) -> u16 {
        1 + u16::from(self.body.is_some())
    }
}

impl Widget for Callout {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let c = self.theme.tone(self.tone);
        let rows = self.height().min(area.height);
        buf.set_style(
            Rect::new(area.x, area.y, area.width, rows),
            Style::default().bg(c.weak_bg),
        );
        let bar = Style::default().fg(c.fg).bg(c.weak_bg);
        let text_width = area.width.saturating_sub(INDENT);

        let head = Line::from(vec![
            Span::styled(BAR, bar),
            Span::styled(format!("{} ", tone_glyph(self.tone)), bar),
            Span::styled(
                truncate(&self.title, usize::from(text_width)),
                Style::default()
                    .fg(c.fg_contrast)
                    .bg(c.weak_bg)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        buf.set_line(area.x, area.y, &head, area.width);

        if let (Some(body), true) = (&self.body, rows >= 2) {
            let line = Line::from(vec![
                Span::styled(BAR, bar),
                Span::styled("  ", bar),
                Span::styled(
                    truncate(body, usize::from(text_width)),
                    self.theme.fg(fg::NEUTRAL_MUTED).bg(c.weak_bg),
                ),
            ]);
            buf.set_line(area.x, area.y + 1, &line, area.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::tokens::{bg, fg};
    use crate::ui::seed::widgets::testing::{all_themes, render, row};

    #[test]
    fn callout_draws_bar_title_body_on_weak_bg() {
        for theme in all_themes() {
            let callout = Callout::new(theme, Tone::Critical, "Session failed").body("exit 1");
            assert_eq!(callout.height(), 2);
            let buf = render(callout, 24, 2);
            assert_eq!(row(&buf, 0).trim_end(), "┃ ✗ Session failed");
            assert_eq!(row(&buf, 1).trim_end(), "┃   exit 1");
            let weak = theme.color(bg::CRITICAL_WEAK);
            assert_eq!(buf[(0, 0)].fg, theme.color(fg::CRITICAL));
            assert_eq!(buf[(4, 0)].fg, theme.color(fg::CRITICAL_CONTRAST));
            assert!(buf[(4, 0)].modifier.contains(Modifier::BOLD));
            assert_eq!(buf[(4, 1)].fg, theme.color(fg::NEUTRAL_MUTED));
            for y in 0..2 {
                for x in 0..24 {
                    assert_eq!(buf[(x, y)].bg, weak, "({x},{y})");
                }
            }
        }
    }

    #[test]
    fn callout_truncates_and_skips_body_without_room() {
        let theme = all_themes()[0];
        let buf = render(
            Callout::new(theme, Tone::Informative, "A very long title").body("body"),
            12,
            1,
        );
        assert_eq!(row(&buf, 0), "┃ i A very …");
        let one = Callout::new(theme, Tone::Positive, "ok");
        assert_eq!(one.height(), 1);
    }
}
