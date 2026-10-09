// Generated from SEED Design (daangn/seed-design@22b68ce014c894edf286a2328a96250a36ab33a2, Apache-2.0) rootage tokens;
// brand role's carrot references replaced with same-step purple; alpha tokens composited to opaque
// (Apache-2.0 §4(b) modification notice); do not edit, run UPDATE_SEED=1 cargo test --locked seed_tokens.
#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Themed {
    pub name: &'static str,
    pub light: Rgb,
    pub dark: Rgb,
    pub light_256: u8,
    pub dark_256: u8,
}

pub mod fg {
    use super::*;

    pub const CRITICAL: Themed = Themed { name: "fg.critical", light: Rgb(0xfa, 0x34, 0x2c), dark: Rgb(0xff, 0x6e, 0x60), light_256: 160, dark_256: 203 };
    pub const CRITICAL_CONTRAST: Themed = Themed { name: "fg.critical-contrast", light: Rgb(0x92, 0x17, 0x08), dark: Rgb(0xf8, 0xc5, 0xc3), light_256: 88, dark_256: 224 };
    pub const DISABLED: Themed = Themed { name: "fg.disabled", light: Rgb(0xd1, 0xd3, 0xd8), dark: Rgb(0x5b, 0x60, 0x6a), light_256: 252, dark_256: 59 };
    pub const INFORMATIVE: Themed = Themed { name: "fg.informative", light: Rgb(0x21, 0x7c, 0xf9), dark: Rgb(0x41, 0xa2, 0xf9), light_256: 33, dark_256: 75 };
    pub const INFORMATIVE_CONTRAST: Themed = Themed { name: "fg.informative-contrast", light: Rgb(0x0b, 0x45, 0x96), dark: Rgb(0xb9, 0xd7, 0xfb), light_256: 25, dark_256: 153 };
    pub const NEUTRAL: Themed = Themed { name: "fg.neutral", light: Rgb(0x1a, 0x1c, 0x20), dark: Rgb(0xf3, 0xf4, 0xf5), light_256: 234, dark_256: 255 };
    pub const NEUTRAL_INVERTED: Themed = Themed { name: "fg.neutral-inverted", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0x16, 0x17, 0x1b), light_256: 231, dark_256: 233 };
    pub const NEUTRAL_MUTED: Themed = Themed { name: "fg.neutral-muted", light: Rgb(0x55, 0x5d, 0x6d), dark: Rgb(0xdc, 0xde, 0xe3), light_256: 59, dark_256: 253 };
    pub const NEUTRAL_SUBTLE: Themed = Themed { name: "fg.neutral-subtle", light: Rgb(0x86, 0x8b, 0x94), dark: Rgb(0xb0, 0xb3, 0xba), light_256: 245, dark_256: 249 };
    pub const ON_CRITICAL_SOLID: Themed = Themed { name: "fg.on-critical-solid", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0xff, 0xff, 0xff), light_256: 231, dark_256: 231 };
    pub const ON_INFORMATIVE_SOLID: Themed = Themed { name: "fg.on-informative-solid", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0xff, 0xff, 0xff), light_256: 231, dark_256: 231 };
    pub const ON_NEUTRAL_SOLID: Themed = Themed { name: "fg.on-neutral-solid", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0x16, 0x17, 0x1b), light_256: 231, dark_256: 233 };
    pub const ON_POSITIVE_SOLID: Themed = Themed { name: "fg.on-positive-solid", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0xff, 0xff, 0xff), light_256: 231, dark_256: 231 };
    pub const ON_WARNING_SOLID: Themed = Themed { name: "fg.on-warning-solid", light: Rgb(0x2f, 0x2f, 0x2f), dark: Rgb(0x04, 0x04, 0x05), light_256: 236, dark_256: 232 };
    pub const PLACEHOLDER: Themed = Themed { name: "fg.placeholder", light: Rgb(0xb0, 0xb3, 0xba), dark: Rgb(0x86, 0x8b, 0x94), light_256: 249, dark_256: 245 };
    pub const POSITIVE: Themed = Themed { name: "fg.positive", light: Rgb(0x07, 0x91, 0x71), dark: Rgb(0x22, 0xb2, 0x7f), light_256: 29, dark_256: 36 };
    pub const POSITIVE_CONTRAST: Themed = Themed { name: "fg.positive-contrast", light: Rgb(0x07, 0x54, 0x45), dark: Rgb(0x93, 0xe5, 0xc0), light_256: 23, dark_256: 115 };
    pub const WARNING: Themed = Themed { name: "fg.warning", light: Rgb(0x9b, 0x78, 0x21), dark: Rgb(0xca, 0x90, 0x1c), light_256: 94, dark_256: 136 };
    pub const WARNING_CONTRAST: Themed = Themed { name: "fg.warning-contrast", light: Rgb(0x4f, 0x3e, 0x1f), dark: Rgb(0xe5, 0xd4, 0x9b), light_256: 238, dark_256: 187 };
}

