//! SEED components for the terminal.
//!
//! Each widget holds a [`Theme`](super::theme::Theme) and is built by a
//! constructor taking the theme first. Inline widgets expose
//! `spans() -> Vec<Span<'static>>` (to splice into a `Line`) and `width()`;
//! block widgets implement [`ratatui::widgets::Widget`].

mod badge;
mod callout;
mod chip;
mod divider;
mod key_hint;
mod list_item;
mod progress;
mod skeleton;
mod snackbar;
mod tablist;

pub use badge::{Badge, BadgeVariant};
pub use callout::Callout;
pub use chip::Chip;
pub use divider::Divider;
pub use key_hint::KeyHint;
pub use list_item::ListItem;
pub use progress::Spinner;
pub use skeleton::Skeleton;
pub use snackbar::Snackbar;
pub use tablist::TabList;

use ratatui::text::Span;

/// Display width of `s` in terminal columns, saturating at `u16::MAX` (the
/// unit of widget geometry).
pub fn text_width(s: &str) -> u16 {
    u16::try_from(crate::ui::text::width(s)).unwrap_or(u16::MAX)
}

/// Total display width of `spans`.
pub fn spans_width(spans: &[Span<'_>]) -> u16 {
    spans
        .iter()
        .fold(0u16, |w, s| w.saturating_add(text_width(&s.content)))
}

#[cfg(test)]
pub(crate) mod testing {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, widgets::Widget};

    use crate::ui::seed::theme::{Depth, Mode, Theme};

    /// Every mode × depth combination.
    pub fn all_themes() -> [Theme; 4] {
        [
            Theme::new(Mode::Light, Depth::TrueColor),
            Theme::new(Mode::Dark, Depth::TrueColor),
            Theme::new(Mode::Light, Depth::Ansi256),
            Theme::new(Mode::Dark, Depth::Ansi256),
        ]
    }

    /// Render `widget` into a `width` × `height` TestBackend frame.
    pub fn render(widget: impl Widget, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| f.render_widget(widget, Rect::new(0, 0, width, height)))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// The characters of row `y`, concatenated.
    pub fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_count_columns_not_chars() {
        assert_eq!(text_width("안a"), 3);
        assert_eq!(crate::ui::text::truncate("안녕하세요", 5), "안녕…");
        assert_eq!(spans_width(&[Span::raw("ab"), Span::raw("안")]), 4);
    }
}
