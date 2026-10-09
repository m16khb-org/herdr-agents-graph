//! SEED badge (`badge.yaml`): a tone-coloured label with a status glyph.

use ratatui::{style::Style, text::Span};

use super::text_width;
use crate::ui::seed::theme::{Theme, Tone};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    /// `fg.X-contrast` on `bg.X-weak`.
    #[default]
    Weak,
    /// `fg.on-X-solid` on `bg.X-solid`.
    Solid,
    /// `fg.X`, no background (a terminal has no 1 px stroke to draw).
    Outline,
}

#[derive(Clone, Debug)]
pub struct Badge {
    theme: Theme,
    tone: Tone,
    label: String,
    glyph: Option<char>,
    variant: BadgeVariant,
}

impl Badge {
    pub fn new(theme: Theme, tone: Tone, label: impl Into<String>) -> Self {
        Self {
            theme,
            tone,
            label: label.into(),
            glyph: None,
            variant: BadgeVariant::Weak,
        }
    }

    pub fn glyph(mut self, glyph: char) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    fn text(&self) -> String {
        match self.glyph {
            Some(g) => format!(" {g} {} ", self.label),
            None => format!(" {} ", self.label),
        }
    }

    pub fn style(&self) -> Style {
        let c = self.theme.tone(self.tone);
        match self.variant {
            BadgeVariant::Weak => Style::default().fg(c.fg_contrast).bg(c.weak_bg),
            BadgeVariant::Solid => Style::default().fg(c.on_solid).bg(c.solid_bg),
            BadgeVariant::Outline => Style::default().fg(c.fg),
        }
    }

    /// One span, padded with a space on each side.
    pub fn spans(&self) -> Vec<Span<'static>> {
        vec![Span::styled(self.text(), self.style())]
    }

    pub fn width(&self) -> u16 {
        text_width(&self.text())
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, style::Color};
    use ratatui::{layout::Rect, text::Line, widgets::Paragraph};

    use super::*;
    use crate::ui::seed::theme::{Depth, Mode};
    use crate::ui::seed::tokens::{bg, fg};
    use crate::ui::seed::widgets::testing::{all_themes, row};

    fn draw(badge: &Badge) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(20, 1)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(
                    Paragraph::new(Line::from(badge.spans())),
                    Rect::new(0, 0, 20, 1),
                )
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn badge_renders_glyph_and_word() {
        let theme = Theme::new(Mode::Dark, Depth::TrueColor);
        let badge = Badge::new(theme, Tone::Informative, "running").glyph('●');
        let buf = draw(&badge);
        assert_eq!(row(&buf, 0).trim_end(), " ● running");
        assert_eq!(badge.width(), 11);
        let glyph = &buf[(1, 0)];
        assert_eq!(glyph.symbol(), "●");
        assert_eq!(glyph.fg, theme.color(fg::INFORMATIVE_CONTRAST));
        assert_eq!(glyph.bg, theme.color(bg::INFORMATIVE_WEAK));
        let word = &buf[(3, 0)];
        assert_eq!(word.symbol(), "r");
        assert_eq!(word.fg, theme.color(fg::INFORMATIVE_CONTRAST));
        assert_eq!(word.bg, theme.color(bg::INFORMATIVE_WEAK));
        // The padding cells carry the background too (radius full -> end padding).
        assert_eq!(buf[(0, 0)].bg, theme.color(bg::INFORMATIVE_WEAK));
        assert_eq!(buf[(10, 0)].bg, theme.color(bg::INFORMATIVE_WEAK));
        assert_eq!(buf[(11, 0)].bg, Color::Reset);
    }

    #[test]
    fn badge_variants_resolve_tone_tokens_in_every_theme() {
        for theme in all_themes() {
            let c = theme.tone(Tone::Critical);
            let weak = Badge::new(theme, Tone::Critical, "failed").style();
            assert_eq!((weak.fg, weak.bg), (Some(c.fg_contrast), Some(c.weak_bg)));
            let solid = Badge::new(theme, Tone::Critical, "failed")
                .variant(BadgeVariant::Solid)
                .style();
            assert_eq!((solid.fg, solid.bg), (Some(c.on_solid), Some(c.solid_bg)));
            let outline = Badge::new(theme, Tone::Critical, "failed")
                .variant(BadgeVariant::Outline)
                .style();
            assert_eq!((outline.fg, outline.bg), (Some(c.fg), None));
            assert_eq!(c.fg, theme.color(fg::CRITICAL));
        }
        let light = Theme::new(Mode::Light, Depth::TrueColor);
        let dark = Theme::new(Mode::Dark, Depth::TrueColor);
        assert_ne!(
            Badge::new(light, Tone::Positive, "done").style(),
            Badge::new(dark, Tone::Positive, "done").style()
        );
    }

    #[test]
    fn badge_without_glyph_is_just_padded_label() {
        let theme = Theme::new(Mode::Light, Depth::Ansi256);
        let badge = Badge::new(theme, Tone::Neutral, "idle");
        assert_eq!(badge.spans()[0].content, " idle ");
        assert_eq!(badge.width(), 6);
    }
}
