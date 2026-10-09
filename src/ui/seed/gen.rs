//! Generator for `tokens.rs`, run as a test.
//!
//! Reads the SEED rootage YAML vendored under `design/seed/`, resolves the
//! `$token.reference` chains, and renders Rust source. `seed_tokens_match_vendored_yaml`
//! compares that source with the committed `tokens.rs`; with `UPDATE_SEED=1` it
//! rewrites the file instead.
//!
//! Two deliberate departures from the upstream values, both announced in the
//! generated header (Apache-2.0 §4(b)):
//! * every reference to a `carrot` palette step resolves to the same step of
//!   [`BRAND_PALETTE`];
//! * alpha colors are composited onto the same theme's `bg.layer-default`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use yaml_rust2::{Yaml, YamlLoader};

use super::tokens;

/// Palette the brand role is drawn from instead of `carrot`.
const BRAND_PALETTE: &str = "purple";
/// Palette whose references are replaced.
const REPLACED_PALETTE: &str = "carrot";
const THEMES: [&str; 2] = ["theme-light", "theme-dark"];
const BACKDROP: &str = "$color.bg.layer-default";

type Rgba = [u8; 4];

/// One YAML token file: tokens in file order, each with its per-mode values.
struct Doc {
    tokens: Vec<(String, HashMap<String, Yaml>)>,
    index: HashMap<String, usize>,
}

impl Doc {
    fn parse(src: &str) -> Result<Doc, String> {
        let docs = YamlLoader::load_from_str(src).map_err(|e| format!("invalid YAML: {e}"))?;
        let root = docs.first().ok_or("empty YAML document")?;
        let table = root["data"]["tokens"]
            .as_hash()
            .ok_or("missing data.tokens mapping")?;
        let mut tokens = Vec::new();
        let mut index = HashMap::new();
        for (key, body) in table {
            let name = key.as_str().ok_or("token name is not a string")?.to_owned();
            let values = body["values"]
                .as_hash()
                .ok_or_else(|| format!("token {name} has no values"))?;
            let mut modes = HashMap::new();
            for (mode, value) in values {
                let mode = mode.as_str().ok_or("mode name is not a string")?;
                modes.insert(mode.to_owned(), value.clone());
            }
            index.insert(name.clone(), tokens.len());
            tokens.push((name, modes));
        }
        Ok(Doc { tokens, index })
    }

    fn load(file: &str) -> Doc {
        let path = seed_dir().join(file);
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        Doc::parse(&src).unwrap_or_else(|e| panic!("{file}: {e}"))
    }

    /// Follow `$references` from `name` in `mode` to a concrete value.
    /// `carrot` palette references are swapped for the brand palette first.
    fn resolve(&self, name: &str, mode: &str, stack: &mut Vec<String>) -> Result<Yaml, String> {
        let name = swap_brand(name);
        if stack.contains(&name) {
            stack.push(name);
            return Err(format!("cyclic token reference: {}", stack.join(" -> ")));
        }
        let idx = *self
            .index
            .get(&name)
            .ok_or_else(|| format!("unknown token {name} (via {})", stack.join(" -> ")))?;
        let modes = &self.tokens[idx].1;
        // Global collections only define `default`.
        let value = modes
            .get(mode)
            .or_else(|| modes.get("default"))
            .ok_or_else(|| format!("token {name} has no value for {mode}"))?;
        match value.as_str() {
            Some(reference) if reference.starts_with('$') => {
                stack.push(name);
                let out = self.resolve(reference, mode, stack);
                stack.pop();
                out
            }
            _ => Ok(value.clone()),
        }
    }

    fn resolve_color(&self, name: &str, mode: &str) -> Result<Rgba, String> {
        let value = self.resolve(name, mode, &mut Vec::new())?;
        let text = value
            .as_str()
            .ok_or_else(|| format!("token {name}: color is not a string"))?;
        parse_hex(text).map_err(|e| format!("token {name}: {e}"))
    }

