//! Colour blending.
//!
//! `LyricLine.tsx` carried its own `lerp`, `hexToRgb`, `rgbToHex` and
//! `fadeColor`. They are shared here because the header shimmer, the beat
//! pulse and the lyric fade all need the same operation.

use iocraft::prelude::Color;

fn parts(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb { r, g, b } => (r, g, b),
        // The palette is entirely Rgb, so this is only a safety net.
        _ => (0xFF, 0xFF, 0xFF),
    }
}

/// Blend from `a` to `b`. `t` is clamped, so callers can pass raw ratios.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (r1, g1, b1) = parts(a);
    let (r2, g2, b2) = parts(b);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::Rgb { r: lerp(r1, r2), g: lerp(g1, g2), b: lerp(b1, b2) }
}

/// Push a colour toward a dark base. `t` of 0 leaves it alone, 1 makes it the
/// base. This is the spotlight dimming for lines away from the active one.
pub fn fade(c: Color, t: f32, base: (u8, u8, u8)) -> Color {
    mix(c, Color::Rgb { r: base.0, g: base.1, b: base.2 }, t)
}

/// How much to dim a line sitting `distance` rows from the active one.
///
/// An ease out curve: the first step away drops sharply, further steps level
/// off, so the active line reads as lit from above without the outer lines
/// vanishing entirely.
pub fn distance_fade(distance: f32) -> f32 {
    if distance <= 0.0 {
        return 0.0;
    }
    (1.0 - 1.0 / (1.0 + distance * 0.7)).min(0.88)
}

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

/// Color theme presets. Cycled with the `C` key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemePreset {
    #[default]
    Emerald,
    Cyberpunk,
    Ocean,
    Sunset,
    Sakura,
    Mono,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 6] = [
        ThemePreset::Emerald,
        ThemePreset::Cyberpunk,
        ThemePreset::Ocean,
        ThemePreset::Sunset,
        ThemePreset::Sakura,
        ThemePreset::Mono,
    ];

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&t| t == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            ThemePreset::Emerald => "emerald",
            ThemePreset::Cyberpunk => "cyberpunk",
            ThemePreset::Ocean => "ocean",
            ThemePreset::Sunset => "sunset",
            ThemePreset::Sakura => "sakura",
            ThemePreset::Mono => "mono",
        }
    }

    pub fn theme(self) -> Theme {
        Theme::from_preset(self)
    }
}

/// Complete color palette for rendering the UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub border: Color,
    pub header: Color,
    pub live: Color,
    pub paused: Color,
    pub elapsed: Color,
    pub remaining: Color,
    pub ticker: Color,
    pub keybinds_dim: Color,
    pub highlight: Color,
    pub lyric_past: Color,
    pub lyric_future: Color,
    pub lyric_singing: Color,
    pub lyric_hit: Color,
    pub lyric_hit_peak: Color,
    pub spectrum_edge: Color,
    pub spectrum_fill: Color,
    pub spectrum_peak: Color,
    pub note_label: Color,
    pub dark_base: (u8, u8, u8),
}

impl Default for Theme {
    fn default() -> Self {
        ThemePreset::Emerald.theme()
    }
}

