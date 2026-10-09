//! The time-lane view: one row per agent, time across, a tool call drawn as a
//! bar over the columns it covers. Idle stretches nobody worked through fold
//! into a three-column marker so a long pause does not squeeze the activity.

use chrono::{DateTime, Duration, Utc};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::state::session::{AgentInfo, ToolState};
use crate::state::{App, LaneAxis};
use crate::ui::seed::theme::{Theme, status_tone, tool_tone};
use crate::ui::seed::tokens::{bg, brand, fg};
use crate::ui::text::{fmt_clock, fmt_dur, fmt_hm, truncate, width};

/// Widest the agent-name column gets.
const NAME_MAX: u16 = 14;
/// Columns a folded gap takes: `┆`, a middle one, `┆`.
const FOLD_COLS: usize = 3;
/// Columns between two time labels on the axis.
const LABEL_STEP: usize = 12;
const EMPTY: &str = "No tool calls recorded";

/// One body column of the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Col {
    /// Epoch milliseconds at the column's left edge. A fold marker's columns
    /// all carry the gap's start.
    t: i64,
    /// `Some` on the three columns of a folded gap.
    fold: Option<Fold>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fold {
    /// How long the folded gap lasted.
    ms: i64,
    /// Which of the marker's columns this is, `0..FOLD_COLS`.
    pos: usize,
}

/// Lay out `width` columns over `start..end`. `busy` is the merged, ordered
/// list of intervals in which something happened. With `fold`, a stretch
/// between activity longer than a quarter of the span becomes a marker and
/// the remaining columns re-scale to the shorter span. The gap after the last
/// activity is never folded.
fn axis(busy: &[(i64, i64)], start: i64, end: i64, width: usize, fold: bool) -> Vec<Col> {
    let span = (end - start).max(1);
    let mut gaps: Vec<(i64, i64)> = Vec::new();
    if fold && !busy.is_empty() {
        let mut from = start;
        for &(s, e) in busy {
            if s > from && (s - from).saturating_mul(4) > span {
                gaps.push((from, s - from));
            }
            from = from.max(e);
        }
        // Folding must leave most of the width to the activity.
        if gaps.len() * FOLD_COLS * 2 > width {
            gaps.clear();
        }
    }
    let folded: i64 = gaps.iter().map(|g| g.1).sum();
    let body = (width - gaps.len() * FOLD_COLS).max(1);
    let shown = (span - folded).max(1);

    // Where each gap sits on the folded axis.
    let mut before = 0;
    let at: Vec<i64> = gaps
        .iter()
        .map(|&(s, d)| {
            let c = s - start - before;
            before += d;
            c
        })
        .collect();

    let mut cols = Vec::with_capacity(width);
    let mut next = 0;
    let marker = |cols: &mut Vec<Col>, (s, d): (i64, i64)| {
        for pos in 0..FOLD_COLS {
            cols.push(Col {
                t: s,
                fold: Some(Fold { ms: d, pos }),
            });
        }
    };
    // The column right after a marker starts where the gap ends, so what
    // happened first after the pause lands in it, not before the marker.
    let mut pinned: Option<i64> = None;
    for j in 0..body {
        let c = (j as i64).saturating_mul(shown) / body as i64;
        while next < gaps.len() && at[next] <= c {
            marker(&mut cols, gaps[next]);
            pinned = Some(gaps[next].0 + gaps[next].1);
            next += 1;
        }
        let skipped: i64 = gaps[..next].iter().map(|g| g.1).sum();
        let t = start + c + skipped;
        cols.push(Col {
            t: pinned.take().map_or(t, |gap_end| gap_end.min(t)),
            fold: None,
        });
    }
    for g in &gaps[next..] {
        marker(&mut cols, *g);
    }
    cols
}

/// The body column holding `t`; a fold marker never answers (the column
/// before it does).
fn col_of(cols: &[Col], t: i64) -> usize {
    let mut i = cols.partition_point(|c| c.t <= t).saturating_sub(1);
    while i > 0 && cols[i].fold.is_some() {
        i -= 1;
    }
    i
}

