//! Text shaping shared by every view: width-aware truncation and wrapping
//! (CJK and emoji are two columns wide), and the compact number, duration,
//! clock and cost formats the screens agree on.

use chrono::{DateTime, FixedOffset, Utc};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::state::session::{AgentInfo, said};

/// Display columns of `s`.
pub(crate) fn width(s: &str) -> usize {
    s.width()
}

/// Truncate to `max` display columns with an ellipsis. Never panics on
/// multibyte input.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.width() <= max {
        return s.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }
    // Keep columns for the content up to max-1, reserving one for the ellipsis.
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > max - 1 {
            break;
        }
        w += cw;
        out.push(ch);
    }
    out.push('…');
    out
}

/// Like [`truncate`] but keeps the END (a path's basename), eliding the
/// front: `…timeline.rs`.
pub(crate) fn truncate_tail(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.width() <= max {
        return s.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }
    let mut w = 0;
    let mut start = s.len();
    for (i, ch) in s.char_indices().rev() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > max - 1 {
            break;
        }
        w += cw;
        start = i;
    }
    format!("…{}", &s[start..])
}

/// Greedy word-wrap to lines of at most `width` columns, capped at
/// `max_lines` (the last kept line gets a trailing `…` when content was
/// dropped). Over-long words are hard-split.
pub(crate) fn wrap(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for word in text.split_whitespace() {
        let wlen = word.width();
        let fits = if cur.is_empty() {
            wlen <= width
        } else {
            cur_w + 1 + wlen <= width
        };
        if fits {
            if !cur.is_empty() {
                cur.push(' ');
                cur_w += 1;
            }
            cur.push_str(word);
            cur_w += wlen;
        } else if wlen > width {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let mut rest = word;
            while rest.width() > width {
                let cut = split_at_width(rest, width);
                lines.push(rest[..cut].to_string());
                rest = &rest[cut..];
            }
            cur = rest.to_string();
            cur_w = cur.width();
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
            cur_w = wlen;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            *last = truncate(&format!("{last} …"), width);
        }
    }
    lines
}

/// Byte index splitting `s` at no more than `cols` columns — but at least
/// one char, so a char wider than `cols` still advances.
fn split_at_width(s: &str, cols: usize) -> usize {
    let mut w = 0;
    for (i, ch) in s.char_indices() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > cols && i > 0 {
            return i;
        }
        w += cw;
    }
    s.len()
}

/// Compact token count: `34`, `1.2k`, `2.0M`.
pub(crate) fn fmt_tokens(n: u64) -> String {
    if n < 1_000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

/// A tool-call count with its noun: `1 tool`, `0 tools`, `12 tools`.
pub(crate) fn fmt_tool_count(n: usize) -> String {
    if n == 1 {
        "1 tool".to_string()
    } else {
        format!("{n} tools")
    }
}

/// Compact duration at whole-second resolution: `12s`, `2m03s`, `1h05m`.
/// The resolution matches the redraw stamp: a timer changes once a second.
pub(crate) fn fmt_dur(d: chrono::Duration) -> String {
    let s = d.num_seconds().max(0);
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
    }
}

/// How long an agent has been at it: first to last recorded activity, or the
/// clock time it started when only that is known.
pub(crate) fn fmt_timing(agent: &AgentInfo, offset: FixedOffset) -> Option<String> {
    match (agent.first_ts, agent.last_ts) {
        (Some(first), Some(last)) => Some(fmt_dur(last - first)),
        (Some(first), None) => Some(format!("started {}", fmt_clock(first, offset))),
        _ => None,
    }
}

/// A recorded time as wall-clock `HH:MM:SS` in `offset`.
pub(crate) fn fmt_clock(ts: DateTime<Utc>, offset: FixedOffset) -> String {
    ts.with_timezone(&offset).format("%H:%M:%S").to_string()
}

/// A recorded time as `HH:MM` in `offset` (axis labels).
pub(crate) fn fmt_hm(ts: DateTime<Utc>, offset: FixedOffset) -> String {
    ts.with_timezone(&offset).format("%H:%M").to_string()
}

/// A recorded cost: `$1.84`, or `<$0.01` for a cost that rounds to nothing.
pub(crate) fn fmt_cost(usd: f64) -> String {
    if usd > 0.0 && usd < 0.005 {
        "<$0.01".to_string()
    } else {
        format!("${usd:.2}")
    }
}

