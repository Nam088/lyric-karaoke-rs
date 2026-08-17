//! Title, playback state, clock and the detected note.

use std::sync::Arc;

use iocraft::prelude::*;

use super::Session;
use crate::analysis::pitch::Note;
use crate::color::{self, Theme, ThemePreset};
use crate::config;
use crate::ui::footer::long_time;
use crate::ui::spectrum::Style;

/// A slow brightness sweep across the app name, replacing `ink-motion`'s
/// `Shimmer`. It is a function of the playback clock, so it stays in step
/// after a seek and stops dead when paused.
fn shimmer(now: i64, char_offset: usize, theme: &Theme) -> Color {
    let phase = (now as f64 / 900.0 + char_offset as f64 * config::SHIMMER_CHAR_OFFSET).sin()
        as f32
        * 0.5
        + 0.5;
    color::mix(theme.header, theme.highlight, phase)
}

/// The LIVE dot blinks; PAUSED holds steady so the difference is obvious at a
/// glance. Clickable to toggle playback when a session is available.
fn status_label(
    is_playing: bool,
    now: i64,
    theme: &Theme,
    session: Option<Arc<Session>>,
) -> AnyElement<'static> {
    let (content, color, bold) = if !is_playing {
        (config::PAUSED_LABEL, theme.paused, false)
    } else {
        let on = ((now as f64 / config::LIVE_BLINK_MS) as i64) % 2 == 0;
        let c = if on {
            theme.live
        } else {
            color::fade(theme.live, 0.55, theme.dark_base)
        };
        (config::LIVE_LABEL, c, true)
    };

    let weight = if bold { Weight::Bold } else { Weight::Normal };

    if let Some(s) = session {
        element! {
            Button(handler: move |_| s.audio.toggle()) {
                Text(color: color, weight: weight, content: content)
            }
        }
        .into()
    } else {
        element! { Text(color: color, weight: weight, content: content) }.into()
    }
}

/// Note name plus how far off pitch it is. Useful rather than decorative: it
/// tells the singer what they are aiming at.
fn note_label(note: Option<Note>, show: bool, theme: &Theme) -> Vec<AnyElement<'static>> {
    const WIDTH: usize = 10;

    if !show {
        return Vec::new();
    }

    let (text, color) = match note {
        Some(n) => {
            let text = format!(" {:<3} {:+3}¢ ", n.name(), n.cents.round() as i32);
            // Close to centre is note_label color, amber as it drifts.
            let off = (n.cents.abs() / 50.0).clamp(0.0, 1.0);
            (text, color::mix(theme.note_label, theme.paused, off))
        }
        None => ("    ---   ".to_string(), theme.remaining),
    };

    debug_assert_eq!(text.chars().count(), WIDTH, "note label changed width");

    vec![element! {
        View(width: WIDTH as u32, justify_content: JustifyContent::Center) {
            Text(color: color, content: text)
        }
    }
    .into()]
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    is_playing: bool,
    position_ms: i64,
    note: Option<Note>,
    show_note: bool,
    show_keybinds: bool,
    style: Style,
    theme_preset: ThemePreset,
    theme: &Theme,
    session: Option<Arc<Session>>,
) -> AnyElement<'static> {
    let keybinds: Vec<AnyElement<'static>> = if show_keybinds {
        vec![element! {
            View(margin_left: 2) {
                Text(
                    color: theme.keybinds_dim,
                    content: format!(
                        "[Space] Play  [←][→] ±5s  [[ ][]] Track  [S] spectrum: {}  [C] theme: {}  [N] note  [Q] Quit",
                        style.name(),
                        theme_preset.name(),
                    ),
                )
            }
        }
        .into()]
    } else {
        Vec::new()
    };

    let track_badge = session.as_ref().and_then(|s| {
        let p = s.playlist.read().ok()?;
        if p.len() > 1 {
            let idx = *s.track_index.read().ok()?;
            Some(format!(" [{}/{}]", idx + 1, p.len()))
        } else {
            None
        }
    });

    let shimmer_chars: Vec<AnyElement<'static>> = config::APP_NAME
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            element! {
                Text(
                    color: shimmer(position_ms, i, theme),
                    weight: Weight::Bold,
                    content: ch.to_string(),
                )
            }
            .into()
        })
        .collect();

    element! {
        View(justify_content: JustifyContent::SpaceBetween, width: 100pct) {
            View(flex_direction: FlexDirection::Row) {
                #(shimmer_chars)
                #(track_badge.map(|b| element! {
                    Text(color: theme.elapsed, weight: Weight::Bold, content: b)
                }))
            }
            #(keybinds)
            View(flex_direction: FlexDirection::Row) {
                #(note_label(note, show_note, theme))
                // The divider belongs to the note, not to what follows it.
                #(show_note.then(|| -> AnyElement<'static> {
                    element! {
                        Text(color: theme.remaining, content: config::VBAR)
                    }
                    .into()
                }))
                #(status_label(is_playing, position_ms, theme, session))
                Text(color: theme.remaining, content: config::VBAR)
                Text(color: theme.elapsed, content: long_time(position_ms))
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
        let theme = Theme::default();
        for t in [0, 450, 900, 1_800, 62_000] {
            for offset in [0, 3, 6] {
                match shimmer(t, offset, &theme) {
                    Color::Rgb { r, g, b } => {
                        assert!((0x4A..=0x86).contains(&r), "r={r} at t={t}, offset={offset}");
                        assert!((0xDE..=0xEF).contains(&g), "g={g} at t={t}, offset={offset}");
                        assert!((0x80..=0xAC).contains(&b), "b={b} at t={t}, offset={offset}");
                    }
                    other => panic!("expected an rgb colour, got {other:?}"),
                }
            }
        }
    }

    /// Where each glyph of interest sits, in columns.
    fn column_of(rendered: &str, needle: char) -> Option<usize> {
        rendered.lines().find_map(|l| l.chars().position(|c| c == needle))
    }

    fn draw(note: Option<Note>) -> String {
        let theme = Theme::default();
        let mut e = render(true, 62_000, note, true, false, Style::default(), ThemePreset::default(), &theme, None);
        let mut buf = Vec::new();
        e.render(Some(70)).write(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    }

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

    #[test]
    fn hiding_the_note_leaves_nothing_behind() {
        let theme = Theme::default();
        let mut e = render(true, 62_000, Some(pitch::from_hz(440.0)), false, false, Style::default(), ThemePreset::default(), &theme, None);
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
        let theme = Theme::default();
        assert_eq!(note_label(None, true, &theme).len(), 1);
        assert_eq!(note_label(Some(pitch::from_hz(440.0)), true, &theme).len(), 1);
        assert!(note_label(Some(pitch::from_hz(440.0)), false, &theme).is_empty());
        let _ = render(true, 0, None, true, true, Style::default(), ThemePreset::default(), &theme, None);
    }
}
