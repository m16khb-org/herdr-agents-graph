//! The Now view: who is doing what right now.
//!
//! Failed agents rise to callouts on top; below them one row per agent in
//! tree order — status glyph, name, what it is up to, and on the right the
//! tool it is running (or its status word and timing) and its output tokens.
//! The selected row can expand into its most recent tool calls.

use chrono::{DateTime, FixedOffset, Utc};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};

use super::attention;
use super::detail::tool_row;
use crate::state::App;
use crate::state::session::{AgentInfo, ToolCallInfo};
use crate::ui::seed::theme::{Theme, Tone, status_tone};
use crate::ui::seed::tokens::fg;
use crate::ui::seed::widgets::{Callout, Chip, ListItem, Skeleton};
use crate::ui::text::{fmt_dur, fmt_timing, fmt_tokens};

/// Below this width the tool chip and the token count are hidden.
pub(crate) const NARROW: u16 = 60;
/// Failed agents called out on top; the rest are counted by the hint bar.
const MAX_CALLOUTS: usize = 2;
/// Tool calls shown under the expanded row.
const EXPANDED_CALLS: usize = 5;

pub(crate) fn render(frame: &mut Frame, area: Rect, app: &mut App, theme: &Theme) {
    if area.is_empty() {
        return;
    }
    if app.timeline.items.is_empty() {
        frame.render_widget(Skeleton::new(*theme), area);
        return;
    }
    let wall = app.timeline.now_reference();
    let App {
        session,
        selection,
        hit,
        now_offset,
        flash,
        clock,
        utc_offset,
        ..
    } = app;
    let theme = *theme;

    let attention = attention(session, wall);
    let callouts = attention
        .failed
        .len()
        .min(MAX_CALLOUTS)
        .min(usize::from(area.height) / 3);
    for (i, failure) in attention.failed.iter().take(callouts).enumerate() {
        let rect = Rect::new(area.x, area.y + i as u16, area.width, 1);
        let title = format!("{} · {}", failure.name, failure.what);
        frame.render_widget(Callout::new(theme, Tone::Critical, title), rect);
        hit.rows.push((rect, failure.id.to_string()));
    }
    let list = Rect::new(
        area.x,
        area.y + callouts as u16,
        area.width,
        area.height - callouts as u16,
    );
    if list.is_empty() {
        return;
    }

    let rows = session.tree_order();
    let selected = selection
        .agent
        .as_deref()
        .and_then(|id| rows.iter().position(|(r, _)| *r == id));
    let recent: Vec<&ToolCallInfo> = match selected {
        Some(i) if selection.expanded => {
            let mut calls: Vec<_> = session
                .agent(rows[i].0)
                .into_iter()
                .flat_map(|a| a.tool_calls().rev().take(EXPANDED_CALLS))
                .collect();
            calls.reverse();
            calls
        }
        _ => Vec::new(),
    };

    let total = rows.len() + recent.len();
    let selected_at = selected.map(|i| (i, i + recent.len()));
    let offset = scroll_offset(total, usize::from(list.height), *now_offset, selected_at);
    *now_offset = offset;

    let narrow = list.width < NARROW;
    let flashing = flash.as_ref().filter(|f| f.until > *clock);
    let end = offset + usize::from(list.height);
    // Virtual line of the next agent row; expanded calls sit after the
    // selected one.
    let mut line = 0usize;
    for (i, (id, depth)) in rows.iter().enumerate() {
        if line >= end {
            break;
        }
        let is_selected = selected == Some(i);
        let span = 1 + if is_selected { recent.len() } else { 0 };
        if line + span > offset {
            if line >= offset {
                let Some(agent) = session.agent(id) else {
                    line += span;
                    continue;
                };
                let failed = attention.failed.iter().any(|f| f.id == *id);
                let item = agent_item(
                    theme,
                    session.intent_line(id),
                    agent,
                    *depth,
                    RowFlags {
                        failed,
                        narrow,
                        selected: is_selected,
                        flash: flashing.is_some_and(|f| f.ids.iter().any(|x| x == id)),
                    },
                    wall,
                    *utc_offset,
                );
                let rect = Rect::new(list.x, list.y + (line - offset) as u16, list.width, 1);
                frame.render_widget(item, rect);
                hit.rows.push((rect, id.to_string()));
            }
            if is_selected {
                for (k, call) in recent.iter().enumerate() {
                    let at = line + 1 + k;
                    if at < offset || at >= end {
                        continue;
                    }
                    let l = call_line(
                        theme,
                        call,
                        *depth,
                        k + 1 == recent.len(),
                        usize::from(list.width),
                        *utc_offset,
                    );
                    let rect = Rect::new(list.x, list.y + (at - offset) as u16, list.width, 1);
                    frame.render_widget(l, rect);
                }
            }
        }
        line += span;
    }
}

