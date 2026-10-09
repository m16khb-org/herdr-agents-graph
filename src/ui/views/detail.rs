//! The detail panel: one agent in full — who it is, why it exists, and the
//! scrollable list of everything it called.

use chrono::FixedOffset;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::state::App;
use crate::state::session::{
    AgentInfo, AgentKind, AgentStatus, SessionModel, ToolCallInfo, ToolState,
};
use crate::ui::seed::theme::{Theme, status_tone, tool_tone};
use crate::ui::seed::tokens::{bg, brand, fg, stroke};
use crate::ui::seed::widgets::{Badge, Divider};
use crate::ui::text::{
    fmt_clock, fmt_cost, fmt_timing, fmt_tokens, fmt_tool_count, tool_text, truncate, width, wrap,
};

/// Line cap for the prompt (and the reasoning) in the provenance block: it
/// sits in a fixed region, so an unbounded prompt would starve the tool list.
const PROMPT_MAX_LINES: usize = 6;
/// Columns of the `↳ prompt  ` label the provenance text is indented by.
const LABEL_COLS: usize = 10;
/// Tool-list rows (plus its rule) the provenance block never crowds out.
const MIN_TOOL_ROWS: u16 = 4;

pub(crate) fn render(frame: &mut Frame, area: Rect, app: &mut App, theme: &Theme, agent_id: &str) {
    let theme = *theme;
    let App {
        session,
        selection,
        utc_offset,
        ..
    } = app;
    let surface = theme.surface();
    let mut block = panel(&theme);
    let inner = block.inner(area);

    let Some(agent) = session.agent(agent_id) else {
        frame.render_widget(block, area);
        if !inner.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No detail for this agent",
                    theme.fg(fg::NEUTRAL_MUTED),
                ))
                .style(surface),
                inner,
            );
        }
        return;
    };
    if inner.is_empty() {
        frame.render_widget(block, area);
        return;
    }

    let w = usize::from(inner.width);
    let header = header_lines(&theme, agent, *utc_offset, w);
    let header_h = header.len() as u16;
    let mut prov = provenance_lines(&theme, session, agent, w);
    prov.truncate(usize::from(
        inner.height.saturating_sub(header_h + MIN_TOOL_ROWS),
    ));
    let [header_area, prov_area, tools_area] = Layout::vertical([
        Constraint::Length(header_h),
        Constraint::Length(prov.len() as u16),
        Constraint::Fill(1),
    ])
    .areas(inner);

    // The list under its rule: resolve the scroll against the real length
    // and write it back so the next key press matches what is on screen.
    let list_h = tools_area.height.saturating_sub(1);
    let (eras, total) = tool_list_lines(session, agent);
    let headers = eras >= 2;
    let (scroll, follow) = resolve_scroll(
        total.min(usize::from(u16::MAX)) as u16,
        list_h,
        selection.scroll,
        selection.follow,
    );
    selection.scroll = scroll;
    selection.follow = follow;
    if total > usize::from(list_h) && list_h > 0 {
        let label = if follow {
            " j/k ↕ tail ".to_string()
        } else {
            format!(" j/k ↕ {scroll}/{total} ")
        };
        block = block.title_bottom(
            Line::from(label)
                .right_aligned()
                .style(surface.fg(theme.color(fg::NEUTRAL_SUBTLE))),
        );
    }
    frame.render_widget(block, area);

    frame.render_widget(Paragraph::new(header).style(surface), header_area);
    if !prov.is_empty() {
        frame.render_widget(Paragraph::new(prov).style(surface), prov_area);
    }
    if tools_area.height == 0 {
        return;
    }
    let rule = Rect::new(tools_area.x, tools_area.y, tools_area.width, 1);
    frame.buffer_mut().set_style(rule, surface);
    frame.render_widget(Divider::new(theme), rule);
    if rule.width > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled(" tool calls ", theme.fg(fg::NEUTRAL_SUBTLE)))
                .style(surface),
            Rect::new(rule.x + 2, rule.y, rule.width - 2, 1),
        );
    }
    let list = Rect::new(tools_area.x, tools_area.y + 1, tools_area.width, list_h);
    if list.is_empty() {
        return;
    }
    if agent.tool_calls().len() == 0 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "No tool calls recorded",
                theme.fg(fg::NEUTRAL_MUTED),
            ))
            .style(surface),
            list,
        );
        return;
    }
    let lines = visible_tool_lines(
        &theme,
        session,
        agent,
        headers,
        usize::from(scroll),
        usize::from(list_h),
        w,
        *utc_offset,
    );
    frame.render_widget(Paragraph::new(lines).style(surface), list);
}

