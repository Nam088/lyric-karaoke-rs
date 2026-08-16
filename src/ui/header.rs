//! Title, playback state, clock and the detected note.

use iocraft::prelude::*;

use crate::analysis::pitch::Note;
use crate::color;
use crate::config;
use crate::ui::footer::long_time;
use crate::ui::spectrum::Style;

/// A slow brightness sweep across the app name, replacing `ink-motion`'s
/// `Shimmer`. It is a function of the playback clock, so it stays in step
/// after a seek and stops dead when paused.
fn shimmer(now: i64) -> Color {
    let phase = (now as f64 / 900.0).sin() as f32 * 0.5 + 0.5;
    color::mix(config::HEADER_KARAOKE, config::KEYBINDS_HIGHLIGHT, phase)
}

/// The LIVE dot blinks; PAUSED holds steady so the difference is obvious at a
/// glance.
fn status_label(is_playing: bool, now: i64) -> AnyElement<'static> {
    if !is_playing {
        return element! {
            Text(color: config::PAUSED_INDICATOR, content: config::PAUSED_LABEL)
        }
        .into();
    }

    let on = ((now as f64 / config::LIVE_BLINK_MS) as i64) % 2 == 0;
    let c = if on {
        config::LIVE_INDICATOR
    } else {
        color::fade(config::LIVE_INDICATOR, 0.55, config::DARK_BASE)
    };

    element! { Text(color: c, weight: Weight::Bold, content: config::LIVE_LABEL) }.into()
}

/// Note name plus how far off pitch it is. Useful rather than decorative: it
/// tells the singer what they are aiming at.
///
/// Always the same width. Pitch detection drops in and out constantly on real
/// music, and this label sits in a `SpaceBetween` row, so a shorter version for
/// silence would drag the clock and the playing indicator sideways several
/// times a second.
fn note_label(note: Option<Note>, show: bool) -> Vec<AnyElement<'static>> {
    const WIDTH: usize = 10;

    if !show {
        return Vec::new();
    }

    let (text, color) = match note {
        Some(n) => {
            let text = format!(" {:<3} {:+3}¢ ", n.name(), n.cents.round() as i32);
            // Green when close to centre, amber as it drifts.
            let off = (n.cents.abs() / 50.0).clamp(0.0, 1.0);
            (text, color::mix(config::NOTE_LABEL, config::PAUSED_INDICATOR, off))
        }
        None => ("    ---   ".to_string(), config::TIMELINE_REMAINING),
    };

    debug_assert_eq!(text.chars().count(), WIDTH, "note label changed width");

    // Pinned width. The label ends in spaces, the renderer drops trailing
    // whitespace, and this sits in a SpaceBetween row, so without the wrapper
    // the clock beside it would shuffle every time a pitch came and went.
    vec![element! {
        View(width: WIDTH as u32, justify_content: JustifyContent::Center) {
            Text(color: color, content: text)
        }
    }
    .into()]
}

pub fn render(
    is_playing: bool,
    position_ms: i64,
    note: Option<Note>,
    show_note: bool,
    show_keybinds: bool,
    style: Style,
) -> AnyElement<'static> {
    let keybinds: Vec<AnyElement<'static>> = if show_keybinds {
        vec![element! {
            View(margin_left: 2) {
                Text(
                    color: config::KEYBINDS_DIM,
                    content: format!(
                        "[Space] Play  [←][→] ±5s  [S] spectrum: {}  [N] note  [Q] Quit",
                        style.name()
                    ),
                )
            }
        }
        .into()]
    } else {
        Vec::new()
    };

    element! {
        View(justify_content: JustifyContent::SpaceBetween, width: 100pct) {
            Text(
                color: shimmer(position_ms),
                weight: Weight::Bold,
                content: config::APP_NAME,
            )
            #(keybinds)
            View(flex_direction: FlexDirection::Row) {
                #(note_label(note, show_note))
                // The divider belongs to the note, not to what follows it.
                #(show_note.then(|| -> AnyElement<'static> {
                    element! {
                        Text(color: config::TIMELINE_REMAINING, content: config::VBAR)
                    }
                    .into()
                }))
                #(status_label(is_playing, position_ms))
                Text(color: config::TIMELINE_REMAINING, content: config::VBAR)
                Text(color: config::TIMELINE_ELAPSED, content: long_time(position_ms))
            }
        }
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::pitch;

    #[test]
    fn the_shimmer_stays_inside_the_two_palette_colours() {
        for t in [0, 450, 900, 1_800, 62_000] {
            match shimmer(t) {
                Color::Rgb { r, g, b } => {
                    assert!((0x4A..=0x86).contains(&r), "r={r} at {t}");
                    assert!((0xDE..=0xEF).contains(&g), "g={g} at {t}");
                    assert!((0x80..=0xAC).contains(&b), "b={b} at {t}");
                }
                other => panic!("expected an rgb colour, got {other:?}"),
            }
        }
    }

    /// Where each glyph of interest sits, in columns.
    fn column_of(rendered: &str, needle: char) -> Option<usize> {
        rendered.lines().find_map(|l| l.chars().position(|c| c == needle))
    }

    fn draw(note: Option<Note>) -> String {
        let mut e = render(true, 62_000, note, true, false, Style::default());
        let mut buf = Vec::new();
        e.render(Some(70)).write(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Pitch detection drops in and out constantly on real music. If the label
    /// changed width with it, the clock next to it would shuffle sideways
    /// several times a second.
    #[test]
    fn the_clock_does_not_move_when_a_pitch_comes_and_goes() {
        let with = draw(Some(pitch::from_hz(440.0)));
        let without = draw(None);

        assert_eq!(
            column_of(&with, '\u{25cf}'),
            column_of(&without, '\u{25cf}'),
            "the LIVE dot moved:\n{with}\n{without}"
        );
    }

    /// With the note hidden the header must not leave a gap where it was.
    #[test]
    fn hiding_the_note_leaves_nothing_behind() {
        let mut e = render(true, 62_000, Some(pitch::from_hz(440.0)), false, false, Style::default());
        let mut buf = Vec::new();
        e.render(Some(70)).write(&mut buf).unwrap();
        let text = String::from_utf8_lossy(&buf).into_owned();

        assert!(!text.contains('\u{00a2}'), "the cents sign survived: {text:?}");
        assert!(!text.contains("---"), "the placeholder survived: {text:?}");
        assert!(text.contains("LIVE"), "the rest of the header went missing");

        // Exactly one divider left, the one between LIVE and the clock.
        assert_eq!(
            text.matches('\u{2502}').count(),
            1,
            "a divider was left hanging with nothing beside it: {text:?}"
        );
    }

    #[test]
    fn a_detected_note_renders_and_silence_falls_back() {
        // Only checking these do not panic and that both paths are reachable.
        assert_eq!(note_label(None, true).len(), 1);
        assert_eq!(note_label(Some(pitch::from_hz(440.0)), true).len(), 1);
        assert!(note_label(Some(pitch::from_hz(440.0)), false).is_empty());
        // Both paths must be the same width or the row twitches.
        let _ = render(true, 0, None, true, true, Style::default());
    }
}