/// First line shown: the stored offset clamped to the reachable maximum and
/// moved just far enough that the selected row (and, when it is expanded,
/// as much of its calls as fit) is on screen. `selected_at` is the
/// selected row's line and the last line of its block.
fn scroll_offset(
    total: usize,
    height: usize,
    stored: usize,
    selected_at: Option<(usize, usize)>,
) -> usize {
    let mut offset = stored.min(total.saturating_sub(height));
    if let Some((first, last)) = selected_at {
        if first < offset {
            offset = first;
        } else if last >= offset + height {
            offset = (last + 1).saturating_sub(height).min(first);
        }
    }
    offset
}

struct RowFlags {
    failed: bool,
    narrow: bool,
    selected: bool,
    flash: bool,
}

fn agent_item(
    theme: Theme,
    intent: Option<&str>,
    agent: &AgentInfo,
    depth: usize,
    flags: RowFlags,
    wall: Option<DateTime<Utc>>,
    offset: FixedOffset,
) -> ListItem {
    let tone = if flags.failed {
        Tone::Critical
    } else {
        status_tone(agent.status)
    };
    let tone_fg = theme.tone(tone).fg;
    let word_style = Style::default().fg(tone_fg);
    let glyph = Span::styled(
        agent.status.glyph().to_string(),
        Style::default().fg(theme.tone(status_tone(agent.status)).fg),
    );

    let mut trailing: Vec<Span<'static>> = Vec::new();
    let in_flight = agent.current_call().filter(|_| !agent.is_terminal());
    match in_flight {
        Some(call) if !flags.failed && !flags.narrow => {
            let label = match call.ts.zip(wall) {
                Some((start, now)) => format!("{} {}", call.name, fmt_dur(now - start)),
                None => call.name.clone(),
            };
            trailing.extend(Chip::new(theme, label).spans());
        }
        _ => {
            trailing.push(Span::styled(agent.status_word(), word_style));
            if !flags.narrow
                && let Some(timing) = fmt_timing(agent, offset)
            {
                trailing.push(Span::styled(
                    format!(" {timing}"),
                    theme.fg(fg::NEUTRAL_MUTED),
                ));
            }
        }
    }
    if !flags.narrow && agent.output_tokens > 0 {
        trailing.push(Span::styled(
            format!("  {} tok", fmt_tokens(agent.output_tokens)),
            theme.fg(fg::NEUTRAL_MUTED),
        ));
    }

    ListItem::new(theme)
        .depth(depth as u16)
        .leading(vec![glyph])
        .title(agent.display_name())
        .detail(intent.unwrap_or("—"))
        .trailing(trailing)
        .selected(flags.selected)
        .flash(flags.flash)
}

/// One recent call under the expanded row: connector, state glyph, name,
/// what it did, and when.
fn call_line(
    theme: Theme,
    call: &ToolCallInfo,
    depth: usize,
    last: bool,
    cols: usize,
    offset: FixedOffset,
) -> Line<'static> {
    let prefix = format!(
        "{}{} ",
        " ".repeat(2 + depth * 2 + 2),
        if last { '└' } else { '├' }
    );
    tool_row(&theme, call, &prefix, cols, offset)
}