/// The panel's frame: a rounded focus border with the close hint.
fn panel(theme: &Theme) -> Block<'static> {
    let surface = theme.surface();
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            theme
                .fg(stroke::NEUTRAL_CONTRAST)
                .bg(theme.color(bg::LAYER_DEFAULT)),
        )
        .style(surface)
        .padding(Padding::horizontal(1))
        .title_top(
            Line::from(" esc ✕ ")
                .right_aligned()
                .style(surface.fg(theme.color(fg::NEUTRAL_SUBTLE))),
        )
}

/// A fold in full: how many are done, their calls and tokens, and one line
/// per member — status, name, timing, tokens.
pub(crate) fn render_fold(frame: &mut Frame, area: Rect, app: &App, theme: &Theme, parent: &str) {
    let block = panel(theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let folds = app.folds();
    let members: Vec<&AgentInfo> = folds
        .by_parent
        .get(parent)
        .into_iter()
        .flatten()
        .filter_map(|id| app.session.agent(id))
        .collect();
    let w = usize::from(inner.width);
    let muted = theme.fg(fg::NEUTRAL_MUTED);
    let done = AgentStatus::Done;
    let mut first = Badge::new(*theme, status_tone(done), "done")
        .glyph(done.glyph())
        .spans();
    first.push(Span::styled(
        format!(" {} done", members.len()),
        theme.fg(fg::NEUTRAL).add_modifier(Modifier::BOLD),
    ));
    let calls = members.iter().map(|a| a.tool_calls().len()).sum();
    let tokens = members
        .iter()
        .fold(0u64, |sum, a| sum.saturating_add(a.output_tokens));
    let facts = format!(
        "{} · {} tok · enter unfolds",
        fmt_tool_count(calls),
        fmt_tokens(tokens)
    );
    let mut lines = vec![
        Line::from(first),
        Line::from(Span::styled(truncate(&facts, w), muted)),
    ];
    for agent in members {
        let mut row = vec![
            Span::styled(
                format!("{} ", agent.status.glyph()),
                Style::default().fg(theme.tone(status_tone(agent.status)).fg),
            ),
            Span::styled(agent.display_name().to_string(), theme.fg(fg::NEUTRAL)),
        ];
        if let Some(timing) = fmt_timing(agent, app.utc_offset) {
            row.push(Span::styled(format!("  {timing}"), muted));
        }
        if agent.output_tokens > 0 {
            row.push(Span::styled(
                format!("  {} tok", fmt_tokens(agent.output_tokens)),
                muted,
            ));
        }
        lines.push(Line::from(row));
    }
    frame.render_widget(Paragraph::new(lines).style(theme.surface()), inner);
}

/// The status badge: glyph + word + tone, never color alone.
fn status_badge(theme: &Theme, agent: &AgentInfo) -> Badge {
    Badge::new(*theme, status_tone(agent.status), agent.status_word()).glyph(agent.status.glyph())
}

/// The glyph a tool call's state shows as.
fn tool_glyph(state: ToolState) -> char {
    match state {
        ToolState::Pending => '●',
        ToolState::Ok => '✓',
        ToolState::Err => '✗',
    }
}

/// Name, state, model, timing, tokens and — only when one was recorded —
/// cost, cut to `cols` with an ellipsis; then the agent's own description.
fn header_lines(
    theme: &Theme,
    agent: &AgentInfo,
    offset: FixedOffset,
    cols: usize,
) -> Vec<Line<'static>> {
    let muted = theme.fg(fg::NEUTRAL_MUTED);
    let mut first = status_badge(theme, agent).spans();
    first.push(Span::styled(
        format!(" {}", agent.display_name()),
        theme.fg(fg::NEUTRAL).add_modifier(Modifier::BOLD),
    ));
    if let Some(timing) = fmt_timing(agent, offset) {
        first.push(Span::styled(format!("  {timing}"), muted));
    }

    let mut facts: Vec<String> = Vec::new();
    if let Some(model) = agent.model.as_deref() {
        facts.push(model.to_string());
    }
    facts.push(fmt_tool_count(agent.tool_calls().len()));
    facts.push(format!("{} tok", fmt_tokens(agent.output_tokens)));
    if let Some(cost) = agent.cost_usd {
        facts.push(fmt_cost(cost));
    }
    let mut lines = vec![
        Line::from(first),
        Line::from(Span::styled(truncate(&facts.join(" · "), cols), muted)),
    ];
    if let Some(desc) = agent.description.as_deref().filter(|d| !d.is_empty()) {
        lines.push(Line::from(Span::styled(
            truncate(&desc.split_whitespace().collect::<Vec<_>>().join(" "), 400),
            theme.fg(fg::NEUTRAL_SUBTLE),
        )));
    }
    lines
}

