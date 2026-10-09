//! Theme resolution: which SEED token column (light/dark × truecolor/256) a
//! frame is drawn with, and the semantic styles built from it.
//!
//! A [`Theme`] is `Copy` and passed by value/reference into every render fn;
//! nothing stores it in application state. All `Color` values in the UI come
//! from [`Theme::color`] (or the helpers built on it) — this file is the only
//! place that constructs `Color::Rgb` / `Color::Indexed` / `Color::Reset`.

use ratatui::style::{Color, Style};

use super::tokens::{self, Rgb, Themed};
use crate::state::session::{AgentStatus, ToolState};

/// Light or dark token column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Light,
    Dark,
}

/// Color depth the terminal can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Depth {
    /// 24-bit `Color::Rgb`.
    TrueColor,
    /// xterm-256 `Color::Indexed` (the `*_256` token column).
    Ansi256,
}

/// Resolved theme for one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub mode: Mode,
    pub depth: Depth,
    /// Paint `bg.layer-*` backgrounds. `false` leaves the terminal's own
    /// background untouched (`AG_BG=none`).
    pub paint_bg: bool,
}

/// SEED semantic role of a badge/callout/status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tone {
    Neutral,
    Brand,
    Positive,
    Critical,
    Warning,
    Informative,
}

/// The colors one [`Tone`] contributes, per SEED badge/callout specs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToneColors {
    /// `fg.X` — glyphs, outline text, accent bars.
    pub fg: Color,
    /// `fg.X-contrast` — text on `weak_bg` (SEED badge `weak` / callout).
    pub fg_contrast: Color,
    /// `bg.X-weak`.
    pub weak_bg: Color,
    /// `bg.X-solid`.
    pub solid_bg: Color,
    /// `fg.on-X-solid` — text on `solid_bg`.
    pub on_solid: Color,
}

impl Theme {
    pub const fn new(mode: Mode, depth: Depth) -> Self {
        Self {
            mode,
            depth,
            paint_bg: true,
        }
    }

    /// Token value for this theme: `Rgb` in truecolor, `Indexed` in 256 colors.
    pub fn color(&self, t: Themed) -> Color {
        match (self.depth, self.mode) {
            (Depth::TrueColor, Mode::Light) => Color::Rgb(t.light.0, t.light.1, t.light.2),
            (Depth::TrueColor, Mode::Dark) => Color::Rgb(t.dark.0, t.dark.1, t.dark.2),
            (Depth::Ansi256, Mode::Light) => Color::Indexed(t.light_256),
            (Depth::Ansi256, Mode::Dark) => Color::Indexed(t.dark_256),
        }
    }

    /// The RGB a terminal shows for `t` in this theme (xterm palette for 256).
    #[cfg(test)]
    pub fn resolved_rgb(&self, t: Themed) -> Rgb {
        match (self.depth, self.mode) {
            (Depth::TrueColor, Mode::Light) => t.light,
            (Depth::TrueColor, Mode::Dark) => t.dark,
            (Depth::Ansi256, Mode::Light) => xterm_rgb(t.light_256),
            (Depth::Ansi256, Mode::Dark) => xterm_rgb(t.dark_256),
        }
    }

    pub fn fg(&self, t: Themed) -> Style {
        Style::default().fg(self.color(t))
    }

    pub fn bg(&self, t: Themed) -> Style {
        Style::default().bg(self.color(t))
    }

    fn layer(&self, bg: Themed) -> Style {
        let style = self.fg(tokens::fg::NEUTRAL);
        if self.paint_bg {
            style.bg(self.color(bg))
        } else {
            style
        }
    }

    /// App canvas: `fg.neutral` on `bg.layer-basement`.
    pub fn base(&self) -> Style {
        self.layer(tokens::bg::LAYER_BASEMENT)
    }

    /// Content surface: `fg.neutral` on `bg.layer-default`.
    pub fn surface(&self) -> Style {
        self.layer(tokens::bg::LAYER_DEFAULT)
    }

    /// Floating layer (popovers, overlays): `fg.neutral` on `bg.layer-floating`.
    pub fn floating(&self) -> Style {
        self.layer(tokens::bg::LAYER_FLOATING)
    }

