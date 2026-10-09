//! Rendering: the screen shell and the three views.
//!
//! The shell is a top bar (who, what, how long, which view), the body (the
//! active view, with the selected agent's detail beside it when there is
//! room), and a hint bar (what needs attention, which keys work here).
//! Overlays (help, session info, a snackbar) draw last.
//!
//! Every color comes from the SEED tokens through [`seed::theme::Theme`],
//! which the frontend owns and passes in. Times come from the app's state —
//! the timeline's `now_reference` (the playhead, or the system clock at a
//! live edge) and the loop's clock — never from the ui reading a clock itself,
//! so a replayed frame is a function of `App` and `Theme`.

pub(crate) mod chrome;
pub mod seed;
#[cfg(all(test, feature = "native"))]
mod snapshots;
pub(crate) mod text;
pub(crate) mod views;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::widgets::{Block, Paragraph};

use crate::state::{App, HitMap, View};
use seed::theme::Theme;
use seed::widgets::Snackbar;

/// Below this the shell cannot lay out its bars and a body; it asks for room.
pub const MIN_WIDTH: u16 = 44;
/// See [`MIN_WIDTH`].
pub const MIN_HEIGHT: u16 = 10;

/// From this width the Now view keeps the selected agent's detail open on
/// its right (40 % of the body).
pub const SIDE_DETAIL_WIDTH: u16 = 100;

/// Horizontal body margin: SEED `spacing-x.global-gutter` (16 px = 2 cells).
const GUTTER: u16 = 2;

/// Render one frame.
pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    app.hit = HitMap::default();
    frame.render_widget(Block::default().style(theme.base()), area);

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = text::truncate("Enlarge the window", area.width as usize);
        let y = area.y + area.height / 2;
        frame.render_widget(
            Paragraph::new(msg).style(theme.base()).centered(),
            Rect::new(area.x, y, area.width, 1),
        );
        return;
    }

    let [top, body, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    chrome::top_bar(frame, top, app, theme);
    render_body(frame, body, app, theme);
    chrome::bottom_bar(frame, bottom, app, theme);

    if app.show_help {
        chrome::help(frame, area, theme);
    }
    if app.show_info {
        chrome::info(frame, area, app, theme);
    }
    if let Some(snack) = &app.snackbar
        && snack.until > app.clock
    {
        let bar = Snackbar::new(*theme, snack.message.clone());
        frame.render_widget(bar, Snackbar::area(body));
    }
}

/// The body: the active view, or the selected agent's detail when it has
/// been opened full-size.
fn render_body(frame: &mut Frame, body: Rect, app: &mut App, theme: &Theme) {
    // A selection can outlive its agent (a backward seek removes agents).
    let selected = app
        .selected_agent_id()
        .filter(|id| app.session.agent(id).is_some());
    let inset = body.inner(Margin::new(GUTTER, 0));

    if app.selection.detail
        && let Some(id) = selected.as_deref()
    {
        views::detail::render(frame, inset, app, theme, id);
        return;
    }
    match app.view {
        View::Now => match selected.as_deref() {
            Some(id) if body.width >= SIDE_DETAIL_WIDTH => {
                // The list keeps the full-row layout (tool chip, tokens) the
                // 60-99 column band shows; the detail takes the rest, at
                // most 40 %.
                let [list, side] = Layout::horizontal([
                    Constraint::Min(views::now::NARROW),
                    Constraint::Percentage(40),
                ])
                .areas(inset);
                views::now::render(frame, list, app, theme);
                let side = Rect {
                    x: side.x + 1,
                    width: side.width.saturating_sub(1),
                    ..side
                };
                views::detail::render(frame, side, app, theme, id);
            }
            _ => views::now::render(frame, inset, app, theme),
        },
        View::Lanes => views::lanes::render(frame, inset, app, theme),
        View::Graph => views::graph::render(frame, body, app, theme),
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use seed::theme::{Depth, Mode};

    /// From 100 columns the detail opens beside the Now list, and the list
    /// still gets the full-row layout (tool or status, tokens) that 60-99
    /// columns show.
    #[test]
    fn side_detail_keeps_the_list_full_width_from_100_columns() {
        let Some(mut app) = snapshots::fixture_app() else {
            return;
        };
        app.select_step(0);
        let theme = Theme::new(Mode::Dark, Depth::TrueColor);
        for width in [100, 104, 140] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 30)).unwrap();
            terminal.draw(|f| draw(f, &mut app, &theme)).unwrap();
            let buf = terminal.backend().buffer();
            let row: String = (0..width).map(|x| buf[(x, 1)].symbol()).collect();
            assert!(row.contains("tok"), "{width} cols: {row}");
            assert!(row.contains('╭'), "{width} cols: the detail is open: {row}");
        }
    }
}
