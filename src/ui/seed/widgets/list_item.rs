//! SEED list item (`list-item.yaml`): one row — marker, indent, leading,
//! title, detail, right-aligned trailing.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use super::{spans_width, text_width};
use crate::ui::seed::{
    theme::Theme,
    tokens::{bg, brand, fg},
};
use crate::ui::text::truncate;

const MARKER: &str = "▶ ";
const NO_MARKER: &str = "  ";
/// Cells between title and detail.
const GAP: u16 = 2;

#[derive(Clone, Debug)]
pub struct ListItem {
    theme: Theme,
    depth: u16,
    leading: Vec<Span<'static>>,
    title: String,
    detail: String,
    trailing: Vec<Span<'static>>,
    selected: bool,
    flash: bool,
}

impl ListItem {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            depth: 0,
            leading: Vec::new(),
            title: String::new(),
            detail: String::new(),
            trailing: Vec::new(),
            selected: false,
            flash: false,
        }
    }

    /// Tree depth; each level indents two cells after the marker.
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    pub fn leading(mut self, spans: Vec<Span<'static>>) -> Self {
        self.leading = spans;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    pub fn trailing(mut self, spans: Vec<Span<'static>>) -> Self {
        self.trailing = spans;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Highlight the title in brand color (an agent whose status just changed).
    pub fn flash(mut self, flash: bool) -> Self {
        self.flash = flash;
        self
    }
}

impl Widget for ListItem {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let row = Rect::new(area.x, area.y, area.width, 1);
        let theme = self.theme;
        let mut base = theme.fg(fg::NEUTRAL);
        if self.selected {
            base = base.bg(theme.color(bg::NEUTRAL_WEAK));
        }
        buf.set_style(row, base);

        // Trailing is right-aligned and never truncated unless wider than the row.
        let trailing_w = spans_width(&self.trailing).min(row.width);
        let trailing_x = row.right() - trailing_w;
        if trailing_w > 0 {
            buf.set_line(trailing_x, row.y, &Line::from(self.trailing), trailing_w);
        }
        // One cell of air between the left content and the trailing spans.
        let left_limit = if trailing_w > 0 {
            trailing_x.saturating_sub(row.x + 1)
        } else {
            row.width
        };

        let marker = if self.selected {
            Span::styled(MARKER, theme.fg(brand::FG))
        } else {
            Span::raw(NO_MARKER)
        };
        let mut head = vec![marker, Span::raw(" ".repeat(usize::from(self.depth) * 2))];
        if !self.leading.is_empty() {
            head.extend(self.leading);
            head.push(Span::raw(" "));
        }
        let head_w = spans_width(&head);

        let mut title_style = Style::default();
        if self.selected {
            title_style = title_style.add_modifier(Modifier::BOLD);
        }
        if self.flash {
            title_style = title_style.fg(theme.color(brand::FG));
        }
        let title_room = left_limit.saturating_sub(head_w);
        let title = truncate(&self.title, usize::from(title_room));
        let title_w = text_width(&title);

        let mut spans = head;
        spans.push(Span::styled(title, title_style));
        let detail_room = left_limit.saturating_sub(head_w + title_w + GAP);
        if !self.detail.is_empty() && detail_room >= 2 && title_w < title_room {
            spans.push(Span::raw(" ".repeat(usize::from(GAP))));
            spans.push(Span::styled(
                truncate(&self.detail, usize::from(detail_room)),
                theme.fg(fg::NEUTRAL_MUTED),
            ));
        }
        buf.set_line(row.x, row.y, &Line::from(spans), left_limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::tokens::bg;
    use crate::ui::seed::widgets::testing::{all_themes, render, row};

    fn item(theme: Theme) -> ListItem {
        ListItem::new(theme)
            .leading(vec![Span::raw("●")])
            .title("main")
            .detail("reading the repository layout")
            .trailing(vec![Span::raw("12s")])
    }

    #[test]
    fn list_item_selected_has_marker_background_and_bold_title() {
        for theme in all_themes() {
            let buf = render(item(theme).selected(true).depth(1), 40, 1);
            let text = row(&buf, 0);
            assert!(text.starts_with("▶   ● main  reading"), "{text:?}");
            assert!(text.ends_with("12s"), "{text:?}");
            assert_eq!(buf[(0, 0)].fg, theme.color(brand::FG));
            for x in 0..40 {
                assert_eq!(buf[(x, 0)].bg, theme.color(bg::NEUTRAL_WEAK), "x={x}");
            }
            let title = &buf[(6, 0)];
            assert_eq!(title.symbol(), "m");
            assert!(title.modifier.contains(Modifier::BOLD));
            assert_eq!(title.fg, theme.color(fg::NEUTRAL));
        }
    }

    #[test]
    fn list_item_unselected_has_blank_marker_and_no_background() {
        let theme = all_themes()[1];
        let buf = render(item(theme), 40, 1);
        let text = row(&buf, 0);
        assert!(text.starts_with("  ● main  reading"), "{text:?}");
        assert_eq!(buf[(0, 0)].bg, ratatui::style::Color::Reset);
        assert!(!buf[(4, 0)].modifier.contains(Modifier::BOLD));
        let detail_x = text.find("reading").unwrap() as u16;
        assert_eq!(buf[(detail_x, 0)].fg, theme.color(fg::NEUTRAL_MUTED));
    }

    #[test]
    fn list_item_truncates_detail_so_trailing_stays_visible() {
        let theme = all_themes()[0];
        for width in [16u16, 22, 30] {
            let buf = render(item(theme), width, 1);
            let text = row(&buf, 0);
            assert!(text.ends_with("12s"), "w={width} {text:?}");
        }
        let buf = render(item(theme), 24, 1);
        assert_eq!(row(&buf, 0), "  ● main  reading t… 12s");
        // CJK detail is cut by display width, not char count.
        let cjk = ListItem::new(theme)
            .title("a")
            .detail("안녕하세요안녕하세요")
            .trailing(vec![Span::raw("1s")]);
        let buf = render(cjk, 16, 1);
        let text = row(&buf, 0);
        assert!(text.trim_end().ends_with("1s"), "{text:?}");
        assert_eq!((buf[(14, 0)].symbol(), buf[(15, 0)].symbol()), ("1", "s"));
        assert_eq!(buf[(1, 0)].symbol(), " ");
    }

    #[test]
    fn list_item_flash_colors_title_brand() {
        for theme in all_themes() {
            let buf = render(item(theme).flash(true), 40, 1);
            assert_eq!(buf[(4, 0)].symbol(), "m");
            assert_eq!(buf[(4, 0)].fg, theme.color(brand::FG));
        }
    }
}
