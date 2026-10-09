//! Buffer snapshots in token names (test-only).
//!
//! A snapshot golden records what a frame says and which SEED token paints
//! each run of cells — not the RGB values, so retuning a token's value does
//! not churn every golden, while painting a cell with the wrong role (or with
//! a color that is no token at all) does. Lives here, next to the tokens, so
//! the only `Color` matching in the ui stays inside `src/ui/seed/`.

use std::collections::HashMap;
use std::fmt::Write;

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use super::theme::Theme;
use super::tokens;

/// Which side of a cell a colour paints.
#[derive(Clone, Copy)]
enum Role {
    Fg,
    Bg,
}

/// How fitting a token's name is for `role`; lower wins. Many tokens share a
/// value, so a colour is named after the token most likely meant: text
/// colours after `fg.*` (then strokes), backgrounds after the layers, then
/// `bg.*`.
fn rank(name: &str, role: Role) -> u8 {
    match role {
        Role::Fg if name.starts_with("fg.neutral") => 0,
        Role::Fg if name.starts_with("fg.") => 1,
        Role::Fg if name.starts_with("stroke.") => 2,
        Role::Bg if name.starts_with("bg.layer-") => 0,
        Role::Bg if name.starts_with("bg.neutral") => 1,
        Role::Bg if name.starts_with("bg.") => 2,
        _ => 3,
    }
}

/// Every colour `theme` can paint on `role`, named after the best-ranked
/// token (ties: declaration order) that resolves to it.
fn names(theme: &Theme, role: Role) -> HashMap<Color, &'static str> {
    let mut ranked: Vec<_> = tokens::ALL.iter().collect();
    ranked.sort_by_key(|t| rank(t.name, role));
    let mut map = HashMap::new();
    for t in ranked {
        map.entry(theme.color(*t)).or_insert(t.name);
    }
    map
}

/// `buf` as text: its rows, then one line per run of cells sharing a style,
/// `row:from-to fg=<token> bg=<token> <modifiers>`. Panics on a color that is
/// no token of `theme` — every cell must be painted from the design system.
pub fn serialize(buf: &Buffer, theme: &Theme) -> String {
    let fg_names = names(theme, Role::Fg);
    let bg_names = names(theme, Role::Bg);
    let name = |c: Option<Color>, role: Role, at: (u16, u16)| -> &'static str {
        let names = match role {
            Role::Fg => &fg_names,
            Role::Bg => &bg_names,
        };
        match c {
            None | Some(Color::Reset) => "-",
            Some(c) => names.get(&c).copied().unwrap_or_else(|| {
                panic!("cell {at:?} is painted {c:?}, which is no SEED token of {theme:?}")
            }),
        }
    };
    let area = buf.area;
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        let mut line = String::new();
        for x in area.left()..area.right() {
            line.push_str(buf[(x, y)].symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.push_str("---\n");
    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            let cell = &buf[(x, y)];
            let key = (cell.fg, cell.bg, cell.modifier);
            let start = x;
            while x < area.right() {
                let c = &buf[(x, y)];
                if (c.fg, c.bg, c.modifier) != key {
                    break;
                }
                x += 1;
            }
            let fg = name(Some(key.0), Role::Fg, (start, y));
            let bg = name(Some(key.1), Role::Bg, (start, y));
            let _ = writeln!(
                out,
                "{y}:{start}-{} fg={fg} bg={bg}{}",
                x - 1,
                modifiers(key.2)
            );
        }
    }
    out
}

fn modifiers(m: Modifier) -> String {
    let mut s = String::new();
    for (flag, tag) in [
        (Modifier::BOLD, " bold"),
        (Modifier::DIM, " dim"),
        (Modifier::ITALIC, " italic"),
        (Modifier::UNDERLINED, " underlined"),
        (Modifier::REVERSED, " reversed"),
    ] {
        if m.contains(flag) {
            s.push_str(tag);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::theme::{Depth, Mode};
    use ratatui::layout::Rect;

    #[test]
    fn runs_are_named_after_tokens() {
        let theme = Theme::new(Mode::Dark, Depth::TrueColor);
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string(0, 0, "ab", theme.fg(tokens::fg::NEUTRAL));
        let text = serialize(&buf, &theme);
        assert!(text.starts_with("ab\n---\n"), "{text}");
        assert!(text.contains("0:0-1 fg=fg.neutral bg=-"), "{text}");
        assert!(text.contains("0:2-3 fg=- bg=-"), "{text}");
    }

    #[test]
    #[should_panic(expected = "no SEED token")]
    fn a_color_outside_the_tokens_fails() {
        let theme = Theme::new(Mode::Light, Depth::TrueColor);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        buf[(0, 0)].set_fg(Color::Rgb(1, 2, 3));
        serialize(&buf, &theme);
    }
}