pub mod bg {
    use super::*;

    pub const CRITICAL_SOLID: Themed = Themed { name: "bg.critical-solid", light: Rgb(0xfa, 0x34, 0x2c), dark: Rgb(0xf7, 0x35, 0x26), light_256: 160, dark_256: 160 };
    pub const CRITICAL_SOLID_PRESSED: Themed = Themed { name: "bg.critical-solid-pressed", light: Rgb(0xca, 0x1d, 0x13), dark: Rgb(0xff, 0x6e, 0x60), light_256: 124, dark_256: 203 };
    pub const CRITICAL_WEAK: Themed = Themed { name: "bg.critical-weak", light: Rgb(0xfd, 0xf0, 0xf0), dark: Rgb(0x32, 0x23, 0x23), light_256: 255, dark_256: 235 };
    pub const CRITICAL_WEAK_PRESSED: Themed = Themed { name: "bg.critical-weak-pressed", light: Rgb(0xfd, 0xe7, 0xe7), dark: Rgb(0x4f, 0x26, 0x24), light_256: 255, dark_256: 236 };
    pub const DISABLED: Themed = Themed { name: "bg.disabled", light: Rgb(0xf3, 0xf4, 0xf5), dark: Rgb(0x2b, 0x2e, 0x35), light_256: 255, dark_256: 236 };
    pub const INFORMATIVE_SOLID: Themed = Themed { name: "bg.informative-solid", light: Rgb(0x21, 0x7c, 0xf9), dark: Rgb(0x1e, 0x82, 0xeb), light_256: 33, dark_256: 33 };
    pub const INFORMATIVE_SOLID_PRESSED: Themed = Themed { name: "bg.informative-solid-pressed", light: Rgb(0x13, 0x5f, 0xcd), dark: Rgb(0x41, 0xa2, 0xf9), light_256: 26, dark_256: 75 };
    pub const INFORMATIVE_WEAK: Themed = Themed { name: "bg.informative-weak", light: Rgb(0xef, 0xf6, 0xff), dark: Rgb(0x20, 0x27, 0x42), light_256: 255, dark_256: 235 };
    pub const INFORMATIVE_WEAK_PRESSED: Themed = Themed { name: "bg.informative-weak-pressed", light: Rgb(0xe2, 0xed, 0xfc), dark: Rgb(0x1e, 0x33, 0x52), light_256: 255, dark_256: 24 };
    pub const LAYER_BASEMENT: Themed = Themed { name: "bg.layer-basement", light: Rgb(0xf3, 0xf4, 0xf5), dark: Rgb(0x00, 0x00, 0x00), light_256: 255, dark_256: 16 };
    pub const LAYER_DEFAULT: Themed = Themed { name: "bg.layer-default", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0x16, 0x17, 0x1b), light_256: 231, dark_256: 233 };
    pub const LAYER_DEFAULT_PRESSED: Themed = Themed { name: "bg.layer-default-pressed", light: Rgb(0xf7, 0xf8, 0xf9), dark: Rgb(0x2b, 0x2e, 0x35), light_256: 231, dark_256: 236 };
    pub const LAYER_FLOATING: Themed = Themed { name: "bg.layer-floating", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0x1d, 0x20, 0x25), light_256: 231, dark_256: 234 };
    pub const LAYER_FLOATING_PRESSED: Themed = Themed { name: "bg.layer-floating-pressed", light: Rgb(0xf7, 0xf8, 0xf9), dark: Rgb(0x2b, 0x2e, 0x35), light_256: 231, dark_256: 236 };
    pub const MAGIC_WEAK: Themed = Themed { name: "bg.magic-weak", light: Rgb(0xf9, 0xf2, 0xee), dark: Rgb(0x20, 0x1f, 0x1f), light_256: 255, dark_256: 234 };
    pub const NEUTRAL_INVERTED: Themed = Themed { name: "bg.neutral-inverted", light: Rgb(0x2a, 0x30, 0x38), dark: Rgb(0xf3, 0xf4, 0xf5), light_256: 236, dark_256: 255 };
    pub const NEUTRAL_INVERTED_PRESSED: Themed = Themed { name: "bg.neutral-inverted-pressed", light: Rgb(0x55, 0x5d, 0x6d), dark: Rgb(0xdc, 0xde, 0xe3), light_256: 59, dark_256: 253 };
    pub const NEUTRAL_MUTED: Themed = Themed { name: "bg.neutral-muted", light: Rgb(0xf7, 0xf8, 0xf9), dark: Rgb(0x1d, 0x20, 0x25), light_256: 231, dark_256: 234 };
    pub const NEUTRAL_SOLID: Themed = Themed { name: "bg.neutral-solid", light: Rgb(0x2a, 0x30, 0x38), dark: Rgb(0xf3, 0xf4, 0xf5), light_256: 236, dark_256: 255 };
    pub const NEUTRAL_SOLID_PRESSED: Themed = Themed { name: "bg.neutral-solid-pressed", light: Rgb(0x55, 0x5d, 0x6d), dark: Rgb(0xdc, 0xde, 0xe3), light_256: 59, dark_256: 253 };
    pub const NEUTRAL_SOLID_MUTED: Themed = Themed { name: "bg.neutral-solid-muted", light: Rgb(0x55, 0x5d, 0x6d), dark: Rgb(0x39, 0x3d, 0x46), light_256: 59, dark_256: 237 };
    pub const NEUTRAL_SOLID_MUTED_PRESSED: Themed = Themed { name: "bg.neutral-solid-muted-pressed", light: Rgb(0x2a, 0x30, 0x38), dark: Rgb(0x5b, 0x60, 0x6a), light_256: 236, dark_256: 59 };
    pub const NEUTRAL_SUBTLE: Themed = Themed { name: "bg.neutral-subtle", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0x16, 0x17, 0x1b), light_256: 231, dark_256: 233 };
    pub const NEUTRAL_WEAK: Themed = Themed { name: "bg.neutral-weak", light: Rgb(0xf3, 0xf4, 0xf5), dark: Rgb(0x2b, 0x2e, 0x35), light_256: 255, dark_256: 236 };
    pub const NEUTRAL_WEAK_ALPHA: Themed = Themed { name: "bg.neutral-weak-alpha", light: Rgb(0xf3, 0xf3, 0xf3), dark: Rgb(0x33, 0x34, 0x38), light_256: 255, dark_256: 236 };
    pub const NEUTRAL_WEAK_ALPHA_PRESSED: Themed = Themed { name: "bg.neutral-weak-alpha-pressed", light: Rgb(0xef, 0xef, 0xef), dark: Rgb(0x40, 0x41, 0x44), light_256: 255, dark_256: 238 };
    pub const NEUTRAL_WEAK_PRESSED: Themed = Themed { name: "bg.neutral-weak-pressed", light: Rgb(0xee, 0xef, 0xf1), dark: Rgb(0x39, 0x3d, 0x46), light_256: 255, dark_256: 237 };
    pub const OVERLAY: Themed = Themed { name: "bg.overlay", light: Rgb(0x8b, 0x8b, 0x8b), dark: Rgb(0x0c, 0x0d, 0x0f), light_256: 245, dark_256: 232 };
    pub const OVERLAY_MUTED: Themed = Themed { name: "bg.overlay-muted", light: Rgb(0xd3, 0xd3, 0xd3), dark: Rgb(0x12, 0x13, 0x16), light_256: 252, dark_256: 233 };
    pub const POSITIVE_SOLID: Themed = Themed { name: "bg.positive-solid", light: Rgb(0x07, 0x91, 0x71), dark: Rgb(0x11, 0x79, 0x56), light_256: 29, dark_256: 29 };
    pub const POSITIVE_SOLID_PRESSED: Themed = Themed { name: "bg.positive-solid-pressed", light: Rgb(0x00, 0x74, 0x5f), dark: Rgb(0x1b, 0x94, 0x6d), light_256: 29, dark_256: 29 };
    pub const POSITIVE_WEAK: Themed = Themed { name: "bg.positive-weak", light: Rgb(0xed, 0xfa, 0xf6), dark: Rgb(0x20, 0x29, 0x26), light_256: 231, dark_256: 235 };
    pub const POSITIVE_WEAK_PRESSED: Themed = Themed { name: "bg.positive-weak-pressed", light: Rgb(0xd9, 0xf6, 0xe9), dark: Rgb(0x20, 0x36, 0x2e), light_256: 195, dark_256: 236 };
    pub const TRANSPARENT: Themed = Themed { name: "bg.transparent", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0x16, 0x17, 0x1b), light_256: 231, dark_256: 233 };
    pub const TRANSPARENT_PRESSED: Themed = Themed { name: "bg.transparent-pressed", light: Rgb(0xf8, 0xf8, 0xf8), dark: Rgb(0x22, 0x23, 0x27), light_256: 231, dark_256: 235 };
    pub const TRANSPARENT_SELECTED: Themed = Themed { name: "bg.transparent-selected", light: Rgb(0xf3, 0xf3, 0xf3), dark: Rgb(0x2b, 0x2c, 0x30), light_256: 255, dark_256: 236 };
    pub const TRANSPARENT_SELECTED_PRESSED: Themed = Themed { name: "bg.transparent-selected-pressed", light: Rgb(0xef, 0xef, 0xef), dark: Rgb(0x33, 0x34, 0x38), light_256: 255, dark_256: 236 };
    pub const WARNING_SOLID: Themed = Themed { name: "bg.warning-solid", light: Rgb(0xfb, 0xdc, 0x65), dark: Rgb(0xda, 0xb1, 0x56), light_256: 221, dark_256: 179 };
    pub const WARNING_SOLID_PRESSED: Themed = Themed { name: "bg.warning-solid-pressed", light: Rgb(0xe9, 0xc6, 0x47), dark: Rgb(0xe5, 0xd4, 0x9b), light_256: 221, dark_256: 187 };
    pub const WARNING_WEAK: Themed = Themed { name: "bg.warning-weak", light: Rgb(0xff, 0xf7, 0xde), dark: Rgb(0x30, 0x28, 0x19), light_256: 230, dark_256: 235 };
    pub const WARNING_WEAK_PRESSED: Themed = Themed { name: "bg.warning-weak-pressed", light: Rgb(0xfd, 0xef, 0xb9), dark: Rgb(0x41, 0x32, 0x18), light_256: 230, dark_256: 236 };
}