#[cfg(test)]
pub(crate) mod fixture {
    //! Apps built from facts and frames drawn into a `TestBackend`, for the
    //! view tests.

    use chrono::{DateTime, FixedOffset, Utc};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect};

    use crate::fact::{AgentKind, Fact, FactKind, Outcome, Statement};
    use crate::state::session::MAIN_ID;
    use crate::state::{App, Mode};
    use crate::tailer::{ReplayItem, UiEvent};
    use crate::ui::seed::theme::Theme;

    pub fn at(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_780_000_000 + s, 0).unwrap()
    }

    pub fn fact(agent: &str, s: i64, kind: FactKind) -> Fact {
        Fact {
            agent: Some(agent.into()),
            ts: Some(at(s)),
            kind,
        }
    }

    pub fn start(
        agent: &str,
        s: i64,
        call: &str,
        summary: Option<&str>,
        intent: Option<&str>,
    ) -> Fact {
        fact(
            agent,
            s,
            FactKind::ToolStart {
                call: call.into(),
                name: "bash".into(),
                summary: summary.map(str::to_string),
                intent: intent.map(str::to_string),
            },
        )
    }

    pub fn end(agent: &str, s: i64, call: &str, outcome: Outcome) -> Fact {
        fact(
            agent,
            s,
            FactKind::ToolEnd {
                call: call.into(),
                outcome,
            },
        )
    }

    /// A replay folded to its end: every statement `(second, facts)`, each
    /// agent born (as main, or a subagent of main) in the statement that
    /// first mentions it.
    pub fn app(steps: Vec<(i64, Vec<Fact>)>) -> App {
        let mut seen: Vec<String> = Vec::new();
        let mut items = Vec::new();
        for (s, facts) in steps {
            let mut all = Vec::new();
            for f in &facts {
                let id = f.agent.clone().unwrap();
                if !seen.contains(&id) {
                    let main = id == MAIN_ID;
                    all.push(fact(
                        &id,
                        s,
                        FactKind::Agent {
                            kind: if main {
                                AgentKind::Main
                            } else {
                                AgentKind::Subagent
                            },
                            parent: (!main).then(|| MAIN_ID.to_string()),
                            agent_type: Some(id.clone()),
                            description: None,
                            spawned_by: None,
                            interactive: main,
                        },
                    ));
                    seen.push(id);
                }
            }
            all.extend(facts);
            items.push(ReplayItem::new(Statement {
                at: Some(at(s)),
                facts: all,
            }));
        }
        let mut app = App::new("s".into(), Mode::Replay);
        app.handle_ui_event(UiEvent::ReplayLoaded {
            session_id: "s".into(),
            items,
            speed: 8.0,
            info: Default::default(),
        });
        app.go_live();
        app.tick_timeline(std::time::Duration::ZERO);
        app.utc_offset = FixedOffset::east_opt(0).unwrap();
        app
    }

    /// Draw `f` into a `w` × `h` frame and return the buffer.
    pub fn draw(w: u16, h: u16, f: impl FnOnce(&mut ratatui::Frame, Rect)) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| f(frame, Rect::new(0, 0, w, h)))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    pub fn rows(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    pub fn render_now(app: &mut App, theme: &Theme, w: u16, h: u16) -> Buffer {
        draw(w, h, |f, a| super::render(f, a, app, theme))
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{app, at, end, fact, render_now, rows, start};
    use super::*;
    use crate::fact::{AgentStatus, FactKind, Outcome};
    use crate::state::session::MAIN_ID;
    use crate::ui::seed::tokens::bg;
    use crate::ui::seed::widgets::testing::all_themes;

    fn tokens(agent: &str, s: i64, n: u64) -> crate::fact::Fact {
        fact(
            agent,
            s,
            FactKind::Tokens {
                output: n,
                dedup: None,
                cost_usd: None,
            },
        )
    }

    #[test]
    fn failures_rise_to_callout() {
        for theme in all_themes() {
            let mut a = app(vec![
                (0, vec![fact(MAIN_ID, 0, FactKind::Activity)]),
                (
                    5,
                    vec![
                        start("a", 5, "c1", Some("cargo test"), None),
                        end("a", 6, "c1", Outcome::Err),
                        fact("a", 7, FactKind::Ended(AgentStatus::Failed)),
                        start("b", 5, "c2", Some("make"), None),
                        end("b", 6, "c2", Outcome::Err),
                        start("c", 5, "c3", Some("lint"), None),
                        end("c", 6, "c3", Outcome::Err),
                    ],
                ),
            ]);
            let buf = render_now(&mut a, &theme, 100, 16);
            let lines = rows(&buf);
            assert!(lines[0].contains("a · bash · cargo test"), "{lines:#?}");
            assert!(lines[1].contains("b · bash · make"), "{lines:#?}");
            // At most two callouts; the third failure is only counted by the hint bar.
            assert_eq!(lines.iter().filter(|l| l.starts_with('┃')).count(), 2);
            assert!(!lines.iter().any(|l| l.contains("c · bash · lint")));
            // The agent rows follow, and the failed agent's word is critical.
            let row = lines
                .iter()
                .position(|l| l.contains("failed") && !l.starts_with('┃'))
                .expect("a row says failed");
            assert!(row >= 2);
            let x = lines[row]
                .find("failed")
                .map(|b| lines[row][..b].chars().count())
                .unwrap();
            assert_eq!(
                buf[(x as u16, row as u16)].fg,
                theme.tone(Tone::Critical).fg
            );
        }
    }

    #[test]
    fn narrow_width_hides_tokens() {
        for theme in all_themes() {
            let mut a = app(vec![(
                0,
                vec![
                    start(MAIN_ID, 0, "c1", None, Some("Fixing parser")),
                    tokens(MAIN_ID, 1, 1500),
                ],
            )]);
            let narrow = rows(&render_now(&mut a, &theme, 50, 8)).join("\n");
            assert!(!narrow.contains("tok"), "{narrow}");
            assert!(narrow.contains("Fixing parser"), "{narrow}");
            assert!(
                narrow.contains("active") || narrow.contains("idle"),
                "{narrow}"
            );
            let wide = rows(&render_now(&mut a, &theme, 80, 8)).join("\n");
            assert!(wide.contains("1.5k tok"), "{wide}");
        }
    }

    #[test]
    fn selected_row_has_marker_and_neutral_weak_background() {
        for theme in all_themes() {
            let mut a = app(vec![(
                0,
                vec![
                    fact(MAIN_ID, 0, FactKind::Activity),
                    fact("a", 1, FactKind::Activity),
                ],
            )]);
            a.selection.agent = Some("a".into());
            let buf = render_now(&mut a, &theme, 80, 6);
            let lines = rows(&buf);
            assert!(lines[1].starts_with("▶"), "{lines:#?}");
            assert!(!lines[0].starts_with("▶"));
            assert_eq!(buf[(0, 1)].bg, theme.color(bg::NEUTRAL_WEAK));
            assert_ne!(buf[(0, 0)].bg, theme.color(bg::NEUTRAL_WEAK));
        }
    }

    #[test]
    fn expanded_shows_at_most_five_recent_calls_newest_last() {
        let theme = all_themes()[0];
        let mut facts = vec![fact(MAIN_ID, 0, FactKind::Activity)];
        for i in 0..8 {
            let call = format!("c{i}");
            facts.push(start(
                MAIN_ID,
                10 + i,
                &call,
                Some(&format!("cmd{i}")),
                None,
            ));
            facts.push(end(MAIN_ID, 10 + i, &call, Outcome::Ok));
        }
        let mut a = app(vec![(10, facts)]);
        a.selection.agent = Some(MAIN_ID.into());
        a.selection.expanded = true;
        let lines = rows(&render_now(&mut a, &theme, 90, 12));
        let calls: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains('├') || l.contains('└'))
            .collect();
        assert_eq!(calls.len(), 5, "{lines:#?}");
        for (k, i) in (3..8).enumerate() {
            assert!(calls[k].contains(&format!("cmd{i}")), "{calls:#?}");
        }
        assert!(calls[4].starts_with("    └"), "{calls:#?}");
        assert!(calls[0].contains("├"));
        assert!(
            calls[0].ends_with(&at(13).format("%H:%M:%S").to_string()),
            "right-aligned clock: {calls:#?}"
        );
    }

    #[test]
    fn pending_elapsed_follows_now_reference_not_wall_clock() {
        for theme in all_themes() {
            let mut a = app(vec![
                (5, vec![start("a", 5, "c1", None, Some("Running tests"))]),
                (65, vec![fact(MAIN_ID, 65, FactKind::Activity)]),
            ]);
            let wall = a.timeline.now_reference();
            assert_eq!(wall, Some(at(65)), "a replay's reference is its playhead");
            let text = rows(&render_now(&mut a, &theme, 100, 6)).join("\n");
            assert!(text.contains("bash 1m00s"), "{text}");
        }
    }

    #[test]
    fn rows_register_hit_rects() {
        let theme = all_themes()[1];
        let mut a = app(vec![(
            0,
            vec![
                fact(MAIN_ID, 0, FactKind::Activity),
                fact("a", 1, FactKind::Activity),
            ],
        )]);
        render_now(&mut a, &theme, 80, 6);
        let ids: Vec<&str> = a.hit.rows.iter().map(|(_, id)| id.as_str()).collect();
        assert_eq!(ids, [MAIN_ID, "a"]);
        assert_eq!(a.hit.rows[0].0, Rect::new(0, 0, 80, 1));
        assert_eq!(a.hit.rows[1].0, Rect::new(0, 1, 80, 1));
    }

    #[test]
    fn scrolling_keeps_the_selected_row_visible_and_persists_the_offset() {
        let theme = all_themes()[0];
        let steps = (0..10)
            .map(|i| {
                let id = format!("w{i}");
                (i, vec![fact(&id, i, FactKind::Activity)])
            })
            .collect();
        let mut a = app(steps);
        a.selection.agent = Some("w9".into());
        let lines = rows(&render_now(&mut a, &theme, 80, 4));
        assert!(lines[3].contains("w9"), "{lines:#?}");
        assert!(a.now_offset > 0);
        // Moving the selection back up scrolls just far enough.
        a.selection.agent = Some("w5".into());
        a.hit.rows.clear();
        let lines = rows(&render_now(&mut a, &theme, 80, 4));
        assert!(lines.iter().any(|l| l.starts_with("▶") && l.contains("w5")));
        assert_eq!(a.hit.rows.len(), 4);
    }

    #[test]
    fn scroll_offset_clamps_and_reveals_the_selected_block() {
        assert_eq!(scroll_offset(10, 4, 99, None), 6);
        assert_eq!(scroll_offset(10, 4, 0, Some((7, 7))), 4);
        assert_eq!(scroll_offset(10, 4, 8, Some((2, 2))), 2);
        // An expanded block taller than the screen keeps its head in view.
        assert_eq!(scroll_offset(20, 4, 0, Some((3, 9))), 3);
        assert_eq!(scroll_offset(3, 10, 5, None), 0);
    }

    #[test]
    fn an_empty_timeline_shows_a_skeleton_and_tiny_areas_do_not_panic() {
        let theme = all_themes()[0];
        let mut empty = crate::state::App::new("s".into(), crate::state::Mode::Replay);
        let text = rows(&render_now(&mut empty, &theme, 30, 4)).join("\n");
        assert!(text.contains('░'));

        let mut a = app(vec![(
            0,
            vec![
                start(MAIN_ID, 0, "c1", Some("x"), None),
                end(MAIN_ID, 0, "c1", Outcome::Err),
            ],
        )]);
        a.selection.agent = Some(MAIN_ID.into());
        a.selection.expanded = true;
        for (w, h) in [(0, 0), (1, 1), (2, 1), (5, 2), (60, 1), (60, 3)] {
            render_now(&mut a, &theme, w, h);
        }
    }
}
