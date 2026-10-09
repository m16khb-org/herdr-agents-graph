//! The frame around the views: the top bar (who, what, how long, which
//! view), the hint bar (what needs attention, which keys work here), and the
//! help and session-info overlays.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use crate::state::{App, SLOW_TOOL, Transport, View};
use crate::ui::seed::theme::{Theme, Tone};
use crate::ui::seed::tokens::{fg, stroke};
use crate::ui::seed::widgets::{Badge, KeyHint, Spinner, TabList, spans_width, text_width};
use crate::ui::text::{fmt_clock, fmt_cost, fmt_dur, fmt_tokens, truncate};
use crate::ui::views;

/// Tab labels, in [`View::ALL`] order.
const TAB_LABELS: [&str; 3] = [
    View::ALL[0].label(),
    View::ALL[1].label(),
    View::ALL[2].label(),
];

/// The top bar: provider · title, the transport badge with the session's
/// elapsed time, output tokens, recorded cost, and the view tabs. Narrow
/// bars drop the least important parts first (title, cost, tokens).
pub(crate) fn top_bar(frame: &mut Frame, area: Rect, app: &mut App, theme: &Theme) {
    let surface = theme.surface();
    frame.render_widget(Block::default().style(surface), area);

    let active = View::ALL.iter().position(|v| *v == app.view).unwrap_or(0);
    let tabs = TabList::new(*theme, &TAB_LABELS, active);
    let tabs_w = tabs.width().min(area.width);
    let tabs_area = Rect::new(area.right() - tabs_w, area.y, tabs_w, 1);
    for (rect, view) in tabs.hit_rects(tabs_area).into_iter().zip(View::ALL) {
        app.hit.tabs.push((rect, view));
    }
    frame.render_widget(tabs, tabs_area);

    let wall = app.timeline.now_reference();
    let muted = theme.fg(fg::NEUTRAL_MUTED);
    let subtle = theme.fg(fg::NEUTRAL_SUBTLE);

    let provider = app
        .session
        .provider()
        .map(|p| p.name())
        .unwrap_or("agents-graph");
    let title = app
        .session_info
        .title
        .as_deref()
        .or_else(|| app.session.first_prompt())
        .unwrap_or("session");

    // Segments in display order, each with a priority (lower survives longer).
    let mut segments: Vec<(u8, Vec<Span<'static>>)> = Vec::new();
    segments.push((
        0,
        vec![Span::styled(
            format!(" {provider}"),
            theme.fg(fg::NEUTRAL).add_modifier(Modifier::BOLD),
        )],
    ));
    segments.push((
        3,
        vec![
            Span::styled(" · ", subtle),
            Span::styled(title.to_string(), muted),
        ],
    ));

    let mut status = vec![Span::raw("  ")];
    status.extend(transport_badge(app, theme).spans());
    if let Some(elapsed) = app.session.elapsed(wall) {
        let running = app.session.in_flight().next().is_some();
        status.push(Span::raw(" "));
        if running {
            status.extend(Spinner::new(*theme, elapsed.num_seconds().max(0) as u64).spans());
            status.push(Span::raw(" "));
        }
        status.push(Span::styled(fmt_dur(elapsed), muted));
    }
    segments.push((1, status));

    let tokens = app.session.output_tokens();
    if tokens > 0 {
        segments.push((
            2,
            vec![Span::styled(
                format!("  {} tok", fmt_tokens(tokens)),
                subtle,
            )],
        ));
    }
    if let Some(cost) = app.session.cost_usd() {
        segments.push((
            2,
            vec![Span::styled(format!("  {}", fmt_cost(cost)), subtle)],
        ));
    }

    let budget = area.width.saturating_sub(tabs_w + 1);
    let line = fit_segments(segments, budget, title, muted);
    frame.render_widget(
        Paragraph::new(line).style(surface),
        Rect::new(area.x, area.y, budget, 1),
    );
}

/// Drop segments by priority (highest number first) until the line fits;
/// the title segment is truncated before it is dropped.
fn fit_segments(
    mut segments: Vec<(u8, Vec<Span<'static>>)>,
    budget: u16,
    title: &str,
    title_style: Style,
) -> Line<'static> {
    let total = |segs: &[(u8, Vec<Span<'static>>)]| -> u16 {
        segs.iter().map(|(_, s)| spans_width(s)).sum()
    };
    // First shrink the title, which is the only free-length segment.
    let over = total(&segments).saturating_sub(budget);
    if over > 0
        && let Some((_, spans)) = segments.iter_mut().find(|(p, _)| *p == 3)
    {
        let room = text_width(title).saturating_sub(over);
        if room >= 8 {
            spans[1] = Span::styled(truncate(title, room as usize), title_style);
        }
    }
    for drop in [3u8, 2] {
        if total(&segments) <= budget {
            break;
        }
        // Drop the lowest-priority segments from the right end first.
        while total(&segments) > budget {
            match segments.iter().rposition(|(p, _)| *p == drop) {
                Some(i) => {
                    segments.remove(i);
                }
                None => break,
            }
        }
    }
    Line::from(
        segments
            .into_iter()
            .flat_map(|(_, s)| s)
            .collect::<Vec<_>>(),
    )
}

/// The emergent transport state as a badge. Away from the live edge the
/// badge says where the playhead is. Its words and glyphs stay clear of the
/// agent statuses (`● ◌ ✓ ✗ ■`, `active`/`idle`/`stopped`), so the top bar
/// never seems to contradict an agent row.
fn transport_badge(app: &App, theme: &Theme) -> Badge {
    let at = app
        .timeline
        .cursor
        .map(|t| fmt_clock(t, app.utc_offset))
        .unwrap_or_default();
    let n = app.timeline.items.len();
    match app.transport_at(app.clock) {
        Transport::Live => Badge::new(*theme, Tone::Positive, "live").glyph('◉'),
        Transport::Playing => Badge::new(
            *theme,
            Tone::Informative,
            format!("replay {}/{n} · {at}", app.timeline.folded),
        )
        .glyph('▶'),
        Transport::Paused => Badge::new(*theme, Tone::Warning, format!("paused · {at}")).glyph('⏸'),
        Transport::History => Badge::new(*theme, Tone::Neutral, format!("past · {at}")).glyph('⏮'),
        Transport::Idle => Badge::new(*theme, Tone::Neutral, "quiet").glyph('◦'),
    }
}

/// The hint bar: what needs attention on the left (failures, slow tools, a
/// tailer error), this view's keys on the right. When both do not fit, keys
/// go first from the right end.
pub(crate) fn bottom_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let surface = theme.surface();
    frame.render_widget(Block::default().style(surface), area);

    let wall = app.timeline.now_reference();
    let attention = views::attention(&app.session, wall);
    let mut left: Vec<Span<'static>> = vec![Span::raw(" ")];
    let critical = theme.tone(Tone::Critical);
    let warning = theme.tone(Tone::Warning);
    if !attention.failed.is_empty() {
        left.push(Span::styled(
            format!("✗ {} failed", attention.failed.len()),
            Style::default()
                .fg(critical.fg)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if attention.slow > 0 {
        if left.len() > 1 {
            left.push(Span::styled(" · ", theme.fg(fg::NEUTRAL_SUBTLE)));
        }
        left.push(Span::styled(
            format!(
                "⏳ {} tool{} over {}",
                attention.slow,
                plural(attention.slow),
                fmt_dur(SLOW_TOOL)
            ),
            Style::default().fg(warning.fg),
        ));
    }
    if let Some(err) = &app.last_error {
        if left.len() > 1 {
            left.push(Span::styled(" · ", theme.fg(fg::NEUTRAL_SUBTLE)));
        }
        left.push(Span::styled(
            format!("⚠ {}", truncate(err, 40)),
            Style::default().fg(critical.fg),
        ));
    }
    let left_w = spans_width(&left);

    let mut hints: Vec<Span<'static>> = Vec::new();
    let budget = area.width.saturating_sub(left_w + 1);
    for (key, label) in key_hints(app) {
        let hint = KeyHint::new(*theme, key, label);
        let gap = u16::from(!hints.is_empty());
        if spans_width(&hints) + gap + hint.width() + 1 > budget {
            break;
        }
        if gap == 1 {
            hints.push(Span::raw(" "));
        }
        hints.extend(hint.spans());
    }
    hints.push(Span::raw(" "));

    let hints_w = spans_width(&hints).min(area.width);
    frame.render_widget(Paragraph::new(Line::from(left)).style(surface), area);
    frame.render_widget(
        Paragraph::new(Line::from(hints)).style(surface),
        Rect::new(area.right() - hints_w, area.y, hints_w, 1),
    );
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// The keys the hint bar offers for the current view, most useful first.
fn key_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    let mut keys = match app.view {
        View::Now => vec![("j/k", "move"), ("enter", "expand"), ("tab", "view")],
        View::Lanes => vec![
            ("←/→", "cursor"),
            ("space", "play"),
            ("z", "gaps"),
            ("tab", "view"),
        ],
        View::Graph => vec![
            ("j/k", "select"),
            ("hjkl", "pan"),
            ("+/-", "zoom"),
            ("tab", "view"),
        ],
    };
    if app.selection.detail {
        keys = vec![("j/k", "scroll"), ("esc", "back")];
    }
    keys.push(("?", "help"));
    keys
}

/// A centered popup rect of at most `w` × `h` within `area`.
fn popup(area: Rect, w: u16, h: u16) -> Rect {
    let w = area.width.min(w);
    let h = area.height.min(h);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

/// A floating box with a centered title and a closing hint.
fn floating_block(theme: &Theme, title: &str, footer: &str) -> Block<'static> {
    let floating = theme.floating();
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(floating.patch(theme.fg(stroke::NEUTRAL_CONTRAST)))
        .style(floating)
        .title_top(
            Line::from(format!(" {title} "))
                .centered()
                .style(floating.add_modifier(Modifier::BOLD)),
        )
        .title_bottom(
            Line::from(format!(" {footer} "))
                .centered()
                .style(floating.patch(theme.fg(fg::NEUTRAL_SUBTLE))),
        )
}

/// The full key reference.
pub(crate) fn help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let quit = if cfg!(target_arch = "wasm32") {
        "close the browser tab"
    } else {
        "q · ctrl-c"
    };
    let rows: [(&str, &str); 17] = [
        ("tab 1 2 3", "switch view: now · lanes · graph"),
        ("j k ↑ ↓", "select agent (scroll inside the detail)"),
        ("enter", "unfold done · expand the row · full detail"),
        ("esc", "back: close detail, collapse, fold, deselect"),
        ("space", "play / pause"),
        ("[ ]", "previous / next prompt"),
        ("G end", "back to live"),
        ("← →", "lanes: move the cursor · graph: select sideways"),
        ("z", "lanes: fold idle gaps"),
        ("h l H J K L", "graph: pan"),
        ("+ - 0", "graph: zoom in / out / reset"),
        ("map", "whole graph; the frame is what you see"),
        ("f o r", "follow activity · overview · rearrange"),
        ("s", "pacing: skip idle gaps while replaying"),
        ("i", "session details"),
        ("pgup pgdn", "scroll the detail list"),
        ("quit", quit),
    ];
    let rect = popup(area, 64, rows.len() as u16 + 4);
    if rect.width < 30 || rect.height < 8 {
        return;
    }
    frame.render_widget(Clear, rect);
    let floating = theme.floating();
    let key = floating
        .patch(theme.fg(fg::NEUTRAL))
        .add_modifier(Modifier::BOLD);
    let txt = floating.patch(theme.fg(fg::NEUTRAL_MUTED));
    let mut lines = vec![Line::from("")];
    for (k, what) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!(" {k:<12} "), key),
            Span::styled(what.to_string(), txt),
        ]));
    }
    let block = floating_block(theme, "agents-graph — keys", "? or esc to close");
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