pub mod stroke {
    use super::*;

    pub const CRITICAL_SOLID: Themed = Themed { name: "stroke.critical-solid", light: Rgb(0xfa, 0x34, 0x2c), dark: Rgb(0xff, 0x6e, 0x60), light_256: 160, dark_256: 203 };
    pub const CRITICAL_WEAK: Themed = Themed { name: "stroke.critical-weak", light: Rgb(0xfe, 0xd4, 0xd2), dark: Rgb(0x74, 0x28, 0x26), light_256: 224, dark_256: 52 };
    pub const FOCUS_RING: Themed = Themed { name: "stroke.focus-ring", light: Rgb(0x5e, 0x98, 0xfe), dark: Rgb(0x1e, 0x82, 0xeb), light_256: 68, dark_256: 33 };
    pub const INFORMATIVE_SOLID: Themed = Themed { name: "stroke.informative-solid", light: Rgb(0x21, 0x7c, 0xf9), dark: Rgb(0x41, 0xa2, 0xf9), light_256: 33, dark_256: 75 };
    pub const INFORMATIVE_WEAK: Themed = Themed { name: "stroke.informative-weak", light: Rgb(0xcb, 0xdf, 0xfa), dark: Rgb(0x1a, 0x42, 0x75), light_256: 153, dark_256: 24 };
    pub const NEUTRAL_CONTRAST: Themed = Themed { name: "stroke.neutral-contrast", light: Rgb(0x1a, 0x1c, 0x20), dark: Rgb(0xf3, 0xf4, 0xf5), light_256: 234, dark_256: 255 };
    pub const NEUTRAL_MUTED: Themed = Themed { name: "stroke.neutral-muted", light: Rgb(0xef, 0xef, 0xef), dark: Rgb(0x2b, 0x2c, 0x30), light_256: 255, dark_256: 236 };
    pub const NEUTRAL_SOLID: Themed = Themed { name: "stroke.neutral-solid", light: Rgb(0x55, 0x5d, 0x6d), dark: Rgb(0xdc, 0xde, 0xe3), light_256: 59, dark_256: 253 };
    pub const NEUTRAL_SUBTLE: Themed = Themed { name: "stroke.neutral-subtle", light: Rgb(0xf3, 0xf3, 0xf3), dark: Rgb(0x22, 0x23, 0x27), light_256: 255, dark_256: 235 };
    pub const NEUTRAL_WEAK: Themed = Themed { name: "stroke.neutral-weak", light: Rgb(0xdc, 0xde, 0xe3), dark: Rgb(0x39, 0x3d, 0x46), light_256: 253, dark_256: 237 };
    pub const POSITIVE_SOLID: Themed = Themed { name: "stroke.positive-solid", light: Rgb(0x07, 0x91, 0x71), dark: Rgb(0x22, 0xb2, 0x7f), light_256: 29, dark_256: 36 };
    pub const POSITIVE_WEAK: Themed = Themed { name: "stroke.positive-weak", light: Rgb(0xb9, 0xe9, 0xd2), dark: Rgb(0x20, 0x49, 0x3b), light_256: 151, dark_256: 23 };
    pub const WARNING_SOLID: Themed = Themed { name: "stroke.warning-solid", light: Rgb(0x9b, 0x78, 0x21), dark: Rgb(0xca, 0x90, 0x1c), light_256: 94, dark_256: 136 };
    pub const WARNING_WEAK: Themed = Themed { name: "stroke.warning-weak", light: Rgb(0xfb, 0xdc, 0x65), dark: Rgb(0x54, 0x3e, 0x15), light_256: 221, dark_256: 58 };
}