impl Theme {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Emerald => Self {
                name: "emerald",
                border: rgb(0x22, 0xC5, 0x5E),
                header: rgb(0x4A, 0xDE, 0x80),
                live: rgb(0x22, 0xC5, 0x5E),
                paused: rgb(0xEA, 0xB3, 0x08),
                elapsed: rgb(0x22, 0xC5, 0x5E),
                remaining: rgb(0x37, 0x41, 0x51),
                ticker: rgb(0x86, 0xEF, 0xAC),
                keybinds_dim: rgb(0x4B, 0x55, 0x63),
                highlight: rgb(0x86, 0xEF, 0xAC),
                lyric_past: rgb(0x16, 0xA3, 0x4A),
                lyric_future: rgb(0x4B, 0x55, 0x63),
                lyric_singing: rgb(0xF0, 0xFD, 0xF4),
                lyric_hit: rgb(0x4A, 0xDE, 0x80),
                lyric_hit_peak: rgb(0xFF, 0xFF, 0xFF),
                spectrum_edge: rgb(0xBB, 0xF7, 0xD0),
                spectrum_fill: rgb(0x15, 0x80, 0x3D),
                spectrum_peak: rgb(0xF0, 0xFD, 0xF4),
                note_label: rgb(0x86, 0xEF, 0xAC),
                dark_base: (0x03, 0x07, 0x12),
            },
            ThemePreset::Cyberpunk => Self {
                name: "cyberpunk",
                border: rgb(0xEC, 0x48, 0x99),
                header: rgb(0xF4, 0x72, 0xB6),
                live: rgb(0x06, 0xB6, 0xD4),
                paused: rgb(0xF5, 0x9E, 0x0B),
                elapsed: rgb(0xEC, 0x48, 0x99),
                remaining: rgb(0x3F, 0x3F, 0x46),
                ticker: rgb(0x67, 0xE8, 0xF9),
                keybinds_dim: rgb(0x52, 0x52, 0x5B),
                highlight: rgb(0xF4, 0x72, 0xB6),
                lyric_past: rgb(0x9D, 0x17, 0x4D),
                lyric_future: rgb(0x71, 0x71, 0x7A),
                lyric_singing: rgb(0xFD, 0xFA, 0xFC),
                lyric_hit: rgb(0x06, 0xB6, 0xD4),
                lyric_hit_peak: rgb(0xFF, 0xFF, 0xFF),
                spectrum_edge: rgb(0x67, 0xE8, 0xF9),
                spectrum_fill: rgb(0x83, 0x18, 0x43),
                spectrum_peak: rgb(0xFE, 0xF0, 0x8A),
                note_label: rgb(0x67, 0xE8, 0xF9),
                dark_base: (0x0F, 0x05, 0x1D),
            },
            ThemePreset::Ocean => Self {
                name: "ocean",
                border: rgb(0x0E, 0xA5, 0xE9),
                header: rgb(0x38, 0xBD, 0xF8),
                live: rgb(0x0E, 0xA5, 0xE9),
                paused: rgb(0xFB, 0xBF, 0x24),
                elapsed: rgb(0x02, 0x84, 0xC7),
                remaining: rgb(0x33, 0x41, 0x55),
                ticker: rgb(0x7D, 0xD3, 0xFC),
                keybinds_dim: rgb(0x47, 0x55, 0x69),
                highlight: rgb(0x38, 0xBD, 0xF8),
                lyric_past: rgb(0x03, 0x69, 0xA1),
                lyric_future: rgb(0x47, 0x55, 0x69),
                lyric_singing: rgb(0xF0, 0xF9, 0xFF),
                lyric_hit: rgb(0x38, 0xBD, 0xF8),
                lyric_hit_peak: rgb(0xFF, 0xFF, 0xFF),
                spectrum_edge: rgb(0xBA, 0xE6, 0xFD),
                spectrum_fill: rgb(0x07, 0x59, 0x85),
                spectrum_peak: rgb(0xF0, 0xF9, 0xFF),
                note_label: rgb(0x7D, 0xD3, 0xFC),
                dark_base: (0x02, 0x06, 0x17),
            },
            ThemePreset::Sunset => Self {
                name: "sunset",
                border: rgb(0xF9, 0x73, 0x16),
                header: rgb(0xFB, 0x92, 0x3C),
                live: rgb(0xFB, 0x92, 0x3C),
                paused: rgb(0xEF, 0x44, 0x44),
                elapsed: rgb(0xEA, 0x58, 0x0C),
                remaining: rgb(0x44, 0x40, 0x3C),
                ticker: rgb(0xFD, 0xBA, 0x74),
                keybinds_dim: rgb(0x57, 0x53, 0x4E),
                highlight: rgb(0xFE, 0xD7, 0xAA),
                lyric_past: rgb(0xC2, 0x41, 0x0C),
                lyric_future: rgb(0x78, 0x71, 0x6C),
                lyric_singing: rgb(0xFF, 0xFB, 0xEB),
                lyric_hit: rgb(0xFB, 0x92, 0x3C),
                lyric_hit_peak: rgb(0xFF, 0xFF, 0xFF),
                spectrum_edge: rgb(0xFE, 0xD7, 0xAA),
                spectrum_fill: rgb(0x9A, 0x34, 0x12),
                spectrum_peak: rgb(0xFE, 0xF0, 0x8A),
                note_label: rgb(0xFD, 0xBA, 0x74),
                dark_base: (0x1C, 0x0A, 0x00),
            },
            ThemePreset::Sakura => Self {
                name: "sakura",
                border: rgb(0xF4, 0x3F, 0x5E),
                header: rgb(0xFB, 0x71, 0x85),
                live: rgb(0xFB, 0x71, 0x85),
                paused: rgb(0xEA, 0xB3, 0x08),
                elapsed: rgb(0xE1, 0x1D, 0x48),
                remaining: rgb(0x4C, 0x05, 0x19),
                ticker: rgb(0xFD, 0xA4, 0xAF),
                keybinds_dim: rgb(0x71, 0x3F, 0x4C),
                highlight: rgb(0xFF, 0xE4, 0xE6),
                lyric_past: rgb(0xBE, 0x12, 0x3C),
                lyric_future: rgb(0x88, 0x13, 0x37),
                lyric_singing: rgb(0xFF, 0xF1, 0xF2),
                lyric_hit: rgb(0xFB, 0x71, 0x85),
                lyric_hit_peak: rgb(0xFF, 0xFF, 0xFF),
                spectrum_edge: rgb(0xFE, 0xCD, 0xD3),
                spectrum_fill: rgb(0x88, 0x13, 0x37),
                spectrum_peak: rgb(0xFF, 0xFF, 0xFF),
                note_label: rgb(0xFD, 0xA4, 0xAF),
                dark_base: (0x14, 0x02, 0x07),
            },
            ThemePreset::Mono => Self {
                name: "mono",
                border: rgb(0x94, 0xA3, 0xB8),
                header: rgb(0xE2, 0xE8, 0xF0),
                live: rgb(0xE2, 0xE8, 0xF0),
                paused: rgb(0x94, 0xA3, 0xB8),
                elapsed: rgb(0x94, 0xA3, 0xB8),
                remaining: rgb(0x33, 0x41, 0x55),
                ticker: rgb(0xCB, 0xD5, 0xE1),
                keybinds_dim: rgb(0x64, 0x74, 0x8B),
                highlight: rgb(0xFF, 0xFF, 0xFF),
                lyric_past: rgb(0x64, 0x74, 0x8B),
                lyric_future: rgb(0x47, 0x55, 0x69),
                lyric_singing: rgb(0xF8, 0xFA, 0xFC),
                lyric_hit: rgb(0xCB, 0xD5, 0xE1),
                lyric_hit_peak: rgb(0xFF, 0xFF, 0xFF),
                spectrum_edge: rgb(0xF1, 0xF5, 0xF9),
                spectrum_fill: rgb(0x47, 0x55, 0x69),
                spectrum_peak: rgb(0xFF, 0xFF, 0xFF),
                note_label: rgb(0xCB, 0xD5, 0xE1),
                dark_base: (0x02, 0x06, 0x17),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Color = Color::Rgb { r: 0, g: 0, b: 0 };
    const B: Color = Color::Rgb { r: 100, g: 200, b: 50 };

    #[test]
    fn mixing_hits_both_ends_exactly() {
        assert_eq!(mix(A, B, 0.0), A);
        assert_eq!(mix(A, B, 1.0), B);
    }

    #[test]
    fn mixing_clamps_out_of_range_ratios() {
        assert_eq!(mix(A, B, -5.0), A);
        assert_eq!(mix(A, B, 5.0), B);
    }

    #[test]
    fn the_midpoint_is_halfway() {
        assert_eq!(mix(A, B, 0.5), Color::Rgb { r: 50, g: 100, b: 25 });
    }

    #[test]
    fn distance_fade_grows_but_never_reaches_full_dark() {
        assert_eq!(distance_fade(0.0), 0.0);
        assert_eq!(distance_fade(-1.0), 0.0);
        let (d1, d2, d3) = (distance_fade(1.0), distance_fade(2.0), distance_fade(3.0));
        assert!(d1 < d2 && d2 < d3);
        assert!(distance_fade(100.0) <= 0.88);
    }

    #[test]
    fn cycling_visits_every_theme_and_comes_back() {
        let mut t = ThemePreset::Emerald;
        for &expected in &ThemePreset::ALL {
            assert_eq!(t, expected);
            t = t.next();
        }
        assert_eq!(t, ThemePreset::Emerald);
    }

    #[test]
    fn all_themes_produce_valid_palettes() {
        for &preset in &ThemePreset::ALL {
            let theme = preset.theme();
            assert_eq!(theme.name, preset.name());
            assert!(!preset.name().is_empty());
        }
    }
}