/// The untimed session metadata kept off the timeline: title, the
/// provider's labelled fields, and counts.
pub(crate) fn info(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let rect = popup(area, 56, 12);
    if rect.width < 24 || rect.height < 7 {
        return;
    }
    frame.render_widget(Clear, rect);
    let floating = theme.floating();
    let key = floating
        .patch(theme.fg(fg::NEUTRAL_SUBTLE))
        .add_modifier(Modifier::BOLD);
    let txt = floating.patch(theme.fg(fg::NEUTRAL));
    let info = &app.session_info;
    let value_w = (rect.width as usize).saturating_sub(13);
    let row = |label: &str, value: &str| {
        Line::from(vec![
            Span::styled(format!(" {label:<9} "), key),
            Span::styled(truncate(value, value_w), txt),
        ])
    };
    let mut lines = vec![
        Line::from(""),
        row("title", info.title.as_deref().unwrap_or("—")),
    ];
    for (label, value) in &info.fields {
        lines.push(row(label, value));
    }
    if !info.tallies.is_empty() {
        let tally = info
            .tallies
            .iter()
            .map(|(label, n)| format!("{n} {label}"))
            .collect::<Vec<_>>()
            .join(" · ");
        lines.push(row("count", &tally));
    }
    let block = floating_block(theme, "session", "i or esc to close");
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}