/// First index in `0..n` for which `pred` is false (`pred` is monotone).
fn partition(n: usize, mut pred: impl FnMut(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The calls that can show on `from..=to`, out of `n` start-ordered ones:
/// binary-search the first call starting after `to`, and the first starting
/// at `from`, then walk back over earlier calls still running at `from`.
fn visible_range(
    n: usize,
    start_of: impl Fn(usize) -> i64,
    end_of: impl Fn(usize) -> i64,
    from: i64,
    to: i64,
) -> std::ops::Range<usize> {
    let hi = partition(n, |i| start_of(i) <= to);
    let mut lo = partition(hi, |i| start_of(i) < from);
    while lo > 0 && end_of(lo - 1) >= from {
        lo -= 1;
    }
    lo..hi
}

fn ms(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

fn start_ms(call: &crate::state::session::ToolCallInfo) -> i64 {
    call.ts.map_or(i64::MIN, ms)
}

fn end_ms(call: &crate::state::session::ToolCallInfo, end: i64, terminal: bool) -> i64 {
    match call.end_ts {
        Some(t) => ms(t),
        None if call.state == ToolState::Pending && !terminal => end,
        None => start_ms(call),
    }
}

/// Every interval something happened in, merged and ordered: tool calls (a
/// pending one runs to `end`) and the moments each agent was first and last
/// heard from. Calls are start-ordered per agent, so this merges the agents'
/// lists rather than sorting them all.
fn busy_intervals(agents: &[&AgentInfo], end: i64) -> Vec<(i64, i64)> {
    let head = |a: &AgentInfo, i: usize| -> Option<(i64, i64)> {
        let mut i = i;
        loop {
            let c = a.tool_call(i)?;
            if c.ts.is_some() {
                let s = start_ms(c);
                return Some((s, end_ms(c, end, a.is_terminal()).max(s).min(end.max(s))));
            }
            i += 1;
        }
    };
    let mut points: Vec<i64> = agents
        .iter()
        .flat_map(|a| [a.first_ts, a.last_ts])
        .flatten()
        .map(ms)
        .collect();
    points.sort_unstable();
    let mut p = 0;
    let mut pos = vec![0usize; agents.len()];
    let mut heads: Vec<Option<(i64, i64)>> = agents.iter().map(|a| head(a, 0)).collect();
    let mut out: Vec<(i64, i64)> = Vec::new();
    loop {
        let mut best: Option<(i64, usize)> = None;
        for (k, h) in heads.iter().enumerate() {
            if let Some((s, _)) = h
                && best.is_none_or(|(b, _)| *s < b)
            {
                best = Some((*s, k));
            }
        }
        let next = match (best, points.get(p)) {
            (Some((s, _)), Some(&pt)) if pt < s => {
                p += 1;
                (pt, pt)
            }
            (Some((_, k)), _) => {
                let iv = heads[k].take().unwrap_or((0, 0));
                pos[k] += 1;
                heads[k] = head(agents[k], pos[k]);
                iv
            }
            (None, Some(&pt)) => {
                p += 1;
                (pt, pt)
            }
            (None, None) => break,
        };
        match out.last_mut() {
            Some(last) if next.0 <= last.1 => last.1 = last.1.max(next.1),
            _ => out.push(next),
        }
    }
    out
}

/// `12m` for a folded gap: whole minutes once there are some.
fn gap_label(ms: i64) -> String {
    let d = fmt_dur(Duration::milliseconds(ms));
    match (d.find('m'), d.ends_with('s')) {
        (Some(i), true) => d[..=i].to_string(),
        _ => d,
    }
}

pub(crate) fn render(frame: &mut Frame, area: Rect, app: &mut App, theme: &Theme) {
    let name_w = NAME_MAX.min(area.width / 5);
    if area.height < 3 || area.width <= name_w + 2 {
        app.hit.lane_axis = None;
        return;
    }
    let rows = app.agent_rows();
    let buf = frame.buffer_mut();

    let plan = {
        let agents: Vec<&AgentInfo> = rows
            .iter()
            .filter_map(|(id, _)| app.session.agent(id))
            .collect();
        let first_call = agents
            .iter()
            .filter_map(|a| a.tool_call(0).and_then(|c| c.ts))
            .min();
        let start = agents
            .iter()
            .filter_map(|a| a.first_ts)
            .chain(first_call)
            .min()
            .map(ms);
        let has_calls = agents.iter().any(|a| a.tool_calls().len() > 0);
        match start.filter(|_| has_calls) {
            None => None,
            Some(start) => {
                let last = agents
                    .iter()
                    .filter_map(|a| a.last_ts)
                    .map(ms)
                    .max()
                    .unwrap_or(start);
                let end = app.timeline.now_reference().map_or(last, ms).max(start + 1);
                Some((start, end, agents))
            }
        }
    };
    let Some((start, end, agents)) = plan else {
        let w = width(EMPTY) as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height / 2;
        buf.set_stringn(x, y, EMPTY, area.width as usize, theme.fg(fg::PLACEHOLDER));
        app.hit.lane_axis = None;
        return;
    };

    let body_x = area.x + name_w + 1;
    let body_w = usize::from(area.width - name_w - 1);
    let lanes_h = usize::from(area.height - 2);
    let visible = rows.len().min(lanes_h);

    // Keep the selected lane on screen.
    let selected_idx = app
        .selection
        .agent
        .as_deref()
        .and_then(|s| rows.iter().position(|(id, _)| id == s));
    let mut scroll = app.now_offset.min(rows.len() - visible);
    if let Some(i) = selected_idx {
        if i < scroll {
            scroll = i;
        } else if i >= scroll + visible {
            scroll = i + 1 - visible;
        }
    }
    app.now_offset = scroll;

    let offset = app.utc_offset;
    let follow = app.timeline.follow_head;
    let cursor_ms = app.timeline.cursor.map_or(end, ms).clamp(start, end);
    let fold = app.fold_gaps;

    let (cols, ms_per_col, hit_rows) = {
        let busy = busy_intervals(&agents, end);
        let cols = axis(&busy, start, end, body_w, fold);
        let ms_per_col = ((end - start) / body_w as i64).max(1);

        let edge = if follow {
            "now".to_string()
        } else {
            fmt_hm(from_ms(cursor_ms), offset)
        };
        draw_axis(
            buf, area.x, area.y, name_w, body_x, &cols, theme, offset, &edge,
        );

        let mut hit_rows = Vec::with_capacity(visible);
        for (n, (id, _)) in rows.iter().skip(scroll).take(visible).enumerate() {
            let Some(agent) = app.session.agent(id) else {
                continue;
            };
            let y = area.y + 1 + n as u16;
            let selected = selected_idx == Some(scroll + n);
            let row = Rect::new(area.x, y, area.width, 1);
            if selected {
                buf.set_style(row, theme.bg(bg::NEUTRAL_WEAK));
            }
            draw_name(buf, Rect::new(area.x, y, name_w, 1), agent, selected, theme);
            draw_calls(buf, body_x, y, &cols, agent, end, theme);
            hit_rows.push((Rect::new(area.x, y, name_w, 1), id.clone()));
        }
        (cols, ms_per_col, hit_rows)
    };

    // The cursor: a vertical line through the lanes, unless riding the edge.
    let cur_col = col_of(&cols, cursor_ms);
    if !follow {
        let style = theme.fg(brand::FG);
        for n in 0..visible as u16 {
            let cell = &mut buf[(body_x + cur_col as u16, area.y + 1 + n)];
            cell.set_char('│').set_style(style);
        }
    }
    let footer = if follow {
        "live".to_string()
    } else {
        format!("cursor {}", fmt_clock(from_ms(cursor_ms), offset))
    };
    draw_footer(
        buf,
        body_x,
        body_w,
        area.y + 1 + visible as u16,
        cur_col,
        &footer,
        follow,
        theme,
    );

    app.hit.rows.extend(hit_rows);
    app.hit.lane_axis = Some(LaneAxis {
        area: Rect::new(body_x, area.y, body_w as u16, 1 + visible as u16),
        columns: cols.iter().map(|c| from_ms(c.t)).collect(),
        ms_per_col,
    });
}

fn from_ms(t: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(t).unwrap_or_default()
}

/// Row 0: `Time`, then `─` with an `HH:MM` label every few columns, fold
/// markers labelled with how long they hide, and the right end saying where
/// the view stands.
#[allow(clippy::too_many_arguments)]
fn draw_axis(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    name_w: u16,
    body_x: u16,
    cols: &[Col],
    theme: &Theme,
    offset: chrono::FixedOffset,
    label: &str,
) {
    let subtle = theme.fg(fg::NEUTRAL_SUBTLE);
    let muted = theme.fg(fg::NEUTRAL_MUTED);
    buf.set_stringn(x, y, "Time", usize::from(name_w), subtle);
    let w = cols.len();
    for i in 0..w {
        buf[(body_x + i as u16, y)].set_char('─').set_style(subtle);
    }
    let label_w = width(label);
    let right_at = w.saturating_sub(label_w);
    let mut prev = String::new();
    for i in (0..w).step_by(LABEL_STEP) {
        let text = fmt_hm(from_ms(cols[i].t), offset);
        if text == prev {
            continue;
        }
        prev.clone_from(&text);
        let near_fold = cols[i.saturating_sub(3)..(i + 3).min(w)]
            .iter()
            .any(|c| c.fold.is_some());
        if cols[i].fold.is_none() && !near_fold && i + width(&text) < right_at {
            buf.set_stringn(body_x + i as u16, y, &text, w - i, muted);
        }
    }
    if w > label_w + 6 {
        buf.set_stringn(body_x + right_at as u16, y, label, w - right_at, muted);
    }
    for (i, c) in cols.iter().enumerate() {
        if let Some(Fold { ms, pos: 1 }) = c.fold {
            let text = format!("┆{}┆", gap_label(ms));
            let len = width(&text);
            let at = (i + 1).saturating_sub(len / 2).min(w.saturating_sub(len));
            buf.set_stringn(body_x + at as u16, y, &text, w - at, subtle);
        }
    }
}

/// The lane's left column: selection marker, status glyph in its tone, name.
fn draw_name(buf: &mut Buffer, rect: Rect, agent: &AgentInfo, selected: bool, theme: &Theme) {
    let (marker, name_style) = if selected {
        (
            "▶",
            theme.fg(brand::FG_CONTRAST).add_modifier(Modifier::BOLD),
        )
    } else {
        (" ", theme.fg(fg::NEUTRAL))
    };
    let w = usize::from(rect.width);
    buf.set_stringn(
        rect.x,
        rect.y,
        marker,
        w,
        if selected {
            theme.fg(brand::FG)
        } else {
            Style::default()
        },
    );
    if w < 3 {
        return;
    }
    let glyph = agent.status.glyph().to_string();
    buf.set_stringn(
        rect.x + 1,
        rect.y,
        &glyph,
        w - 1,
        Style::default().fg(theme.tone(status_tone(agent.status)).fg),
    );
    let name = truncate(agent.display_name(), w.saturating_sub(3));
    buf.set_stringn(rect.x + 3, rect.y, &name, w - 3, name_style);
}

/// One lane's bars.
fn draw_calls(
    buf: &mut Buffer,
    body_x: u16,
    y: u16,
    cols: &[Col],
    agent: &AgentInfo,
    end: i64,
    theme: &Theme,
) {
    let w = cols.len();
    let Some(first) = cols.first() else {
        return;
    };
    let terminal = agent.is_terminal();
    let n = agent.tool_calls().len();
    let range = visible_range(
        n,
        |i| agent.tool_call(i).map_or(i64::MAX, start_ms),
        |i| {
            agent
                .tool_call(i)
                .map_or(i64::MIN, |c| end_ms(c, end, terminal))
        },
        first.t,
        end,
    );
    let rail = theme.fg(fg::NEUTRAL_SUBTLE);
    // Fold markers show as a dotted rule down the lane.
    for (i, c) in cols.iter().enumerate() {
        if matches!(c.fold, Some(Fold { pos: 0 | 2, .. })) {
            buf[(body_x + i as u16, y)].set_char('┆').set_style(rail);
        }
    }
    // Indexed, not `skip(range.start)`, which walks every call before the
    // visible range.
    for call in range.clone().filter_map(|i| agent.tool_call(i)) {
        let Some(ts) = call.ts else { continue };
        let s = ms(ts);
        let e = end_ms(call, end, terminal).clamp(s, end.max(s));
        let (c0, c1) = (col_of(cols, s), col_of(cols, e));
        let style = Style::default().fg(theme.tone(tool_tone(call.state)).fg);
        let running = call.state == ToolState::Pending && !terminal;
        for c in c0..=c1.min(w - 1) {
            if cols[c].fold.is_some() {
                continue;
            }
            let ch = match call.state {
                _ if running && c == c1 => '▮',
                ToolState::Err if c == c1 => '✗',
                _ => '━',
            };
            buf[(body_x + c as u16, y)].set_char(ch).set_style(style);
        }
        let cells = c1 + 1 - c0;
        let name_w = width(&call.name);
        if cells >= name_w + 2 {
            buf.set_stringn(body_x + c0 as u16 + 1, y, &call.name, name_w, style);
        }
    }
}

/// Under the lanes: where the cursor is, pointing up at its column; `live`
/// (and the arrow) at the right edge when riding it.
#[allow(clippy::too_many_arguments)]
fn draw_footer(
    buf: &mut Buffer,
    body_x: u16,
    body_w: usize,
    y: u16,
    cursor_col: usize,
    text: &str,
    live: bool,
    theme: &Theme,
) {
    let style = theme.fg(fg::NEUTRAL_SUBTLE);
    let len = width(text) + 2;
    if len > body_w {
        return;
    }
    let col = if live {
        body_w - 1
    } else {
        cursor_col.min(body_w - 1)
    };
    let (at, line) = if !live && col + len <= body_w {
        (col, format!("▲ {text}"))
    } else {
        ((col + 1).saturating_sub(len), format!("{text} ▲"))
    };
    buf.set_stringn(body_x + at as u16, y, &line, body_w - at, style);
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use chrono::FixedOffset;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::fact::{AgentKind, Fact, FactKind, Outcome, Statement};
    use crate::state::Mode;
    use crate::state::session::MAIN_ID;
    use crate::tailer::{ReplayItem, UiEvent};
    use crate::ui::seed::theme::{Depth, Mode as ThemeMode};
    use crate::ui::seed::widgets::testing::{all_themes, row};

    fn at(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_780_000_000 + s, 0).unwrap()
    }

    fn fact(agent: &str, s: i64, kind: FactKind) -> Fact {
        Fact {
            agent: Some(agent.into()),
            ts: Some(at(s)),
            kind,
        }
    }

    fn born(s: i64) -> Fact {
        fact(
            MAIN_ID,
            s,
            FactKind::Agent {
                kind: AgentKind::Main,
                parent: None,
                agent_type: Some("main".into()),
                description: None,
                spawned_by: None,
                interactive: true,
            },
        )
    }

    fn start(call: &str, s: i64) -> Fact {
        fact(
            MAIN_ID,
            s,
            FactKind::ToolStart {
                call: call.into(),
                name: "Bash".into(),
                summary: None,
                intent: None,
            },
        )
    }

    fn done(call: &str, s: i64, outcome: Outcome) -> Fact {
        fact(
            MAIN_ID,
            s,
            FactKind::ToolEnd {
                call: call.into(),
                outcome,
            },
        )
    }

    fn activity(s: i64) -> Fact {
        fact(MAIN_ID, s, FactKind::Activity)
    }

    /// A replay of the main agent's facts, folded to its end.
    fn app_of(items: Vec<(i64, Vec<Fact>)>) -> App {
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items: items
                .into_iter()
                .map(|(s, facts)| {
                    ReplayItem::new(Statement {
                        at: Some(at(s)),
                        facts,
                    })
                })
                .collect(),
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        app.utc_offset = FixedOffset::east_opt(0).unwrap();
        app
    }

    /// Two quick calls ten minutes apart, then a moment of activity.
    fn gappy() -> App {
        app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (5, vec![done("a", 5, Outcome::Ok)]),
            (600, vec![start("b", 600)]),
            (605, vec![done("b", 605, Outcome::Err)]),
            (700, vec![activity(700)]),
        ])
    }

    fn draw(app: &mut App, theme: &Theme, w: u16, h: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| render(f, f.area(), app, theme)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn dark() -> Theme {
        Theme::new(ThemeMode::Dark, Depth::TrueColor)
    }

    fn folds(cols: &[Col]) -> usize {
        cols.iter().filter(|c| c.fold.is_some()).count()
    }

    #[test]
    fn lane_columns_cover_session_span() {
        let cols = axis(&[(0, 100_000)], 0, 100_000, 50, true);
        assert_eq!(cols.len(), 50);
        assert_eq!(cols[0].t, 0);
        assert!(cols.windows(2).all(|w| w[0].t < w[1].t));
        assert_eq!(
            col_of(&cols, 100_000),
            49,
            "the end falls in the last column"
        );
        assert_eq!(col_of(&cols, 50_000), 25);

        // And drawn: one column per body cell, from the first fact to now.
        let mut app = gappy();
        app.fold_gaps = false;
        draw(&mut app, &dark(), 80, 12);
        let hit = app.hit.lane_axis.clone().unwrap();
        assert_eq!(hit.columns.len(), usize::from(hit.area.width));
        assert_eq!(hit.columns[0], at(0));
        assert!(*hit.columns.last().unwrap() < at(700));
        assert!(*hit.columns.last().unwrap() > at(680));
        assert_eq!(hit.ms_per_col, 700_000 / 65);
    }

    #[test]
    fn running_call_reaches_now_edge() {
        let mut app = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (10, vec![done("a", 10, Outcome::Ok), start("b", 20)]),
            (100, vec![activity(100)]),
        ]);
        let theme = dark();
        let buf = draw(&mut app, &theme, 80, 12);
        let last = 14 + 65;
        // Lane 1 is the main agent; its running call ends in `▮` on the edge.
        assert_eq!(buf[(last, 1)].symbol(), "▮");
        assert_eq!(
            buf[(last, 1)].fg,
            theme.tone(tool_tone(ToolState::Pending)).fg
        );
        assert_eq!(buf[(last - 1, 1)].symbol(), "━");
        // The finished call is quiet, and no cell sits past the edge.
        assert_eq!(buf[(15, 1)].fg, theme.tone(tool_tone(ToolState::Ok)).fg);
        assert_eq!(buf[(last, 1)].symbol(), "▮");
    }

    #[test]
    fn errored_call_ends_in_a_cross_ok_call_does_not() {
        let mut app = gappy();
        app.fold_gaps = false;
        let buf = draw(&mut app, &dark(), 80, 12);
        let line = row(&buf, 1);
        assert_eq!(line.matches('✗').count(), 1, "{line}");
        let x = (0..80).find(|&x| buf[(x, 1)].symbol() == "✗").unwrap();
        assert_eq!(buf[(x, 1)].fg, dark().tone(tool_tone(ToolState::Err)).fg);
        let mut ok = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (5, vec![done("a", 5, Outcome::Ok)]),
            (100, vec![activity(100)]),
        ]);
        let buf = draw(&mut ok, &dark(), 80, 12);
        assert!(!row(&buf, 1).contains('✗'));
    }

    #[test]
    fn unanswered_call_of_terminal_agent_draws_no_running_marker() {
        let mut app = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (
                10,
                vec![fact(
                    MAIN_ID,
                    10,
                    FactKind::Ended(crate::fact::AgentStatus::Failed),
                )],
            ),
            (100, vec![activity(100)]),
        ]);
        assert!(app.session.agent(MAIN_ID).unwrap().is_terminal());
        let buf = draw(&mut app, &dark(), 80, 12);
        assert!(!row(&buf, 1).contains('▮'), "{}", row(&buf, 1));
    }

    #[test]
    fn visible_slice_uses_binary_search() {
        let n = 10_000usize;
        let compares = Cell::new(0usize);
        let start_of = |i: usize| {
            compares.set(compares.get() + 1);
            (i as i64) * 10
        };
        let end_of = |i: usize| (i as i64) * 10 + 5;
        let r = visible_range(n, start_of, end_of, 0, 50_000);
        assert_eq!(r, 0..5_001);
        assert!(compares.get() <= 40, "{} comparisons", compares.get());

        compares.set(0);
        let r = visible_range(n, start_of, end_of, 0, i64::MAX);
        assert_eq!(r, 0..n);
        assert!(compares.get() <= 40, "{} comparisons", compares.get());

        // A window further in starts where the calls do, plus the one still running.
        let r = visible_range(n, start_of, |i| (i as i64) * 10 + 25, 50_000, 50_100);
        assert_eq!(r, 4_998..5_011);
    }

    #[test]
    fn idle_gap_over_quarter_width_folds_to_three_columns() {
        let busy = [(0, 10_000), (60_000, 100_000)];
        let cols = axis(&busy, 0, 100_000, 40, true);
        assert_eq!(cols.len(), 40);
        assert_eq!(folds(&cols), 3, "exactly one three-column marker");
        let marker: Vec<usize> = cols
            .iter()
            .enumerate()
            .filter(|(_, c)| c.fold.is_some())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(marker[2] - marker[0], 2, "the marker is contiguous");
        assert!(cols[marker[0]..=marker[2]].iter().all(|c| c.t == 10_000));
        assert_eq!(cols[marker[1]].fold.unwrap().ms, 50_000);
        // The 50 s gap is half the span, so the rest re-scales to 50 s over 37 columns.
        assert_eq!(
            cols[marker[2] + 1].t,
            60_000,
            "the column after starts where the gap ends"
        );
        assert!(cols.windows(2).all(|w| w[0].t <= w[1].t));

        // A gap of exactly a quarter stays.
        let cols = axis(&[(0, 10_000), (35_000, 100_000)], 0, 100_000, 40, true);
        assert_eq!(folds(&cols), 0);

        // Drawn, the marker names how long it hides.
        let mut app = gappy();
        let buf = draw(&mut app, &dark(), 80, 12);
        assert!(row(&buf, 0).contains("┆9m┆"), "{}", row(&buf, 0));
        let hit = app.hit.lane_axis.clone().unwrap();
        assert_eq!(hit.columns.len(), 65);
    }

    #[test]
    fn running_tool_span_is_never_folded() {
        // The same gap, covered by a call that has not finished.
        let busy = [(0, 10_000), (10_000, 100_000)];
        assert_eq!(folds(&axis(&busy, 0, 100_000, 40, true)), 0);

        let mut app = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (5, vec![done("a", 5, Outcome::Ok), start("b", 20)]),
            (700, vec![activity(700)]),
        ]);
        let buf = draw(&mut app, &dark(), 80, 12);
        assert!(!row(&buf, 0).contains('┆'), "{}", row(&buf, 0));

        // Let it finish and the idle stretch before the last activity folds.
        let mut app = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (5, vec![done("a", 5, Outcome::Ok), start("b", 20)]),
            (21, vec![done("b", 21, Outcome::Ok)]),
            (700, vec![activity(700)]),
        ]);
        let buf = draw(&mut app, &dark(), 80, 12);
        assert!(row(&buf, 0).contains('┆'), "{}", row(&buf, 0));
    }

    #[test]
    fn trailing_gap_is_never_folded() {
        // Activity stops at 10 s; the view runs to 100 s.
        assert_eq!(folds(&axis(&[(0, 10_000)], 0, 100_000, 40, true)), 0);
        // Even with a fold earlier, the tail stays long and plain.
        let cols = axis(&[(0, 5_000), (110_000, 115_000)], 0, 200_000, 40, true);
        assert_eq!(folds(&cols), 3);
        let after = cols.iter().filter(|c| c.t > 115_000).count();
        assert!(
            after > 20,
            "the trailing stretch keeps its columns: {after}"
        );
    }

    #[test]
    fn z_toggles_gap_folding() {
        let theme = dark();
        let mut app = gappy();
        assert!(app.fold_gaps);
        let folded = draw(&mut app, &theme, 80, 12);
        assert!(row(&folded, 0).contains('┆'));
        assert!(row(&folded, 1).contains('┆'), "the rule runs down the lane");

        app.fold_gaps = false;
        let plain = draw(&mut app, &theme, 80, 12);
        for y in 0..12 {
            assert!(!row(&plain, y).contains('┆'), "{}", row(&plain, y));
        }
        let hit = app.hit.lane_axis.clone().unwrap();
        assert!(hit.columns.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn axis_and_rows_are_registered_for_the_mouse() {
        let mut app = gappy();
        app.selection.agent = Some(MAIN_ID.into());
        let buf = draw(&mut app, &dark(), 80, 12);
        let hit = app.hit.lane_axis.clone().unwrap();
        assert_eq!(
            hit.area,
            Rect::new(15, 0, 65, 2),
            "axis row plus the one lane"
        );
        assert_eq!(app.hit.rows.len(), 1);
        assert_eq!(app.hit.rows[0].1, MAIN_ID);
        assert_eq!(app.hit.rows[0].0, Rect::new(0, 1, 14, 1));
        assert_eq!(app.hit.row_at(3, 1), Some(MAIN_ID));
        // Name column: selection marker, status glyph, bold brand name (the
        // contrast step, readable as text on both themes).
        let line = row(&buf, 1);
        assert!(line.starts_with("▶"), "{line}");
        assert!(line.contains("main"), "{line}");
        assert!(buf[(5, 1)].modifier.contains(Modifier::BOLD));
        assert_eq!(buf[(5, 1)].fg, dark().color(brand::FG_CONTRAST));
        // Riding the edge: `now` at the right end and `live` under it.
        assert!(row(&buf, 0).trim_end().ends_with("now"), "{}", row(&buf, 0));
        assert!(
            row(&buf, 2).trim_end().ends_with("live ▲"),
            "{}",
            row(&buf, 2)
        );
        assert!(row(&buf, 0).starts_with("Time"));
    }

    #[test]
    fn cursor_column_is_marked_off_the_edge() {
        // Scrubbed back, the view ends at the playhead: its line is the last column.
        let theme = dark();
        let mut app = gappy();
        app.fold_gaps = false;
        app.seek(at(350));
        assert!(!app.timeline.follow_head);
        let buf = draw(&mut app, &theme, 80, 12);
        let hit = app.hit.lane_axis.clone().unwrap();
        let x = hit.area.x + hit.column_of(at(350)) as u16;
        assert_eq!(x, hit.area.right() - 1);
        assert_eq!(buf[(x, 1)].symbol(), "│");
        assert_eq!(buf[(x, 1)].fg, theme.color(brand::FG));
        let footer = row(&buf, 2);
        assert!(footer.trim_end().ends_with('▲'), "{footer}");
        assert!(
            footer.contains(&format!("cursor {}", fmt_clock(at(350), app.utc_offset))),
            "{footer}"
        );
        assert_eq!(footer.chars().position(|c| c == '▲'), Some(x as usize));
        assert!(
            row(&buf, 0)
                .trim_end()
                .ends_with(&fmt_hm(at(350), app.utc_offset))
        );
        assert!(!row(&buf, 0).contains("now"));
    }

    #[test]
    fn bars_use_state_tones_in_every_theme() {
        for theme in all_themes() {
            let mut app = gappy();
            app.fold_gaps = false;
            let buf = draw(&mut app, &theme, 80, 12);
            let ok = buf[(15, 1)].fg;
            assert_eq!(ok, theme.tone(tool_tone(ToolState::Ok)).fg);
            // The failed call sits near 600/700 of the way across.
            let x = 15 + (65.0 * 602.0 / 700.0) as u16;
            let failed = (x - 2..=x + 2)
                .map(|x| &buf[(x, 1)])
                .find(|c| matches!(c.symbol(), "━" | "✗"));
            assert_eq!(
                failed.map(|c| c.fg),
                Some(theme.tone(tool_tone(ToolState::Err)).fg)
            );
        }
    }

    #[test]
    fn tool_name_is_written_into_a_wide_enough_span() {
        let mut app = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (60, vec![done("a", 60, Outcome::Ok)]),
            (100, vec![activity(100)]),
        ]);
        let buf = draw(&mut app, &dark(), 80, 12);
        assert!(row(&buf, 1).contains("Bash"), "{}", row(&buf, 1));
        // A 1-second call in a 100 s view is a single cell: no name.
        let mut app = app_of(vec![
            (0, vec![born(0), start("a", 0)]),
            (1, vec![done("a", 1, Outcome::Ok)]),
            (100, vec![activity(100)]),
        ]);
        app.fold_gaps = false;
        let buf = draw(&mut app, &dark(), 80, 12);
        assert!(!row(&buf, 1).contains("Bash"));
        assert!(row(&buf, 1).contains('━'));
    }

    #[test]
    fn empty_session_says_so() {
        let mut app = app_of(vec![(0, vec![born(0), activity(0)])]);
        let theme = dark();
        let buf = draw(&mut app, &theme, 60, 9);
        let (y, line) = (0..9)
            .map(|y| (y, row(&buf, y)))
            .find(|(_, l)| l.contains("No tool calls recorded"))
            .expect("empty text");
        assert_eq!(y, 4);
        let x = line.find("No tool").unwrap();
        assert_eq!(x, (60 - 22) / 2);
        assert_eq!(buf[(x as u16, y)].fg, theme.color(fg::PLACEHOLDER));
        assert!(app.hit.lane_axis.is_none());
        assert!(app.hit.rows.is_empty());
    }
}
