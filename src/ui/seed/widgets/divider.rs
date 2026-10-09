//! SEED divider (`divider.yaml`, 1 px): a hairline in `stroke.neutral-weak`.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::ui::seed::{theme::Theme, tokens::stroke};

#[derive(Clone, Copy, Debug)]
pub struct Divider {
    theme: Theme,
    vertical: bool,
}

impl Divider {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            vertical: false,
        }
    }

    /// Draw `│` down the first column instead of `─` along the first row.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }
}

impl Widget for Divider {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let style = self.theme.fg(stroke::NEUTRAL_WEAK);
        if self.vertical {
            for y in area.y..area.bottom() {
                buf[(area.x, y)].set_symbol("│").set_style(style);
            }
        } else {
            for x in area.x..area.right() {
                buf[(x, area.y)].set_symbol("─").set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::widgets::testing::{all_themes, render, row};

    #[test]
    fn divider_draws_hairline_in_stroke_token() {
        for theme in all_themes() {
            let buf = render(Divider::new(theme), 6, 2);
            assert_eq!(row(&buf, 0), "──────");
            assert_eq!(row(&buf, 1).trim(), "");
            assert_eq!(buf[(3, 0)].fg, theme.color(stroke::NEUTRAL_WEAK));

            let buf = render(Divider::new(theme).vertical(), 3, 4);
            for y in 0..4 {
                assert_eq!(buf[(0, y)].symbol(), "│");
                assert_eq!(buf[(0, y)].fg, theme.color(stroke::NEUTRAL_WEAK));
                assert_eq!(buf[(1, y)].symbol(), " ");
            }
        }
    }
}