/// What a tool call says about itself, cut to `max` columns: the agent's own
/// intent when stated, else the provider's summary. A path tool's summary
/// keeps its END (the file name); everything else keeps its head.
pub(crate) fn tool_text(
    name: &str,
    intent: Option<&str>,
    summary: Option<&str>,
    max: usize,
) -> Option<String> {
    if let Some(intent) = said(intent) {
        return Some(truncate(intent, max));
    }
    let summary = said(summary)?;
    Some(
        if matches!(name, "Read" | "Write" | "Edit" | "read" | "write" | "edit") {
            truncate_tail(summary, max)
        } else {
            truncate(summary, max)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn truncate_basic() {
        assert_eq!(truncate("session title", 7), "sessio…");
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("é😀x", 2), "é…");
    }

    #[test]
    fn tool_count_is_singular_for_one() {
        assert_eq!(fmt_tool_count(0), "0 tools");
        assert_eq!(fmt_tool_count(1), "1 tool");
        assert_eq!(fmt_tool_count(2), "2 tools");
    }

    #[test]
    fn wrap_word_wraps_and_caps() {
        assert_eq!(
            wrap("one two three four", 8, 5),
            ["one two", "three", "four"]
        );
        assert_eq!(wrap("aaa bbb ccc ddd", 3, 2), ["aaa", "bb…"]);
        assert_eq!(wrap("short", 20, 3), ["short"]);
    }

    #[test]
    fn truncate_tail_keeps_the_basename() {
        assert_eq!(truncate_tail("src/state/timeline.rs", 12), "…timeline.rs");
        assert_eq!(truncate_tail("Cargo.toml", 20), "Cargo.toml");
    }

    #[test]
    fn wide_text_stays_within_column_budgets() {
        let s = "日本語テスト"; // 6 chars, 12 columns
        assert!(truncate(s, 6).width() <= 6);
        assert!(truncate_tail(s, 6).width() <= 6);
        assert!(truncate(s, 10).ends_with('…'));
        assert_eq!(truncate(s, 12), s);
        for line in wrap("修复解析错误 and fix the parser", 6, usize::MAX) {
            assert!(line.width() <= 6, "{line:?}");
        }
    }

    #[test]
    fn token_formatting() {
        assert_eq!(fmt_tokens(0), "0");
        assert_eq!(fmt_tokens(999), "999");
        assert_eq!(fmt_tokens(1_500), "1.5k");
        assert_eq!(fmt_tokens(2_000_000), "2.0M");
    }

    #[test]
    fn durations_at_whole_seconds() {
        let s = chrono::Duration::seconds;
        assert_eq!(fmt_dur(s(-3)), "0s");
        assert_eq!(fmt_dur(s(42)), "42s");
        assert_eq!(fmt_dur(s(123)), "2m03s");
        assert_eq!(fmt_dur(s(3_900)), "1h05m");
    }

    #[test]
    fn timing_spans_first_to_last_activity() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let at = |s| Some(Utc.timestamp_opt(s, 0).unwrap());
        let mut a = AgentInfo::new(crate::state::session::AgentKind::Subagent);
        assert_eq!(fmt_timing(&a, utc), None);
        a.first_ts = at(100);
        a.last_ts = at(225);
        assert_eq!(fmt_timing(&a, utc).as_deref(), Some("2m05s"));
        a.last_ts = at(50);
        assert_eq!(fmt_timing(&a, utc).as_deref(), Some("0s"));
        a.last_ts = None;
        assert_eq!(fmt_timing(&a, utc).as_deref(), Some("started 00:01:40"));
    }

    #[test]
    fn clock_uses_the_display_offset() {
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, 23, 30, 5).unwrap();
        assert_eq!(fmt_clock(ts, FixedOffset::east_opt(0).unwrap()), "23:30:05");
        assert_eq!(
            fmt_clock(ts, FixedOffset::east_opt(9 * 3600).unwrap()),
            "08:30:05"
        );
    }

    #[test]
    fn cost_rounds_to_cents() {
        assert_eq!(fmt_cost(1.8449), "$1.84");
        assert_eq!(fmt_cost(0.001), "<$0.01");
        assert_eq!(fmt_cost(0.0), "$0.00");
    }
}