    pub fn tone(&self, tone: Tone) -> ToneColors {
        use tokens::{bg, brand, fg};
        let (f, fc, weak, solid, on_solid) = match tone {
            Tone::Neutral => (
                fg::NEUTRAL_MUTED,
                fg::NEUTRAL_MUTED,
                bg::NEUTRAL_WEAK,
                bg::NEUTRAL_SOLID,
                fg::ON_NEUTRAL_SOLID,
            ),
            Tone::Brand => (
                brand::FG,
                brand::FG_CONTRAST,
                brand::BG_WEAK,
                brand::BG_SOLID,
                brand::FG_ON_SOLID,
            ),
            Tone::Positive => (
                fg::POSITIVE,
                fg::POSITIVE_CONTRAST,
                bg::POSITIVE_WEAK,
                bg::POSITIVE_SOLID,
                fg::ON_POSITIVE_SOLID,
            ),
            Tone::Critical => (
                fg::CRITICAL,
                fg::CRITICAL_CONTRAST,
                bg::CRITICAL_WEAK,
                bg::CRITICAL_SOLID,
                fg::ON_CRITICAL_SOLID,
            ),
            Tone::Warning => (
                fg::WARNING,
                fg::WARNING_CONTRAST,
                bg::WARNING_WEAK,
                bg::WARNING_SOLID,
                fg::ON_WARNING_SOLID,
            ),
            Tone::Informative => (
                fg::INFORMATIVE,
                fg::INFORMATIVE_CONTRAST,
                bg::INFORMATIVE_WEAK,
                bg::INFORMATIVE_SOLID,
                fg::ON_INFORMATIVE_SOLID,
            ),
        };
        ToneColors {
            fg: self.color(f),
            fg_contrast: self.color(fc),
            weak_bg: self.color(weak),
            solid_bg: self.color(solid),
            on_solid: self.color(on_solid),
        }
    }

    /// The rataflow palette for the graph canvas, derived from this theme.
    pub fn palette(&self) -> rataflow::Palette {
        rataflow::Palette {
            canvas_bg: if self.paint_bg {
                self.color(tokens::bg::LAYER_BASEMENT)
            } else {
                Color::Reset
            },
            surface: self.color(tokens::bg::LAYER_DEFAULT),
            muted: self.color(tokens::fg::NEUTRAL_SUBTLE),
            subtle: self.color(tokens::stroke::NEUTRAL_WEAK),
            accent: self.color(tokens::brand::FG),
            text: self.color(tokens::fg::NEUTRAL),
            success: self.color(tokens::fg::POSITIVE),
            error: self.color(tokens::fg::CRITICAL),
        }
    }

    /// Inverse of [`Theme::palette`]: recover the theme a palette was built
    /// from. `None` for a palette that did not come from a SEED theme.
    pub fn from_palette(p: &rataflow::Palette) -> Option<Theme> {
        const MODES: [Mode; 2] = [Mode::Light, Mode::Dark];
        const DEPTHS: [Depth; 2] = [Depth::TrueColor, Depth::Ansi256];
        MODES.iter().find_map(|&mode| {
            DEPTHS.iter().find_map(|&depth| {
                let theme = Theme::new(mode, depth);
                let own = theme.palette();
                (own.surface == p.surface && own.text == p.text).then_some(Theme {
                    paint_bg: p.canvas_bg != Color::Reset,
                    ..theme
                })
            })
        })
    }

    /// Detect the theme of the controlling terminal: environment first, then
    /// an OSC 11 background query (up to 1 s), see [`decide`].
    #[cfg(feature = "native")]
    pub fn detect() -> Theme {
        decide(&EnvSnapshot::from_env(), || {
            terminal_colorsaurus::background_color(terminal_colorsaurus::QueryOptions::default())
                .ok()
                .map(|c| {
                    let (r, g, b) = c.scale_to_8bit();
                    Rgb(r, g, b)
                })
        })
    }
}

/// The environment variables theme detection reads, captured once so
/// [`decide`] is a pure function.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnvSnapshot {
    /// `AG_THEME`: `light` | `dark`.
    pub ag_theme: Option<String>,
    /// `AG_COLOR`: `256` | `truecolor` | `24bit`.
    pub ag_color: Option<String>,
    /// `AG_BG`: `none` leaves the terminal background alone.
    pub ag_bg: Option<String>,
    pub colorterm: Option<String>,
    pub colorfgbg: Option<String>,
}