/// Why the agent exists: the prompt that led to its spawn (the first prompt
/// for the main agent) on a quiet band, and the reasoning right before the
/// spawn, muted.
fn provenance_lines(
    theme: &Theme,
    session: &SessionModel,
    agent: &AgentInfo,
    cols: usize,
) -> Vec<Line<'static>> {
    let (prompt, reasoning) = match session.provenance(agent) {
        Some(ctx) => (session.provenance_prompt(ctx), ctx.reasoning.as_deref()),
        None if agent.kind == AgentKind::Main => (session.first_prompt(), None),
        None => (None, None),
    };
    let text_w = cols.saturating_sub(LABEL_COLS).max(1);
    let label = theme.fg(brand::FG_CONTRAST);
    let band = Style::default().bg(theme.color(bg::NEUTRAL_WEAK));
    let mut lines = Vec::new();
    for (i, l) in wrap(prompt.unwrap_or_default(), text_w, PROMPT_MAX_LINES)
        .into_iter()
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let prefix = if i == 0 { "↳ prompt  " } else { "          " };
        let pad = cols.saturating_sub(LABEL_COLS + width(&l));
        lines.push(Line::from(vec![
            Span::styled(prefix, label.bg(theme.color(bg::NEUTRAL_WEAK))),
            Span::styled(l, band.fg(theme.color(fg::NEUTRAL))),
            Span::styled(" ".repeat(pad), band),
        ]));
    }
    for (i, l) in wrap(reasoning.unwrap_or_default(), text_w, PROMPT_MAX_LINES)
        .into_iter()
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let prefix = if i == 0 { "↳ thought " } else { "          " };
        lines.push(Line::from(vec![
            Span::styled(prefix, label),
            Span::styled(l, theme.fg(fg::NEUTRAL_MUTED)),
        ]));
    }
    lines
}

/// Walks an agent's start-ordered calls beside the session's prompts in one
/// merge, yielding each call with `Some(era)` when it is the first call of a
/// new prompt era (the index of that prompt). Prompts are timestamp-ordered,
/// so a single pointer into them replaces a lookup per call.
struct EraWalk<'a, I> {
    calls: I,
    session: &'a SessionModel,
    next_prompt: usize,
    era: Option<usize>,
}

fn era_walk<'a>(
    session: &'a SessionModel,
    agent: &'a AgentInfo,
) -> EraWalk<'a, impl Iterator<Item = &'a ToolCallInfo>> {
    EraWalk {
        calls: agent.tool_calls(),
        session,
        next_prompt: 0,
        era: None,
    }
}