pub mod brand {
    use super::*;

    pub const FG: Themed = Themed { name: "fg.brand", light: Rgb(0x9f, 0x84, 0xfb), dark: Rgb(0xa7, 0x8d, 0xf0), light_256: 141, dark_256: 140 };
    pub const FG_CONTRAST: Themed = Themed { name: "fg.brand-contrast", light: Rgb(0x89, 0x69, 0xea), dark: Rgb(0xa7, 0x8d, 0xf0), light_256: 98, dark_256: 140 };
    pub const FG_ON_SOLID: Themed = Themed { name: "fg.on-brand-solid", light: Rgb(0xff, 0xff, 0xff), dark: Rgb(0xff, 0xff, 0xff), light_256: 231, dark_256: 231 };
    pub const BG_SOLID: Themed = Themed { name: "bg.brand-solid", light: Rgb(0x9f, 0x84, 0xfb), dark: Rgb(0xa7, 0x8d, 0xf0), light_256: 141, dark_256: 140 };
    pub const BG_SOLID_PRESSED: Themed = Themed { name: "bg.brand-solid-pressed", light: Rgb(0x89, 0x69, 0xea), dark: Rgb(0xbe, 0xad, 0xf2), light_256: 98, dark_256: 147 };
    pub const BG_WEAK: Themed = Themed { name: "bg.brand-weak", light: Rgb(0xf5, 0xf3, 0xfe), dark: Rgb(0x28, 0x21, 0x3b), light_256: 255, dark_256: 235 };
    pub const BG_WEAK_PRESSED: Themed = Themed { name: "bg.brand-weak-pressed", light: Rgb(0xef, 0xea, 0xfe), dark: Rgb(0x3b, 0x28, 0x73), light_256: 255, dark_256: 17 };
    pub const STROKE_SOLID: Themed = Themed { name: "stroke.brand-solid", light: Rgb(0x89, 0x69, 0xea), dark: Rgb(0xa7, 0x8d, 0xf0), light_256: 98, dark_256: 140 };
    pub const STROKE_WEAK: Themed = Themed { name: "stroke.brand-weak", light: Rgb(0xe1, 0xd8, 0xff), dark: Rgb(0x44, 0x30, 0x81), light_256: 189, dark_256: 61 };
}

