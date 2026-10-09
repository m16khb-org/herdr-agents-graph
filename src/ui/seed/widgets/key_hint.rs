//! Keyboard hint: a key cap followed by what it does.

use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::spans_width;
use crate::ui::seed::{
    theme::{Theme, Tone},
    tokens::fg,
};

#[derive(Clone, Debug)]
pub struct KeyHint {
    theme: Theme,
    key: String,
    label: String,
}

impl KeyHint {
    pub fn new(theme: Theme, key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            theme,
            key: key.into(),
            label: label.into(),
        }
    }

    /// ` key ` as a neutral-weak chip (bold `fg.neutral`), then ` label` in
    /// `fg.neutral-subtle`.
    pub fn spans(&self) -> Vec<Span<'static>> {
        let cap = Style::default()
            .fg(self.theme.color(fg::NEUTRAL))
            .bg(self.theme.tone(Tone::Neutral).weak_bg)
            .add_modifier(Modifier::BOLD);
        vec![
            Span::styled(format!(" {} ", self.key), cap),
            Span::styled(
                format!(" {}", self.label),
                self.theme.fg(fg::NEUTRAL_SUBTLE),
            ),
        ]
    }

    pub fn width(&self) -> u16 {
        spans_width(&self.spans())
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use ratatui::{layout::Rect, text::Line, widgets::Paragraph};

    use super::*;
    use crate::ui::seed::tokens::bg;
    use crate::ui::seed::widgets::testing::{all_themes, row};

    #[test]
    fn key_hint_has_key_cap_and_subtle_label() {
        for theme in all_themes() {
            let hint = KeyHint::new(theme, "Tab", "switch view");
            let mut terminal = Terminal::new(TestBackend::new(24, 1)).unwrap();
            terminal
                .draw(|f| {
                    f.render_widget(
                        Paragraph::new(Line::from(hint.spans())),
                        Rect::new(0, 0, 24, 1),
                    )
                })
                .unwrap();
            let buf = terminal.backend().buffer();
            assert_eq!(row(buf, 0).trim_end(), " Tab  switch view");
            assert_eq!(hint.width(), 17);
            let key = &buf[(1, 0)];
            assert_eq!(key.symbol(), "T");
            assert_eq!(key.fg, theme.color(fg::NEUTRAL));
            assert_eq!(key.bg, theme.color(bg::NEUTRAL_WEAK));
            assert!(key.modifier.contains(Modifier::BOLD));
            let label = &buf[(6, 0)];
            assert_eq!(label.symbol(), "s");
            assert_eq!(label.fg, theme.color(fg::NEUTRAL_SUBTLE));
        }
    }
}