impl<'a, I: Iterator<Item = &'a ToolCallInfo>> Iterator for EraWalk<'a, I> {
    type Item = (&'a ToolCallInfo, Option<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let call = self.calls.next()?;
        let mut opens = None;
        if let Some(ts) = call.ts {
            let mut reached = None;
            while let Some(prompt) = self.session.prompts.get(self.next_prompt) {
                match prompt.ts {
                    Some(at) if at <= ts => reached = Some(self.next_prompt),
                    Some(_) => break,
                    None => {}
                }
                self.next_prompt += 1;
            }
            if reached.is_some() && reached != self.era {
                self.era = reached;
                opens = reached;
            }
        }
        Some((call, opens))
    }
}

/// `(eras opened, total lines)` of the tool list. Era headers show only when
/// the calls span two or more eras — a single era is the provenance's story.
fn tool_list_lines(session: &SessionModel, agent: &AgentInfo) -> (usize, usize) {
    let eras = era_walk(session, agent)
        .filter(|(_, o)| o.is_some())
        .count();
    let calls = agent.tool_calls().len();
    (eras, calls + if eras >= 2 { eras } else { 0 })
}

/// Only the lines intersecting `[offset, offset + height)`.
#[allow(clippy::too_many_arguments)]
fn visible_tool_lines(
    theme: &Theme,
    session: &SessionModel,
    agent: &AgentInfo,
    headers: bool,
    offset: usize,
    height: usize,
    cols: usize,
    utc_offset: FixedOffset,
) -> Vec<Line<'static>> {
    let end = offset + height;
    let mut lines = Vec::with_capacity(height);
    let mut at = 0usize;
    for (call, opens) in era_walk(session, agent) {
        if at >= end {
            break;
        }
        if let (true, Some(era)) = (headers, opens) {
            if at >= offset {
                let excerpt = session.prompts.get(era).map_or("", |p| p.excerpt.as_str());
                lines.push(era_header(theme, excerpt, cols));
            }
            at += 1;
        }
        if at >= offset && at < end {
            lines.push(tool_row(theme, call, "", cols, utc_offset));
        }
        at += 1;
    }
    lines
}

fn era_header(theme: &Theme, excerpt: &str, cols: usize) -> Line<'static> {
    let band = Style::default().bg(theme.color(bg::NEUTRAL_WEAK));
    let text = truncate(excerpt, cols.saturating_sub(2));
    let pad = cols.saturating_sub(2 + width(&text));
    Line::from(vec![
        Span::styled("◆ ", band.fg(theme.color(brand::FG))),
        Span::styled(text, band.fg(theme.color(fg::NEUTRAL))),
        Span::styled(" ".repeat(pad), band),
    ])
}

/// One tool call: state glyph, name, what it did, and the clock time flush
/// right. `prefix` (muted) goes first, for rows nested under something.
pub(crate) fn tool_row(
    theme: &Theme,
    call: &ToolCallInfo,
    prefix: &str,
    cols: usize,
    offset: FixedOffset,
) -> Line<'static> {
    let muted = theme.fg(fg::NEUTRAL_MUTED);
    let glyph = format!("{} ", tool_glyph(call.state));
    let clock = call.ts.map(|t| fmt_clock(t, offset));
    let clock_w = clock.as_ref().map_or(0, |c| width(c) + 1);
    let mut used = width(prefix) + width(&glyph) + width(&call.name);
    let budget = cols.saturating_sub(used + 1 + clock_w);
    let text = tool_text(
        &call.name,
        call.intent.as_deref(),
        call.summary.as_deref(),
        budget,
    );

    let mut spans = Vec::with_capacity(6);
    if !prefix.is_empty() {
        spans.push(Span::styled(prefix.to_string(), muted));
    }
    spans.push(Span::styled(
        glyph,
        Style::default().fg(theme.tone(tool_tone(call.state)).fg),
    ));
    spans.push(Span::styled(
        call.name.clone(),
        theme.fg(fg::NEUTRAL).add_modifier(Modifier::BOLD),
    ));
    if let Some(text) = text.filter(|t| !t.is_empty()) {
        used += 1 + width(&text);
        spans.push(Span::styled(format!(" {text}"), muted));
    }
    if let Some(clock) = clock {
        let pad = cols.saturating_sub(used + clock_w);
        spans.push(Span::raw(" ".repeat(pad + 1)));
        spans.push(Span::styled(clock, muted));
    }
    Line::from(spans)
}