impl EnvSnapshot {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok();
        Self {
            ag_theme: var("AG_THEME"),
            ag_color: var("AG_COLOR"),
            ag_bg: var("AG_BG"),
            colorterm: var("COLORTERM"),
            colorfgbg: var("COLORFGBG"),
        }
    }
}

fn is(value: &Option<String>, accepted: &[&str]) -> bool {
    value
        .as_deref()
        .is_some_and(|v| accepted.iter().any(|a| v.trim().eq_ignore_ascii_case(a)))
}

/// Pick the theme.
///
/// - mode: `AG_THEME=light|dark` (the terminal is never queried) > `query()`
///   (light when the background's luma exceeds 0.5) > `COLORFGBG` (background
///   index 7 or 15 is light) > dark.
/// - depth: `AG_COLOR=256` > `AG_COLOR=truecolor|24bit` > `COLORTERM`
///   `truecolor|24bit` > 256 colors.
/// - paint_bg: `AG_BG=none` turns it off.
pub fn decide(env: &EnvSnapshot, query: impl FnOnce() -> Option<Rgb>) -> Theme {
    let mode = if is(&env.ag_theme, &["light"]) {
        Mode::Light
    } else if is(&env.ag_theme, &["dark"]) {
        Mode::Dark
    } else if let Some(bg) = query() {
        mode_of_background(bg)
    } else if let Some(mode) = env.colorfgbg.as_deref().and_then(mode_of_colorfgbg) {
        mode
    } else {
        Mode::Dark
    };

    let depth = if is(&env.ag_color, &["256"]) {
        Depth::Ansi256
    } else if is(&env.ag_color, &["truecolor", "24bit"])
        || is(&env.colorterm, &["truecolor", "24bit"])
    {
        Depth::TrueColor
    } else {
        Depth::Ansi256
    };

    Theme {
        mode,
        depth,
        paint_bg: !is(&env.ag_bg, &["none"]),
    }
}

