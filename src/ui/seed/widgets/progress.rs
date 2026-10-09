//! Progress indication (`progress-circle.yaml`, brand tone: range = `bg.brand-solid`).

use ratatui::text::Span;

use crate::ui::seed::{theme::Theme, tokens::brand};

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// One braille cell that turns with `phase` (whole seconds from the caller).
#[derive(Clone, Copy, Debug)]
pub struct Spinner {
    theme: Theme,
    phase: u64,
}

impl Spinner {
    pub fn new(theme: Theme, phase: u64) -> Self {
        Self { theme, phase }
    }

    pub fn glyph(&self) -> char {
        FRAMES[(self.phase % FRAMES.len() as u64) as usize]
    }

    pub fn spans(&self) -> Vec<Span<'static>> {
        vec![Span::styled(
            self.glyph().to_string(),
            self.theme.fg(brand::BG_SOLID),
        )]
    }

    pub fn width(&self) -> u16 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::widgets::testing::all_themes;

    #[test]
    fn spinner_cycles_with_phase_in_brand_color() {
        for theme in all_themes() {
            let a = Spinner::new(theme, 0);
            assert_eq!(a.glyph(), FRAMES[0]);
            assert_eq!(Spinner::new(theme, 3).glyph(), FRAMES[3]);
            assert_eq!(Spinner::new(theme, 10).glyph(), FRAMES[0]);
            assert_ne!(a.glyph(), Spinner::new(theme, 1).glyph());
            let spans = a.spans();
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].style.fg, Some(theme.color(brand::BG_SOLID)));
            assert_eq!(a.width(), 1);
        }
    }
}
