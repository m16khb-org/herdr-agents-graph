//! SEED chip (`chip.yaml`): a pill-shaped toggle label.

use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::text_width;
use crate::ui::seed::theme::{Theme, Tone};

#[derive(Clone, Debug)]
pub struct Chip {
    theme: Theme,
    label: String,
    selected: bool,
}

impl Chip {
    pub fn new(theme: Theme, label: impl Into<String>) -> Self {
        Self {
            theme,
            label: label.into(),
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn text(&self) -> String {
        format!(" {} ", self.label)
    }

    /// Unselected: `fg.neutral` on `bg.neutral-weak`. Selected: brand tone on
    /// `bg.brand-weak`, bold.
    pub fn style(&self) -> Style {
        if self.selected {
            let c = self.theme.tone(Tone::Brand);
            Style::default()
                .fg(c.fg_contrast)
                .bg(c.weak_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            self.theme
                .fg(crate::ui::seed::tokens::fg::NEUTRAL)
                .bg(self.theme.tone(Tone::Neutral).weak_bg)
        }
    }

    pub fn spans(&self) -> Vec<Span<'static>> {
        vec![Span::styled(self.text(), self.style())]
    }

    pub fn width(&self) -> u16 {
        text_width(&self.text())
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use ratatui::{layout::Rect, text::Line, widgets::Paragraph};

    use super::*;
    use crate::ui::seed::tokens::{bg, brand, fg};
    use crate::ui::seed::widgets::testing::{all_themes, row};

    #[test]
    fn chip_selected_and_unselected_use_their_tokens() {
        for theme in all_themes() {
            let chip = Chip::new(theme, "Now");
            let off = chip.style();
            assert_eq!(off.fg, Some(theme.color(fg::NEUTRAL)));
            assert_eq!(off.bg, Some(theme.color(bg::NEUTRAL_WEAK)));
            assert!(!off.add_modifier.contains(Modifier::BOLD));
            let on = chip.selected(true).style();
            assert_eq!(on.fg, Some(theme.color(brand::FG_CONTRAST)));
            assert_eq!(on.bg, Some(theme.color(brand::BG_WEAK)));
            assert!(on.add_modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn chip_renders_padded_label() {
        let theme = all_themes()[1];
        let chip = Chip::new(theme, "Lanes").selected(true);
        assert_eq!(chip.width(), 7);
        let mut terminal = Terminal::new(TestBackend::new(10, 1)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(
                    Paragraph::new(Line::from(chip.spans())),
                    Rect::new(0, 0, 10, 1),
                )
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        assert_eq!(row(buf, 0).trim_end(), " Lanes");
        assert_eq!(buf[(1, 0)].bg, theme.color(brand::BG_WEAK));
    }
}