pub const ALL: &[Themed] = &[
    fg::CRITICAL,
    fg::CRITICAL_CONTRAST,
    fg::DISABLED,
    fg::INFORMATIVE,
    fg::INFORMATIVE_CONTRAST,
    fg::NEUTRAL,
    fg::NEUTRAL_INVERTED,
    fg::NEUTRAL_MUTED,
    fg::NEUTRAL_SUBTLE,
    fg::ON_CRITICAL_SOLID,
    fg::ON_INFORMATIVE_SOLID,
    fg::ON_NEUTRAL_SOLID,
    fg::ON_POSITIVE_SOLID,
    fg::ON_WARNING_SOLID,
    fg::PLACEHOLDER,
    fg::POSITIVE,
    fg::POSITIVE_CONTRAST,
    fg::WARNING,
    fg::WARNING_CONTRAST,
    bg::CRITICAL_SOLID,
    bg::CRITICAL_SOLID_PRESSED,
    bg::CRITICAL_WEAK,
    bg::CRITICAL_WEAK_PRESSED,
    bg::DISABLED,
    bg::INFORMATIVE_SOLID,
    bg::INFORMATIVE_SOLID_PRESSED,
    bg::INFORMATIVE_WEAK,
    bg::INFORMATIVE_WEAK_PRESSED,
    bg::LAYER_BASEMENT,
    bg::LAYER_DEFAULT,
    bg::LAYER_DEFAULT_PRESSED,
    bg::LAYER_FLOATING,
    bg::LAYER_FLOATING_PRESSED,
    bg::MAGIC_WEAK,
    bg::NEUTRAL_INVERTED,
    bg::NEUTRAL_INVERTED_PRESSED,
    bg::NEUTRAL_MUTED,
    bg::NEUTRAL_SOLID,
    bg::NEUTRAL_SOLID_PRESSED,
    bg::NEUTRAL_SOLID_MUTED,
    bg::NEUTRAL_SOLID_MUTED_PRESSED,
    bg::NEUTRAL_SUBTLE,
    bg::NEUTRAL_WEAK,
    bg::NEUTRAL_WEAK_ALPHA,
    bg::NEUTRAL_WEAK_ALPHA_PRESSED,
    bg::NEUTRAL_WEAK_PRESSED,
    bg::OVERLAY,
    bg::OVERLAY_MUTED,
    bg::POSITIVE_SOLID,
    bg::POSITIVE_SOLID_PRESSED,
    bg::POSITIVE_WEAK,
    bg::POSITIVE_WEAK_PRESSED,
    bg::TRANSPARENT,
    bg::TRANSPARENT_PRESSED,
    bg::TRANSPARENT_SELECTED,
    bg::TRANSPARENT_SELECTED_PRESSED,
    bg::WARNING_SOLID,
    bg::WARNING_SOLID_PRESSED,
    bg::WARNING_WEAK,
    bg::WARNING_WEAK_PRESSED,
    stroke::CRITICAL_SOLID,
    stroke::CRITICAL_WEAK,
    stroke::FOCUS_RING,
    stroke::INFORMATIVE_SOLID,
    stroke::INFORMATIVE_WEAK,
    stroke::NEUTRAL_CONTRAST,
    stroke::NEUTRAL_MUTED,
    stroke::NEUTRAL_SOLID,
    stroke::NEUTRAL_SUBTLE,
    stroke::NEUTRAL_WEAK,
    stroke::POSITIVE_SOLID,
    stroke::POSITIVE_WEAK,
    stroke::WARNING_SOLID,
    stroke::WARNING_WEAK,
    brand::FG,
    brand::FG_CONTRAST,
    brand::FG_ON_SOLID,
    brand::BG_SOLID,
    brand::BG_SOLID_PRESSED,
    brand::BG_WEAK,
    brand::BG_WEAK_PRESSED,
    brand::STROKE_SOLID,
    brand::STROKE_WEAK,
];