    /// Opaque color: alpha composited onto the theme's `bg.layer-default`.
    fn opaque_color(&self, name: &str, mode: &str) -> Result<[u8; 3], String> {
        let [r, g, b, a] = self.resolve_color(name, mode)?;
        if a == 255 {
            return Ok([r, g, b]);
        }
        let [back_r, back_g, back_b, back_a] = self.resolve_color(BACKDROP, mode)?;
        if back_a != 255 {
            return Err(format!("{BACKDROP} is not opaque in {mode}"));
        }
        Ok([
            composite(r, back_r, a),
            composite(g, back_g, a),
            composite(b, back_b, a),
        ])
    }
}

/// `fg * a + back * (1 - a)`, rounded to nearest.
fn composite(fg: u8, back: u8, alpha: u8) -> u8 {
    let (fg, back, alpha) = (u32::from(fg), u32::from(back), u32::from(alpha));
    ((fg * alpha + back * (255 - alpha) + 127) / 255) as u8
}

fn swap_brand(name: &str) -> String {
    let from = format!("$color.palette.{REPLACED_PALETTE}-");
    match name.strip_prefix(&from) {
        Some(step) => format!("$color.palette.{BRAND_PALETTE}-{step}"),
        None => name.to_owned(),
    }
}

fn parse_hex(text: &str) -> Result<Rgba, String> {
    let digits = text
        .strip_prefix('#')
        .ok_or_else(|| format!("unsupported color {text:?}"))?;
    if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
        return Err(format!("unsupported color {text:?}"));
    }
    let byte = |i: usize| {
        u8::from_str_radix(&digits[i..i + 2], 16).map_err(|e| format!("color {text:?}: {e}"))
    };
    Ok([
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if digits.len() == 8 { byte(6)? } else { 255 },
    ])
}

fn seed_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("design/seed")
}

/// Whether the YAML is here. The published crate ships only the license and
/// notice from `design/seed/`, so the generator tests skip (loudly) there.
fn vendored() -> bool {
    let here = seed_dir().join("color.yaml").is_file();
    if !here {
        eprintln!(
            "skipping: no SEED YAML at {} (not a git checkout)",
            seed_dir().display()
        );
    }
    here
}

fn source_commit() -> String {
    let path = seed_dir().join("SOURCE.md");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    text.lines()
        .find_map(|l| l.strip_prefix("- Commit: `")?.strip_suffix('`'))
        .unwrap_or_else(|| panic!("{path:?}: no `- Commit: ` line"))
        .to_owned()
}

// ---- xterm-256 nearest -----------------------------------------------------

fn xterm_rgb(index: u8) -> [u8; 3] {
    let tokens::Rgb(r, g, b) = super::theme::xterm_rgb(index);
    [r, g, b]
}

