//! SEED skeleton (`skeleton.yaml`, neutral tone): placeholder bars standing in
//! for content that is still loading.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::ui::seed::{
    theme::Theme,
    tokens::{bg, stroke},
};

/// Bar width as a percentage of the area, repeated down the rows.
const WIDTHS: [u16; 5] = [90, 65, 80, 45, 70];

#[derive(Clone, Copy, Debug)]
pub struct Skeleton {
    theme: Theme,
}

impl Skeleton {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Width in cells of the bar drawn on row `row` of a `width`-wide area.
    pub fn bar_width(width: u16, row: u16) -> u16 {
        let pct = WIDTHS[usize::from(row) % WIDTHS.len()];
        (u32::from(width) * u32::from(pct) / 100).max(1) as u16
    }
}

impl Widget for Skeleton {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let style = self
            .theme
            .fg(stroke::NEUTRAL_WEAK)
            .bg(self.theme.color(bg::NEUTRAL_WEAK));
        for row in 0..area.height {
            let w = Self::bar_width(area.width, row).min(area.width);
            for x in area.x..area.x + w {
                buf[(x, area.y + row)].set_symbol("░").set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::widgets::testing::{all_themes, render, row};

    #[test]
    fn skeleton_rows_have_varying_bar_widths() {
        for theme in all_themes() {
            let buf = render(Skeleton::new(theme), 20, 3);
            assert_eq!(row(&buf, 0), format!("{}  ", "░".repeat(18)));
            assert_eq!(row(&buf, 1).trim_end(), "░".repeat(13));
            assert_eq!(row(&buf, 2).trim_end(), "░".repeat(16));
            assert_eq!(buf[(0, 0)].bg, theme.color(bg::NEUTRAL_WEAK));
            assert_eq!(buf[(0, 0)].fg, theme.color(stroke::NEUTRAL_WEAK));
            assert_ne!(buf[(19, 0)].bg, theme.color(bg::NEUTRAL_WEAK));
        }
        assert_eq!(Skeleton::bar_width(1, 3), 1);
    }
}
