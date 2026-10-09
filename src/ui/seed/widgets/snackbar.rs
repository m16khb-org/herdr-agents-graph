//! SEED snackbar (`snackbar.yaml`): a short message on `bg.neutral-solid`,
//! `fg.on-neutral-solid` text, floating over the bottom row.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use super::text_width;
use crate::ui::seed::{
    theme::{Theme, Tone},
    tokens::fg,
};
use crate::ui::text::truncate;

/// Padding on each side of the message inside the box.
const PAD: u16 = 1;
/// Cells kept free at each side of the area.
const MARGIN: u16 = 2;

#[derive(Clone, Debug)]
pub struct Snackbar {
    theme: Theme,
    message: String,
}

impl Snackbar {
    pub fn new(theme: Theme, message: impl Into<String>) -> Self {
        Self {
            theme,
            message: message.into(),
        }
    }

    /// The row a snackbar occupies within `within`: its bottom row.
    pub fn area(within: Rect) -> Rect {
        let h = within.height.min(1);
        Rect::new(within.x, within.bottom().saturating_sub(h), within.width, h)
    }

    /// Box width this message wants (before clipping to the area).
    pub fn width(&self) -> u16 {
        text_width(&self.message).saturating_add(PAD * 2)
    }
}

impl Widget for Snackbar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let row = Self::area(area);
        if row.is_empty() {
            return;
        }
        let max = row
            .width
            .saturating_sub(MARGIN * 2)
            .max(row.width.min(PAD * 2 + 1));
        let width = self.width().min(max);
        let x = row.x + (row.width - width) / 2;
        let box_rect = Rect::new(x, row.y, width, 1);
        let c = self.theme.tone(Tone::Neutral);
        let style = Style::default()
            .fg(self.theme.color(fg::ON_NEUTRAL_SOLID))
            .bg(c.solid_bg);
        buf.set_style(box_rect, style);
        let text = truncate(&self.message, usize::from(width.saturating_sub(PAD * 2)));
        let line = Line::from(vec![
            Span::styled(" ".repeat(usize::from(PAD)), style),
            Span::styled(text, style),
        ]);
        buf.set_line(box_rect.x, box_rect.y, &line, box_rect.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::tokens::bg;
    use crate::ui::seed::widgets::testing::{all_themes, render, row};

    #[test]
    fn snackbar_is_centered_on_the_bottom_row() {
        for theme in all_themes() {
            let buf = render(Snackbar::new(theme, "Copied"), 20, 3);
            assert_eq!(row(&buf, 0).trim(), "");
            assert_eq!(row(&buf, 1).trim(), "");
            // width 8 centered in 20: x = 6..14
            assert_eq!(row(&buf, 2), "      ".to_string() + " Copied " + "      ");
            assert_eq!(buf[(7, 2)].fg, theme.color(fg::ON_NEUTRAL_SOLID));
            assert_eq!(buf[(7, 2)].bg, theme.color(bg::NEUTRAL_SOLID));
            assert_eq!(buf[(6, 2)].bg, theme.color(bg::NEUTRAL_SOLID));
            assert_eq!(buf[(5, 2)].bg, ratatui::style::Color::Reset);
            assert_eq!(buf[(14, 2)].bg, ratatui::style::Color::Reset);
        }
    }

    #[test]
    fn snackbar_truncates_and_area_is_bottom_row() {
        let theme = all_themes()[0];
        let buf = render(Snackbar::new(theme, "x".repeat(40)), 20, 1);
        let text = row(&buf, 0);
        assert_eq!(text.trim(), format!("{}…", "x".repeat(13)));
        assert_eq!(
            Snackbar::area(Rect::new(3, 4, 20, 5)),
            Rect::new(3, 8, 20, 1)
        );
        assert!(Snackbar::area(Rect::new(0, 0, 10, 0)).is_empty());
    }
}