fn lab(rgb: [u8; 3]) -> [f64; 3] {
    let lin = rgb.map(|c| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let x = (0.4124564 * lin[0] + 0.3575761 * lin[1] + 0.1804375 * lin[2]) / 0.95047;
    let y = 0.2126729 * lin[0] + 0.7151522 * lin[1] + 0.0721750 * lin[2];
    let z = (0.0193339 * lin[0] + 0.1191920 * lin[1] + 0.9503041 * lin[2]) / 1.08883;
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// Nearest of xterm indices 16..=255 by CIE76 distance (ties: lowest index).
fn nearest_256(rgb: [u8; 3]) -> u8 {
    let target = lab(rgb);
    let mut best = (f64::MAX, 16u8);
    for index in 16..=255u8 {
        let candidate = lab(xterm_rgb(index));
        let dist: f64 = (0..3).map(|i| (candidate[i] - target[i]).powi(2)).sum();
        if dist < best.0 {
            best = (dist, index);
        }
    }
    best.1
}

// ---- rendering -------------------------------------------------------------

fn const_name(raw: &str, family_initial: char) -> String {
    let mut name: String = raw
        .chars()
        .map(|c| {
            if c == '-' || c == '.' {
                '_'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, family_initial.to_ascii_uppercase());
    }
    name
}

struct Semantic {
    /// Module the constant is declared in: `fg`, `bg`, `stroke`, or `brand`.
    module: &'static str,
    ident: String,
    seed_name: String,
    light: [u8; 3],
    dark: [u8; 3],
}

fn semantic_tokens(doc: &Doc) -> Result<Vec<Semantic>, String> {
    let mut out: Vec<Semantic> = Vec::new();
    for family in ["fg", "bg", "stroke"] {
        let prefix = format!("$color.{family}.");
        for (name, _) in &doc.tokens {
            let Some(rest) = name.strip_prefix(&prefix) else {
                continue;
            };
            let is_brand = rest.contains("brand");
            let (module, ident) = if is_brand {
                let tail: Vec<&str> = rest.split('-').filter(|p| *p != "brand").collect();
                let joined = std::iter::once(family)
                    .chain(tail.iter().copied())
                    .collect::<Vec<_>>()
                    .join("_");
                ("brand", const_name(&joined, 'b'))
            } else {
                (
                    family_module(family),
                    const_name(rest, family.chars().next().unwrap()),
                )
            };
            if out.iter().any(|t| t.module == module && t.ident == ident) {
                return Err(format!("duplicate constant {module}::{ident} from {name}"));
            }
            out.push(Semantic {
                module,
                ident,
                seed_name: format!("{family}.{rest}"),
                light: doc.opaque_color(name, THEMES[0])?,
                dark: doc.opaque_color(name, THEMES[1])?,
            });
        }
    }
    Ok(out)
}

fn family_module(family: &str) -> &'static str {
    match family {
        "fg" => "fg",
        "bg" => "bg",
        _ => "stroke",
    }
}

fn rgb_lit(c: [u8; 3]) -> String {
    format!("Rgb(0x{:02x}, 0x{:02x}, 0x{:02x})", c[0], c[1], c[2])
}

/// `(name, value)` of every token in a global-collection YAML file.
fn global_tokens(file: &str, prefix: &str) -> Result<Vec<(String, Yaml)>, String> {
    let doc = Doc::load(file);
    let mut out = Vec::new();
    for (name, _) in &doc.tokens {
        let Some(rest) = name.strip_prefix(prefix) else {
            return Err(format!("{file}: unexpected token {name}"));
        };
        let value = doc.resolve(name, "default", &mut Vec::new())?;
        out.push((const_name(rest, 'x'), value));
    }
    Ok(out)
}

fn px(file: &str, name: &str, value: &Yaml) -> Result<u16, String> {
    let text = value
        .as_str()
        .and_then(|t| t.strip_suffix("px"))
        .ok_or_else(|| format!("{file}: {name} is not a px value"))?;
    let n: u32 = text.parse().map_err(|e| format!("{file}: {name}: {e}"))?;
    Ok(u16::try_from(n).unwrap_or(u16::MAX))
}

fn generate() -> Result<String, String> {
    let color = Doc::load("color.yaml");
    let semantics = semantic_tokens(&color)?;
    let commit = source_commit();

    let mut out = String::new();
    writeln!(
        out,
        "// Generated from SEED Design (daangn/seed-design@{commit}, Apache-2.0) rootage tokens;\n\
         // brand role's carrot references replaced with same-step {BRAND_PALETTE}; alpha tokens composited to opaque\n\
         // (Apache-2.0 §4(b) modification notice); do not edit, run UPDATE_SEED=1 cargo test --locked seed_tokens.\n\
         #![allow(dead_code)]\n"
    )
    .unwrap();
    out.push_str(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n\
         pub struct Rgb(pub u8, pub u8, pub u8);\n\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n\
         pub struct Themed {\n    pub name: &'static str,\n    pub light: Rgb,\n    pub dark: Rgb,\n    pub light_256: u8,\n    pub dark_256: u8,\n}\n\n",
    );

    let mut all = Vec::new();
    for module in ["fg", "bg", "stroke", "brand"] {
        writeln!(out, "pub mod {module} {{\n    use super::*;\n").unwrap();
        for t in semantics.iter().filter(|t| t.module == module) {
            writeln!(
                out,
                "    pub const {}: Themed = Themed {{ name: {:?}, light: {}, dark: {}, light_256: {}, dark_256: {} }};",
                t.ident,
                t.seed_name,
                rgb_lit(t.light),
                rgb_lit(t.dark),
                nearest_256(t.light),
                nearest_256(t.dark),
            )
            .unwrap();
            all.push(format!("{module}::{}", t.ident));
        }
        out.push_str("}\n\n");
    }
    out.push_str("pub const ALL: &[Themed] = &[\n");
    for path in &all {
        writeln!(out, "    {path},").unwrap();
    }
    out.push_str("];\n\n");

    // duration
    out.push_str("pub mod duration {\n    use std::time::Duration;\n\n");
    for (name, value) in global_tokens("duration.yaml", "$duration.")? {
        let text = value.as_str().ok_or("duration is not a string")?;
        let ms: u64 = if let Some(ms) = text.strip_suffix("ms") {
            ms.parse().map_err(|e| format!("{name}: {e}"))?
        } else if let Some(s) = text.strip_suffix('s') {
            s.parse::<u64>().map_err(|e| format!("{name}: {e}"))? * 1000
        } else {
            return Err(format!("{name}: unsupported duration {text:?}"));
        };
        writeln!(
            out,
            "    pub const {name}: Duration = Duration::from_millis({ms});"
        )
        .unwrap();
    }
    out.push_str("}\n\n");

    // timing_function
    out.push_str("pub mod timing_function {\n");
    for (name, value) in global_tokens("timing-function.yaml", "$timing-function.")? {
        let points = value["value"]
            .as_vec()
            .filter(|p| p.len() == 4)
            .ok_or_else(|| format!("{name}: not a cubic-bezier with 4 points"))?;
        let nums: Result<Vec<String>, String> = points
            .iter()
            .map(|p| {
                let n = p
                    .as_f64()
                    .or_else(|| p.as_i64().map(|i| i as f64))
                    .ok_or_else(|| format!("{name}: non-numeric control point"))?;
                Ok(format!("{n:?}"))
            })
            .collect();
        writeln!(
            out,
            "    pub const {name}: [f32; 4] = [{}];",
            nums?.join(", ")
        )
        .unwrap();
    }
    out.push_str("}\n\n");

    // dimension / radius / font_weight
    for (module, file, prefix) in [
        ("dimension", "dimension.yaml", "$dimension."),
        ("radius", "radius.yaml", "$radius."),
    ] {
        writeln!(out, "pub mod {module} {{").unwrap();
        for (name, value) in global_tokens(file, prefix)? {
            let px = px(file, &name, &value)?;
            if px == u16::MAX || (module == "radius" && name == "FULL") {
                writeln!(out, "    pub const {name}: u16 = u16::MAX;").unwrap();
            } else {
                writeln!(out, "    pub const {name}: u16 = {px};").unwrap();
            }
        }
        out.push_str("}\n\n");
    }
    out.push_str("pub mod font_weight {\n");
    for (name, value) in global_tokens("font-weight.yaml", "$font-weight.")? {
        let weight = value
            .as_i64()
            .and_then(|w| u16::try_from(w).ok())
            .ok_or_else(|| format!("font-weight {name}: not a u16"))?;
        writeln!(out, "    pub const {name}: u16 = {weight};").unwrap();
    }
    out.push_str("}\n");
    Ok(out)
}

// ---- tests -----------------------------------------------------------------

#[test]
fn seed_tokens_match_vendored_yaml() {
    if !vendored() {
        return;
    }
    let generated = generate().unwrap_or_else(|e| panic!("{e}"));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ui/seed/tokens.rs");
    if std::env::var_os("UPDATE_SEED").is_some_and(|v| v == "1") {
        std::fs::write(&path, generated).unwrap();
        return;
    }
    // A Windows checkout may carry CRLF; the module is compared by content.
    let committed = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        committed == generated,
        "src/ui/seed/tokens.rs is stale; run UPDATE_SEED=1 cargo test --locked seed_tokens"
    );
}

fn rgb(c: [u8; 3]) -> tokens::Rgb {
    tokens::Rgb(c[0], c[1], c[2])
}

#[test]
fn brand_maps_to_purple_not_carrot() {
    if !vendored() {
        return;
    }
    let color = Doc::load("color.yaml");
    let palette = |step: &str, mode: &str| {
        let [r, g, b, a] = color
            .resolve_color(&format!("$color.palette.{step}"), mode)
            .unwrap();
        assert_eq!(a, 255, "{step} is not opaque");
        [r, g, b]
    };
    assert_eq!(
        tokens::brand::BG_SOLID.light,
        rgb(palette("purple-600", THEMES[0]))
    );
    assert_eq!(
        tokens::brand::BG_SOLID.dark,
        rgb(palette("purple-700", THEMES[1]))
    );

    // Read the carrot values straight from the YAML (no brand swap applies to
    // `resolve` of a raw palette token's own hex value).
    let raw = Doc::parse(&std::fs::read_to_string(seed_dir().join("color.yaml")).unwrap()).unwrap();
    let mut carrots = Vec::new();
    for (name, modes) in &raw.tokens {
        if name.starts_with(&format!("$color.palette.{REPLACED_PALETTE}-")) {
            for mode in THEMES {
                let hex = modes[mode].as_str().unwrap();
                let [r, g, b, _] = parse_hex(hex).unwrap();
                carrots.push(tokens::Rgb(r, g, b));
            }
        }
    }
    assert!(!carrots.is_empty(), "no carrot palette found");
    for t in tokens::ALL {
        assert!(
            !carrots.contains(&t.light),
            "{} light is a carrot value",
            t.name
        );
        assert!(
            !carrots.contains(&t.dark),
            "{} dark is a carrot value",
            t.name
        );
    }
}

#[test]
fn alpha_tokens_are_composited() {
    if !vendored() {
        return;
    }
    let color = Doc::load("color.yaml");
    // static-black-alpha-700 is #00000074 in both themes.
    let alpha = u32::from(
        color
            .resolve_color("$color.palette.static-black-alpha-700", THEMES[0])
            .unwrap()[3],
    );
    assert!(alpha < 255, "overlay source must carry alpha");
    for (mode, layer, got) in [
        (
            THEMES[0],
            tokens::bg::LAYER_DEFAULT.light,
            tokens::bg::OVERLAY.light,
        ),
        (
            THEMES[1],
            tokens::bg::LAYER_DEFAULT.dark,
            tokens::bg::OVERLAY.dark,
        ),
    ] {
        let [r, g, b, a] = color.resolve_color("$color.bg.overlay", mode).unwrap();
        assert_eq!([r, g, b], [0, 0, 0], "overlay is black");
        // Black over `layer`: layer * (255 - a) / 255, rounded.
        let expect = |c: u8| ((u32::from(c) * (255 - u32::from(a)) + 127) / 255) as u8;
        assert_eq!(
            got,
            tokens::Rgb(expect(layer.0), expect(layer.1), expect(layer.2)),
            "{mode}"
        );
    }
    assert!(tokens::bg::OVERLAY.name == "bg.overlay");
}

#[test]
fn cyclic_reference_is_reported() {
    let src = "\
kind: Tokens
data:
  tokens:
    $color.fg.a:
      values:
        theme-light: $color.fg.b
    $color.fg.b:
      values:
        theme-light: $color.fg.a
";
    let doc = Doc::parse(src).unwrap();
    let err = doc.resolve_color("$color.fg.a", "theme-light").unwrap_err();
    println!("{err}");
    assert!(err.contains("$color.fg.a"), "{err}");
    assert!(err.contains("cyclic"), "{err}");
}
