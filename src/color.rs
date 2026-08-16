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
}
