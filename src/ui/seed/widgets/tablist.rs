//! SEED tab list (`tablist.yaml`): labels on `bg.layer-default`; the active
//! one is marked by an indicator (a `━` bar under it when the area has a
//! second row, otherwise an underline).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::text_width;
use crate::ui::seed::{
    theme::{Theme, Tone},
    tokens::{dimension, fg, stroke},
};

/// `spacing-x.global-gutter` in cells (8 px per cell).
const GUTTER: u16 = dimension::SPACING_X_GLOBAL_GUTTER / dimension::X2;
/// Padding on each side of a label.
const PAD: u16 = 1;
/// Cells between tabs.
const GAP: u16 = 1;

#[derive(Clone, Copy, Debug)]
pub struct TabList<'a> {
    theme: Theme,
    labels: &'a [&'a str],
    active: usize,
}

impl<'a> TabList<'a> {
    pub fn new(theme: Theme, labels: &'a [&'a str], active: usize) -> Self {
        Self {
            theme,
            labels,
            active,
        }
    }

    /// One rect per tab, spanning the full height of `area`; exactly the
    /// cells [`Widget::render`] paints for that tab (clipped to `area`).
    pub fn hit_rects(&self, area: Rect) -> Vec<Rect> {
        let mut x = area.x.saturating_add(GUTTER);
        self.labels
            .iter()
            .map(|label| {
                let w = text_width(label).saturating_add(PAD * 2);
                let tab = Rect::new(x, area.y, w, area.height);
                x = x.saturating_add(w).saturating_add(GAP);
                tab.intersection(area)
            })
            .collect()
    }

    /// Columns from the left edge of the area to the right edge of the last
    /// tab: the span [`Widget::render`] paints.
    pub fn width(&self) -> u16 {
        self.hit_rects(Rect::new(0, 0, u16::MAX, 1))
            .last()
            .map_or(GUTTER, |tab| tab.right())
    }
}

impl Widget for TabList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let theme = self.theme;
        buf.set_style(area, theme.surface());
        let two_rows = area.height >= 2;
        let indicator = theme.tone(Tone::Brand);

        if two_rows {
            let y = area.bottom() - 1;
            let line = Span::styled(
                "─".repeat(usize::from(area.width)),
                theme.fg(stroke::NEUTRAL_WEAK),
            );
            buf.set_span(area.x, y, &line, area.width);
        }

        for (i, rect) in self.hit_rects(area).into_iter().enumerate() {
            if rect.is_empty() {
                continue;
            }
            let active = i == self.active;
            let style = if active {
                let mut s = Style::default()
                    .fg(indicator.fg_contrast)
                    .add_modifier(Modifier::BOLD);
                if !two_rows {
                    s = s.add_modifier(Modifier::UNDERLINED);
                }
                s
            } else {
                theme.fg(fg::NEUTRAL_SUBTLE)
            };
            let label = Line::from(Span::styled(format!(" {} ", self.labels[i]), style));
            buf.set_line(rect.x, area.y, &label, rect.width);
            if active && two_rows {
                let bar = Span::styled(
                    "━".repeat(usize::from(rect.width)),
                    Style::default().fg(indicator.fg),
                );
                buf.set_span(rect.x, area.bottom() - 1, &bar, rect.width);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::tokens::{bg, brand};
    use crate::ui::seed::widgets::testing::{all_themes, render, row};

    const TABS: [&str; 3] = ["Now", "Lanes", "Graph"];

    #[test]
    fn tablist_marks_active_tab_and_dims_the_rest() {
        for theme in all_themes() {
            let buf = render(TabList::new(theme, &TABS, 1), 30, 1);
            assert_eq!(row(&buf, 0).trim_end(), "   Now   Lanes   Graph");
            let now = &buf[(3, 0)];
            assert_eq!(
                (now.symbol(), now.fg),
                ("N", theme.color(fg::NEUTRAL_SUBTLE))
            );
            let lanes = &buf[(9, 0)];
            assert_eq!(lanes.symbol(), "L");
            assert_eq!(lanes.fg, theme.color(brand::FG_CONTRAST));
            assert!(lanes.modifier.contains(Modifier::BOLD));
            assert!(lanes.modifier.contains(Modifier::UNDERLINED));
            assert!(!now.modifier.contains(Modifier::UNDERLINED));
            assert_eq!(now.bg, theme.color(bg::LAYER_DEFAULT));
        }
    }

    #[test]
    fn tablist_with_two_rows_draws_indicator_bar() {
        let theme = all_themes()[0];
        let buf = render(TabList::new(theme, &TABS, 0), 30, 2);
        let rects = TabList::new(theme, &TABS, 0).hit_rects(Rect::new(0, 0, 30, 2));
        assert_eq!(buf[(rects[0].x, 1)].symbol(), "━");
        assert_eq!(buf[(rects[0].x, 1)].fg, theme.color(brand::FG));
        assert_eq!(buf[(rects[1].x, 1)].symbol(), "─");
        assert_eq!(buf[(rects[1].x, 1)].fg, theme.color(stroke::NEUTRAL_WEAK));
        assert!(
            !buf[(rects[0].x + 1, 0)]
                .modifier
                .contains(Modifier::UNDERLINED)
        );
    }

    #[test]
    fn tablist_hit_rects_match_rendered_geometry() {
        let theme = all_themes()[1];
        let area = Rect::new(4, 2, 30, 1);
        let rects = TabList::new(theme, &TABS, 0).hit_rects(area);
        assert_eq!(
            rects,
            [
                Rect::new(6, 2, 5, 1),
                Rect::new(12, 2, 7, 1),
                Rect::new(20, 2, 7, 1)
            ]
        );
        // Rendering into an offset area writes the label at each rect's x.
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 4));
        TabList::new(theme, &TABS, 0).render(area, &mut buf);
        for (rect, label) in rects.iter().zip(TABS) {
            assert_eq!(buf[(rect.x + 1, rect.y)].symbol(), &label[..1]);
        }
        // Narrow areas clip the rects instead of overflowing.
        let narrow = TabList::new(theme, &TABS, 0).hit_rects(Rect::new(0, 0, 14, 1));
        assert_eq!(narrow[1], Rect::new(8, 0, 6, 1));
        assert!(narrow[2].is_empty());
    }

    #[test]
    fn tablist_width_is_the_painted_span() {
        let theme = all_themes()[0];
        let tabs = TabList::new(theme, &TABS, 0);
        let buf = render(tabs, 40, 1);
        // The last label's trailing pad is a painted space `trim_end` drops.
        let painted = row(&buf, 0).trim_end().chars().count() as u16 + PAD;
        assert_eq!(tabs.width(), painted);
        assert_eq!(tabs.width(), GUTTER + 5 + GAP + 7 + GAP + 7);
    }
}