/// Rec. 709 luma of the gamma-encoded color, 0.0–1.0 — the perceptual midpoint
/// between a dark and a light terminal background sits at 0.5 here (the linear
/// WCAG luminance would put it near 0.18).
fn mode_of_background(Rgb(r, g, b): Rgb) -> Mode {
    let luma = (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
    if luma > 0.5 { Mode::Light } else { Mode::Dark }
}

/// `COLORFGBG` is `fg;bg` or `fg;default;bg`; the last field is the background
/// palette index. 7 and 15 are the light ANSI backgrounds.
fn mode_of_colorfgbg(value: &str) -> Option<Mode> {
    let bg: u8 = value.rsplit(';').next()?.trim().parse().ok()?;
    Some(if matches!(bg, 7 | 15) {
        Mode::Light
    } else {
        Mode::Dark
    })
}

/// Tone of an agent's status badge.
pub fn status_tone(status: AgentStatus) -> Tone {
    match status {
        AgentStatus::Running => Tone::Informative,
        AgentStatus::Done => Tone::Positive,
        AgentStatus::Failed => Tone::Critical,
        AgentStatus::Idle => Tone::Neutral,
        AgentStatus::Stopped => Tone::Warning,
    }
}

/// Tone of a tool call's state.
pub fn tool_tone(state: ToolState) -> Tone {
    match state {
        ToolState::Pending => Tone::Informative,
        ToolState::Ok => Tone::Neutral,
        ToolState::Err => Tone::Critical,
    }
}

/// WCAG 2.x contrast ratio, 1.0–21.0.
#[cfg(test)]
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    fn linear(c: u8) -> f64 {
        let c = f64::from(c) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn luminance(Rgb(r, g, b): Rgb) -> f64 {
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    }
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// The RGB xterm shows for a 256-color palette index.
#[cfg(test)]
pub fn xterm_rgb(index: u8) -> Rgb {
    const SYSTEM: [Rgb; 16] = [
        Rgb(0, 0, 0),
        Rgb(205, 0, 0),
        Rgb(0, 205, 0),
        Rgb(205, 205, 0),
        Rgb(0, 0, 238),
        Rgb(205, 0, 205),
        Rgb(0, 205, 205),
        Rgb(229, 229, 229),
        Rgb(127, 127, 127),
        Rgb(255, 0, 0),
        Rgb(0, 255, 0),
        Rgb(255, 255, 0),
        Rgb(92, 92, 255),
        Rgb(255, 0, 255),
        Rgb(0, 255, 255),
        Rgb(255, 255, 255),
    ];
    const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        0..=15 => SYSTEM[usize::from(index)],
        16..=231 => {
            let n = usize::from(index - 16);
            Rgb(CUBE[n / 36], CUBE[(n / 6) % 6], CUBE[n % 6])
        }
        232..=255 => {
            let v = 8 + 10 * (index - 232);
            Rgb(v, v, v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::seed::tokens::{bg, brand, fg};

    fn env(f: impl FnOnce(&mut EnvSnapshot)) -> EnvSnapshot {
        let mut e = EnvSnapshot::default();
        f(&mut e);
        e
    }

    #[test]
    fn decide_prefers_ag_theme_without_querying() {
        let light = decide(&env(|e| e.ag_theme = Some("light".into())), || {
            panic!("AG_THEME must skip the terminal query")
        });
        assert_eq!(light.mode, Mode::Light);
        let dark = decide(
            &env(|e| {
                e.ag_theme = Some(" Dark ".into());
                e.colorfgbg = Some("0;15".into());
            }),
            || panic!("AG_THEME must skip the terminal query"),
        );
        assert_eq!(dark.mode, Mode::Dark);
    }

    #[test]
    fn decide_uses_query_then_colorfgbg_then_dark() {
        let light_bg = Some(Rgb(0xfa, 0xfa, 0xfa));
        let dark_bg = Some(Rgb(0x10, 0x10, 0x14));
        // The query wins over COLORFGBG.
        let with_fgbg = env(|e| e.colorfgbg = Some("15;0".into()));
        assert_eq!(decide(&with_fgbg, || light_bg).mode, Mode::Light);
        let with_light_fgbg = env(|e| e.colorfgbg = Some("0;15".into()));
        assert_eq!(decide(&with_light_fgbg, || dark_bg).mode, Mode::Dark);
        // No answer: COLORFGBG (three-field form too), then dark.
        assert_eq!(decide(&with_light_fgbg, || None).mode, Mode::Light);
        let three = env(|e| e.colorfgbg = Some("0;default;7".into()));
        assert_eq!(decide(&three, || None).mode, Mode::Light);
        assert_eq!(decide(&with_fgbg, || None).mode, Mode::Dark);
        let junk = env(|e| e.colorfgbg = Some("garbage".into()));
        assert_eq!(decide(&junk, || None).mode, Mode::Dark);
        assert_eq!(decide(&EnvSnapshot::default(), || None).mode, Mode::Dark);
    }

    #[test]
    fn ansi256_fallback() {
        let theme = decide(&EnvSnapshot::default(), || None);
        assert_eq!(theme.depth, Depth::Ansi256);
        assert!(matches!(theme.color(fg::NEUTRAL), Color::Indexed(_)));
        assert!(matches!(theme.color(bg::LAYER_DEFAULT), Color::Indexed(_)));
        // AG_COLOR=256 beats a truecolor-advertising terminal.
        let forced = decide(
            &env(|e| {
                e.ag_color = Some("256".into());
                e.colorterm = Some("truecolor".into());
            }),
            || None,
        );
        assert_eq!(forced.depth, Depth::Ansi256);
    }

    #[test]
    fn truecolor_when_colorterm_says_so() {
        for value in ["truecolor", "24bit"] {
            let theme = decide(&env(|e| e.colorterm = Some(value.into())), || None);
            assert_eq!(theme.depth, Depth::TrueColor, "COLORTERM={value}");
            assert!(matches!(theme.color(fg::NEUTRAL), Color::Rgb(..)));
        }
        let forced = decide(&env(|e| e.ag_color = Some("24bit".into())), || None);
        assert_eq!(forced.depth, Depth::TrueColor);
    }

    #[test]
    fn ag_bg_none_drops_backgrounds() {
        let theme = decide(&env(|e| e.ag_bg = Some("none".into())), || None);
        assert!(!theme.paint_bg);
        assert_eq!(theme.base().bg, None);
        assert_eq!(theme.surface().bg, None);
        assert_eq!(theme.palette().canvas_bg, Color::Reset);
        assert!(decide(&EnvSnapshot::default(), || None).paint_bg);
        assert!(Theme::new(Mode::Dark, Depth::TrueColor).base().bg.is_some());
    }

    #[test]
    fn light_and_dark_resolve_their_own_token_values() {
        let light = Theme::new(Mode::Light, Depth::TrueColor);
        let dark = Theme::new(Mode::Dark, Depth::TrueColor);
        let t = fg::NEUTRAL;
        assert_eq!(light.color(t), Color::Rgb(t.light.0, t.light.1, t.light.2));
        assert_eq!(dark.color(t), Color::Rgb(t.dark.0, t.dark.1, t.dark.2));
        assert_ne!(light.color(t), dark.color(t));
        let l256 = Theme::new(Mode::Light, Depth::Ansi256);
        let d256 = Theme::new(Mode::Dark, Depth::Ansi256);
        assert_eq!(l256.color(t), Color::Indexed(t.light_256));
        assert_eq!(d256.color(t), Color::Indexed(t.dark_256));
        assert_eq!(light.surface().bg, Some(light.color(bg::LAYER_DEFAULT)));
        assert_eq!(dark.base().bg, Some(dark.color(bg::LAYER_BASEMENT)));
        assert_eq!(light.floating().bg, Some(light.color(bg::LAYER_FLOATING)));
        assert_eq!(dark.tone(Tone::Brand).weak_bg, dark.color(brand::BG_WEAK));
        assert_eq!(light.tone(Tone::Neutral).fg, light.color(fg::NEUTRAL_MUTED));
    }

    #[test]
    fn palette_round_trips_through_from_palette() {
        for mode in [Mode::Light, Mode::Dark] {
            for depth in [Depth::TrueColor, Depth::Ansi256] {
                for paint_bg in [true, false] {
                    let theme = Theme {
                        mode,
                        depth,
                        paint_bg,
                    };
                    assert_eq!(
                        Theme::from_palette(&theme.palette()),
                        Some(theme),
                        "{theme:?}"
                    );
                }
            }
        }
        assert_eq!(Theme::from_palette(&rataflow::Palette::DARK), None);
    }

    #[test]
    fn status_and_tool_tones() {
        assert_eq!(status_tone(AgentStatus::Running), Tone::Informative);
        assert_eq!(status_tone(AgentStatus::Done), Tone::Positive);
        assert_eq!(status_tone(AgentStatus::Failed), Tone::Critical);
        assert_eq!(status_tone(AgentStatus::Idle), Tone::Neutral);
        assert_eq!(status_tone(AgentStatus::Stopped), Tone::Warning);
        assert_eq!(tool_tone(ToolState::Pending), Tone::Informative);
        assert_eq!(tool_tone(ToolState::Ok), Tone::Neutral);
        assert_eq!(tool_tone(ToolState::Err), Tone::Critical);
    }

    #[test]
    fn contrast_ratio_matches_wcag_reference_values() {
        let black = Rgb(0, 0, 0);
        let white = Rgb(255, 255, 255);
        assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-9);
        assert!((contrast_ratio(white, white) - 1.0).abs() < 1e-9);
        // #767676 on white is the canonical 4.54:1 AA threshold example.
        let ratio = contrast_ratio(Rgb(0x76, 0x76, 0x76), white);
        assert!((ratio - 4.54).abs() < 0.01, "{ratio}");
    }

    #[test]
    fn xterm_rgb_matches_known_indices() {
        assert_eq!(xterm_rgb(16), Rgb(0, 0, 0));
        assert_eq!(xterm_rgb(231), Rgb(255, 255, 255));
        assert_eq!(xterm_rgb(196), Rgb(255, 0, 0));
        assert_eq!(xterm_rgb(232), Rgb(8, 8, 8));
        assert_eq!(xterm_rgb(255), Rgb(238, 238, 238));
    }

    #[test]
    fn body_text_contrast_meets_wcag() {
        for mode in [Mode::Light, Mode::Dark] {
            for depth in [Depth::TrueColor, Depth::Ansi256] {
                let theme = Theme::new(mode, depth);
                let surface = theme.resolved_rgb(bg::LAYER_DEFAULT);
                let body = contrast_ratio(theme.resolved_rgb(fg::NEUTRAL), surface);
                let subtle = contrast_ratio(theme.resolved_rgb(fg::NEUTRAL_SUBTLE), surface);
                assert!(body >= 4.5, "fg.neutral {mode:?}/{depth:?} = {body:.2}");
                assert!(
                    subtle >= 3.0,
                    "fg.neutral-subtle {mode:?}/{depth:?} = {subtle:.2}"
                );
            }
        }
    }
}