pub mod duration {
    use std::time::Duration;

    pub const D1: Duration = Duration::from_millis(50);
    pub const D2: Duration = Duration::from_millis(100);
    pub const D3: Duration = Duration::from_millis(150);
    pub const D4: Duration = Duration::from_millis(200);
    pub const D5: Duration = Duration::from_millis(250);
    pub const D6: Duration = Duration::from_millis(300);
    pub const COLOR_TRANSITION: Duration = Duration::from_millis(150);
    pub const PRESSED_SCALE: Duration = Duration::from_millis(150);
}

pub mod timing_function {
    pub const LINEAR: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
    pub const EASING: [f32; 4] = [0.35, 0.0, 0.35, 1.0];
    pub const ENTER: [f32; 4] = [0.0, 0.0, 0.15, 1.0];
    pub const EXIT: [f32; 4] = [0.35, 0.0, 1.0, 1.0];
    pub const ENTER_EXPRESSIVE: [f32; 4] = [0.03, 0.4, 0.1, 1.0];
    pub const EXIT_EXPRESSIVE: [f32; 4] = [0.35, 0.0, 0.95, 0.55];
    pub const PRESSED_SCALE: [f32; 4] = [0.0, 0.0, 0.15, 1.0];
}

pub mod dimension {
    pub const X0_5: u16 = 2;
    pub const X1: u16 = 4;
    pub const X1_5: u16 = 6;
    pub const X2: u16 = 8;
    pub const X2_5: u16 = 10;
    pub const X3: u16 = 12;
    pub const X3_5: u16 = 14;
    pub const X4: u16 = 16;
    pub const X4_5: u16 = 18;
    pub const X5: u16 = 20;
    pub const X6: u16 = 24;
    pub const X7: u16 = 28;
    pub const X8: u16 = 32;
    pub const X9: u16 = 36;
    pub const X10: u16 = 40;
    pub const X12: u16 = 48;
    pub const X13: u16 = 52;
    pub const X14: u16 = 56;
    pub const X16: u16 = 64;
    pub const SPACING_X_BETWEEN_CHIPS: u16 = 8;
    pub const SPACING_X_GLOBAL_GUTTER: u16 = 16;
    pub const SPACING_Y_COMPONENT_DEFAULT: u16 = 12;
    pub const SPACING_Y_NAV_TO_TITLE: u16 = 20;
    pub const SPACING_Y_SCREEN_BOTTOM: u16 = 56;
    pub const SPACING_Y_BETWEEN_TEXT: u16 = 6;
}

pub mod radius {
    pub const R0_5: u16 = 2;
    pub const R1: u16 = 4;
    pub const R1_5: u16 = 6;
    pub const R2: u16 = 8;
    pub const R2_5: u16 = 10;
    pub const R3: u16 = 12;
    pub const R3_5: u16 = 14;
    pub const R4: u16 = 16;
    pub const R5: u16 = 20;
    pub const R6: u16 = 24;
    pub const FULL: u16 = u16::MAX;
}

pub mod font_weight {
    pub const REGULAR: u16 = 400;
    pub const MEDIUM: u16 = 500;
    pub const BOLD: u16 = 700;
}