/// Resolve the list's scroll offset for one render: clamp to the reachable
/// maximum (the last screenful, no over-scroll into blank) and reconcile the
/// tail. Following pins to the bottom; scrolling back down to the bottom (or
/// content that fits) re-attaches. Returns `(offset, tailing)`.
fn resolve_scroll(total: u16, height: u16, scroll: u16, follow: bool) -> (u16, bool) {
    let max = total.saturating_sub(height);
    let offset = if follow { max } else { scroll.min(max) };
    (offset, offset >= max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::{FactKind, Outcome};
    use crate::state::session::MAIN_ID;
    use crate::ui::seed::widgets::testing::all_themes;
    use crate::ui::views::now::fixture::{app, draw, end, fact, rows, start};

    fn render_detail(app: &mut App, theme: &Theme, w: u16, h: u16) -> Vec<String> {
        rows(&draw(w, h, |f, a| render(f, a, app, theme, MAIN_ID)))
    }

    fn prompt(s: i64, text: &str) -> crate::fact::Fact {
        fact(MAIN_ID, s, FactKind::Prompt(text.into()))
    }

    fn call(s: i64, n: usize) -> Vec<crate::fact::Fact> {
        let id = format!("c{n}");
        vec![
            start(MAIN_ID, s, &id, Some(&format!("cmd{n}")), None),
            end(MAIN_ID, s, &id, Outcome::Ok),
        ]
    }

    #[test]
    fn resolve_scroll_clamps_and_reconciles_tail() {
        // Content shorter than the viewport → always tailing, offset 0.
        assert_eq!(resolve_scroll(5, 10, 3, false), (0, true));
        // Following → pinned to the bottom (max = 20 - 8 = 12).
        assert_eq!(resolve_scroll(20, 8, 0, true), (12, true));
        // Detached and scrolled up → keep the offset, stay detached.
        assert_eq!(resolve_scroll(20, 8, 5, false), (5, false));
        // Detached but (over-)scrolled to the bottom → clamp + re-attach.
        assert_eq!(resolve_scroll(20, 8, 99, false), (12, true));
    }

    #[test]
    fn tool_list_lines_counts_era_headers() {
        let a = app(vec![
            (0, vec![prompt(0, "first")]),
            (5, call(5, 0)),
            (10, vec![prompt(10, "second")]),
            (15, call(15, 1)),
            (16, call(16, 2)),
        ]);
        let agent = a.session.agent(MAIN_ID).unwrap();
        // 3 tool rows + 2 era headers = 5 lines.
        assert_eq!(tool_list_lines(&a.session, agent), (2, 5));

        // One era shows no headers: the provenance names it.
        let one = app(vec![
            (0, vec![prompt(0, "only")]),
            (5, call(5, 0)),
            (6, call(6, 1)),
        ]);
        let agent = one.session.agent(MAIN_ID).unwrap();
        assert_eq!(tool_list_lines(&one.session, agent), (1, 2));
    }

    #[test]
    fn era_headers_and_provenance_render_for_the_main_agent() {
        for theme in all_themes() {
            let mut a = app(vec![
                (0, vec![prompt(0, "fix the parser")]),
                (5, call(5, 0)),
                (10, vec![prompt(10, "now add tests")]),
                (15, call(15, 1)),
            ]);
            let text = render_detail(&mut a, &theme, 70, 16);
            let joined = text.join("\n");
            assert!(joined.contains("↳ prompt  fix the parser"), "{joined}");
            assert!(joined.contains("◆ fix the parser"), "{joined}");
            assert!(joined.contains("◆ now add tests"), "{joined}");
            let first = joined.find("◆ fix").unwrap();
            let call0 = joined.find("cmd0").unwrap();
            let second = joined.find("◆ now").unwrap();
            let call1 = joined.find("cmd1").unwrap();
            assert!(first < call0 && call0 < second && second < call1);
            // Rounded border, focus stroke.
            assert!(text[0].starts_with('╭'));
            let buf = draw(70, 16, |f, r| render(f, r, &mut a, &theme, MAIN_ID));
            assert_eq!(
                buf[(0, 0)].fg,
                theme.color(crate::ui::seed::tokens::stroke::NEUTRAL_CONTRAST)
            );
        }
    }

    #[test]
    fn header_shows_cost_only_when_recorded() {
        let theme = all_themes()[0];
        let tokens = |cost: Option<f64>| {
            fact(
                MAIN_ID,
                0,
                FactKind::Tokens {
                    output: 1200,
                    dedup: None,
                    cost_usd: cost,
                },
            )
        };
        let mut with = app(vec![(0, vec![tokens(Some(0.42))])]);
        let mut without = app(vec![(0, vec![tokens(None)])]);
        let with = render_detail(&mut with, &theme, 60, 8).join("\n");
        let without = render_detail(&mut without, &theme, 60, 8).join("\n");
        assert!(with.contains("1.2k tok · $0.42"), "{with}");
        assert!(without.contains("1.2k tok"), "{without}");
        assert!(!without.contains('$'), "{without}");
        assert!(with.contains("active") || with.contains("idle"));
    }

    /// A facts line wider than the panel ends in an ellipsis instead of being
    /// cut mid-word, and one call reads `1 tool`.
    #[test]
    fn detail_facts_line_ends_with_ellipsis() {
        let theme = all_themes()[0];
        let model = fact(
            MAIN_ID,
            0,
            FactKind::Model("anthropic/claude-opus-5-with-a-long-suffix".into()),
        );
        let mut a = app(vec![(0, [vec![model], call(0, 0)].concat())]);
        let wide = render_detail(&mut a, &theme, 90, 10);
        assert!(wide[2].contains("· 1 tool ·"), "{wide:#?}");
        let narrow = render_detail(&mut a, &theme, 40, 10);
        let facts = narrow[2].trim_end_matches('│').trim_end();
        assert!(facts.ends_with('…'), "{narrow:#?}");
    }

    #[test]
    fn a_long_list_tails_the_newest_call_when_following() {
        for theme in all_themes() {
            let steps = (0..30).map(|n| (n as i64, call(n as i64, n))).collect();
            let mut a = app(steps);
            let text = render_detail(&mut a, &theme, 60, 14).join("\n");
            assert!(text.contains("cmd29"), "{text}");
            assert!(!text.contains("cmd0 "), "{text}");
            assert!(text.contains("tail"), "{text}");
            assert!(a.selection.follow);
            assert!(a.selection.scroll > 0);

            // Detached at the top: the oldest call shows, the indicator names the offset.
            a.selection.follow = false;
            a.selection.scroll = 0;
            let text = render_detail(&mut a, &theme, 60, 14).join("\n");
            assert!(text.contains("cmd0"), "{text}");
            assert!(!text.contains("cmd29"), "{text}");
            assert!(text.contains("0/30"), "{text}");
            assert!(!a.selection.follow);
        }
    }

    #[test]
    fn stale_agents_and_tiny_areas_do_not_panic() {
        let theme = all_themes()[0];
        let mut a = app(vec![(0, call(0, 0))]);
        let gone = draw(40, 6, |f, r| render(f, r, &mut a, &theme, "nobody"));
        assert!(rows(&gone).join("\n").contains("No detail for this agent"));
        for (w, h) in [(0, 0), (1, 1), (2, 2), (10, 3), (40, 4), (40, 7)] {
            draw(w, h, |f, r| render(f, r, &mut a, &theme, MAIN_ID));
        }
    }
}
