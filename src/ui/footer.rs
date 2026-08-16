//! Ticker, waveform timeline and transport row.

use std::sync::Arc;

use iocraft::prelude::*;

use super::Session;
use crate::analysis::envelope::Envelope;
use crate::braille::Canvas;
use crate::{config, lyrics};

/// `mm:ss`.
pub fn short_time(ms: i64) -> String {
    let ms = ms.max(0);
    format!("{:02}:{:02}", ms / 60_000, (ms % 60_000) / 1000)
}

/// `mm:ss.mmm`, for the header clock.
pub fn long_time(ms: i64) -> String {
    let ms = ms.max(0);
    format!("{:02}:{:02}.{:03}", ms / 60_000, (ms % 60_000) / 1000, ms % 1000)
}

/// Song title scrolling sideways, news ticker style.
pub fn ticker(title: &str, now: i64, width: usize) -> AnyElement<'static> {
    let width = width.max(10);

    // Short titles need more breathing room between repeats or the loop is
    // hard to read.
    let pad = if title.chars().count() < 15 { 12 } else { 6 };
    let unit: Vec<char> = format!("{}{}•{}", title.to_uppercase(), " ".repeat(pad), " ".repeat(pad))
        .chars()
        .collect();

    let offset = ((now as f64 / config::TICKER_SCROLL_MS) as usize) % unit.len();
    let visible: String = unit
        .iter()
        .cycle()
        .skip(offset)
        .take(width)
        .collect();

    element! {
        View(width: width as u32, justify_content: JustifyContent::Center) {
            Text(color: config::TICKER_TEXT, weight: Weight::Bold, content: visible)
        }
    }
    .into()
}

/// The song's loudness over time, with the played part lit.
///
/// This replaces a plain `━━━━●────` bar. The shape alone tells you where the
/// intro ends and where the chorus is, which a uniform bar cannot.
///
/// Always two rows tall, including before the background scan finishes, when
/// it draws as a flat line. A block that changed height on arrival would
/// reflow the whole panel a few seconds into the song.
pub const WAVEFORM_ROWS: usize = 2;

fn waveform(points: &[f32], progress: f32, cells_w: usize) -> AnyElement<'static> {
    const ROWS: usize = WAVEFORM_ROWS;
    let mut canvas = Canvas::new(cells_w, ROWS);
    let (w, h) = (canvas.width(), canvas.height());
    let mid = h / 2;

    for (x, &point) in points.iter().enumerate().take(w) {
        // Mirrored around the middle so it reads as a waveform rather than a
        // bar chart.
        let n = ((point * mid as f32).round() as usize).clamp(1, mid);
        for y in 0..n {
            canvas.set(x, mid - 1 - y);
            canvas.set(x, mid + y);
        }
    }

    // Split at the playhead. Cells are two dots wide, so this rounds to the
    // nearest cell.
    let played_cells = ((progress.clamp(0.0, 1.0) * cells_w as f32).round() as usize).min(cells_w);

    let rows: Vec<AnyElement<'static>> = canvas
        .rows()
        .into_iter()
        .map(|row| {
            let chars: Vec<char> = row.chars().collect();
            let played: String = chars[..played_cells].iter().collect();
            let ahead: String = chars[played_cells..].iter().collect();
            element! {
                View(flex_direction: FlexDirection::Row) {
                    Text(color: config::WAVEFORM_PLAYED, content: played)
                    Text(color: config::WAVEFORM_AHEAD, content: ahead)
                }
            }
            .into()
        })
        .collect();

    element! {
        View(flex_direction: FlexDirection::Column, align_items: AlignItems::Center) {
            #(rows)
        }
    }
    .into()
}

/// Terminal rows the timeline occupies. The layout needs this before it can
/// decide what else fits.
pub fn timeline_rows() -> usize {
    if config::WAVEFORM_TIMELINE {
        WAVEFORM_ROWS
    } else {
        1
    }
}

/// Split a bar of `width` columns into the run behind the playhead and the run
/// ahead of it. The marker takes one column, so the three always add up.
fn bar_parts(progress: f32, width: usize) -> (usize, usize) {
    let usable = width.saturating_sub(1);
    let filled = ((progress.clamp(0.0, 1.0) * usable as f32).round() as usize).min(usable);
    (filled, usable - filled)
}

/// The plain bar: elapsed, a marker at the playhead, remaining.
/// Every column is its own clickable `Button` so seeking is precise.
fn progress_bar(
    progress: f32,
    width: usize,
    session: Option<Arc<Session>>,
    total_ms: i64,
) -> AnyElement<'static> {
    let (filled, empty) = bar_parts(progress, width);
    let usable = filled + empty; // width - 1 (marker takes one column)

    let marker_color = config::KEYBINDS_HIGHLIGHT;

    let mut spans: Vec<AnyElement<'static>> = Vec::with_capacity(width);

    for col in 0..width {
        let (ch, col_color, bold) = if col < filled {
            (config::TIMELINE_FILLED, config::TIMELINE_ELAPSED, false)
        } else if col == filled {
            (config::TIMELINE_MARKER, marker_color, true)
        } else {
            (config::TIMELINE_EMPTY, config::TIMELINE_REMAINING, false)
        };

        let text = element! {
            Text(
                color: col_color,
                weight: if bold { Weight::Bold } else { Weight::Normal },
                content: ch.to_string(),
            )
        };

        if let Some(ref s) = session {
            let s = s.clone();
            let target = if usable > 0 {
                (col.min(usable) as f64 / usable as f64 * total_ms as f64) as i64
            } else {
                0
            };
            spans.push(
                element! {
                    Button(handler: move |_| s.audio.seek_ms(target)) { #(text) }
                }
                .into(),
            );
        } else {
            spans.push(text.into());
        }
    }

    element! {
        View(flex_direction: FlexDirection::Row) { #(spans) }
    }
    .into()
}

pub fn timeline(
    envelope: &Envelope,
    position_ms: i64,
    total_ms: i64,
    width: usize,
    session: Option<Arc<Session>>,
) -> AnyElement<'static> {
    let width = width.max(10);
    let progress = if total_ms > 0 {
        position_ms as f32 / total_ms as f32
    } else {
        0.0
    };

    let body = if config::WAVEFORM_TIMELINE {
        // The waveform is drawn on a braille canvas, so it wants twice as many
        // sample points as it has columns. Zeros until the background scan
        // lands, which the draw renders as a flat line.
        let points = envelope
            .resampled(width * 2)
            .unwrap_or_else(|| vec![0.0; width * 2]);
        waveform(&points, progress, width)
    } else {
        progress_bar(progress, width, session, total_ms)
    };

    element! {
        View(flex_direction: FlexDirection::Row, align_items: AlignItems::Center) {
            Text(color: config::TIMELINE_ELAPSED, content: short_time(position_ms))
            Text(color: config::TIMELINE_REMAINING, content: config::TIMELINE_CAP_LEFT)
            #(body)
            Text(color: config::TIMELINE_REMAINING, content: config::TIMELINE_CAP_RIGHT)
            Text(color: config::TIMELINE_ELAPSED, content: short_time(total_ms))
        }
    }
    .into()
}

/// Transport controls. Clickable in a terminal that reports mouse events, and
/// mirrored by the keyboard either way.
///
/// The skip buttons move by lyric line rather than by track: there is only one
/// song, and stepping through its lines is what a karaoke player is for.
///
/// Every button is the same fixed width, so the row does not twitch when the
/// play glyph changes.
pub fn transport(session: Arc<Session>, now: i64, is_playing: bool) -> AnyElement<'static> {
    let (glyph, color) = if is_playing {
        ("▌▌", config::LIVE_INDICATOR)
    } else {
        ("►", config::PAUSED_INDICATOR)
    };

    let back = session.clone();
    let toggle = session.clone();
    let forward = session;

    element! {
        View(justify_content: JustifyContent::Center, width: 100pct, margin_top: 1) {
            Button(handler: move |_| {
                let target = lyrics::previous_line_start(&back.sentences, now);
                back.audio.seek_ms(target);
            }) {
                View(width: 8, justify_content: JustifyContent::Center) {
                    Text(
                        color: config::TIMELINE_REMAINING,
                        weight: Weight::Bold,
                        content: "|\u{25c4}",
                    )
                }
            }

            Button(handler: move |_| toggle.audio.toggle()) {
                View(width: 8, justify_content: JustifyContent::Center) {
                    Text(color: color, weight: Weight::Bold, content: glyph)
                }
            }

            Button(handler: move |_| {
                let target = lyrics::next_line_start(&forward.sentences, now);
                forward.audio.seek_ms(target);
            }) {
                View(width: 8, justify_content: JustifyContent::Center) {
                    Text(
                        color: config::TIMELINE_REMAINING,
                        weight: Weight::Bold,
                        content: "\u{25ba}|",
                    )
                }
            }
        }
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_times_and_clamps_negatives() {
        assert_eq!(short_time(62_000), "01:02");
        assert_eq!(short_time(0), "00:00");
        assert_eq!(short_time(-500), "00:00");
        assert_eq!(long_time(62_345), "01:02.345");
        assert_eq!(short_time(3_599_000), "59:59");
    }

    #[test]
    fn the_ticker_scrolls_and_wraps() {
        let first = ticker_text("ABC", 0, 8);
        let later = ticker_text("ABC", 700, 8);
        assert_ne!(first, later);
        assert_eq!(first.chars().count(), 8);

        // A full trip round the pattern returns to the start.
        let unit_len = "ABC".len() + 12 + 1 + 12;
        let wrapped = ticker_text("ABC", (unit_len as f64 * config::TICKER_SCROLL_MS) as i64, 8);
        assert_eq!(first, wrapped);
    }

    /// Play and pause draw different glyphs. If they measured differently the
    /// row would twitch every time playback was toggled, and the click targets
    /// would move out from under the pointer.
    ///
    /// Mirrors the real markup, since building a `Session` needs an audio
    /// device.
    #[test]
    fn the_transport_row_does_not_twitch_when_toggled() {
        let draw = |playing: bool| {
            let mut e = element! {
                View(justify_content: JustifyContent::Center, width: 40u32) {
                    View(width: 8, justify_content: JustifyContent::Center) {
                        Text(weight: Weight::Bold, content: "|◄")
                    }
                    View(width: 8, justify_content: JustifyContent::Center) {
                        Text(
                            weight: Weight::Bold,
                            content: if playing { "▌▌" } else { "►" },
                        )
                    }
                    View(width: 8, justify_content: JustifyContent::Center) {
                        Text(weight: Weight::Bold, content: "►|")
                    }
                }
            };
            let mut buf = Vec::new();
            e.render(Some(60)).write(&mut buf).unwrap();
            let text = String::from_utf8_lossy(&buf).into_owned();
            text.lines()
                .find(|l| l.contains('◄'))
                .map(|l| l.chars().position(|c| c == '◄').unwrap())
        };

        assert_eq!(draw(true), draw(false), "the transport row moved");
    }

    #[test]
    fn the_bar_always_spans_exactly_its_width() {
        for width in [10usize, 11, 46, 47, 80] {
            for step in 0..=20 {
                let p = step as f32 / 20.0;
                let (filled, empty) = bar_parts(p, width);
                assert_eq!(
                    filled + 1 + empty,
                    width,
                    "width {width} at progress {p}: {filled} + marker + {empty}"
                );
            }
        }
    }

    #[test]
    fn the_marker_starts_at_the_left_and_ends_at_the_right() {
        let width = 40;
        assert_eq!(bar_parts(0.0, width).0, 0, "marker should start at column 0");
        assert_eq!(
            bar_parts(1.0, width).1,
            0,
            "marker should finish at the last column"
        );
    }

    #[test]
    fn the_bar_never_runs_backwards() {
        let width = 46;
        let mut last = 0;
        for step in 0..=100 {
            let filled = bar_parts(step as f32 / 100.0, width).0;
            assert!(filled >= last, "went backwards at {step}%");
            last = filled;
        }
    }

    #[test]
    fn out_of_range_progress_is_clamped() {
        let width = 30;
        assert_eq!(bar_parts(-1.0, width), bar_parts(0.0, width));
        assert_eq!(bar_parts(5.0, width), bar_parts(1.0, width));
    }

    /// Same maths as `ticker`, without building elements.
    fn ticker_text(title: &str, now: i64, width: usize) -> String {
        let pad = if title.chars().count() < 15 { 12 } else { 6 };
        let unit: Vec<char> =
            format!("{}{}•{}", title.to_uppercase(), " ".repeat(pad), " ".repeat(pad))
                .chars()
                .collect();
        let offset = ((now as f64 / config::TICKER_SCROLL_MS) as usize) % unit.len();
        unit.iter().cycle().skip(offset).take(width).collect()
    }
}

#[cfg(test)]
mod click_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use crossterm::event::MouseButton;
    use futures::stream::{self, StreamExt};
    use iocraft::prelude::*;

    /// Which of the three transport buttons was pressed, if any.
    ///
    /// Global, because the handler closures have to be `'static`, so the tests
    /// take a lock rather than run over each other.
    static PRESSED: AtomicUsize = AtomicUsize::new(0);
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    /// The real transport, with the audio calls swapped for a counter. The
    /// widths and nesting match `transport` exactly, since where the click
    /// lands is the whole point.
    #[component]
    fn Harness(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut done = hooks.use_state(|| false);

        // Every mouse event ends the run, hit or miss. Without this a click
        // that lands outside the buttons leaves the render loop waiting for an
        // exit that never comes.
        hooks.use_terminal_events(move |event| {
            if matches!(event, TerminalEvent::FullscreenMouse(_)) {
                done.set(true);
            }
        });

        let mut system = hooks.use_context_mut::<SystemContext>();
        if done.get() {
            system.exit();
        }

        let mark = |which: usize| move |_| PRESSED.store(which, Ordering::SeqCst);

        // Three columns of margin either side, so the edges of the hit regions
        // are somewhere real to aim at.
        element! {
            View(width: 30u32, padding_left: 3, padding_right: 3) {
                Button(handler: mark(1)) {
                    View(width: 8, justify_content: JustifyContent::Center) {
                        Text(content: "|◄")
                    }
                }
                Button(handler: mark(2)) {
                    View(width: 8, justify_content: JustifyContent::Center) {
                        Text(content: "▌▌")
                    }
                }
                Button(handler: mark(3)) {
                    View(width: 8, justify_content: JustifyContent::Center) {
                        Text(content: "►|")
                    }
                }
            }
        }
    }

    fn click(column: u16) -> usize {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        PRESSED.store(0, Ordering::SeqCst);
        smol::block_on(async move {
            element!(Harness)
                .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::once(
                    async move {
                        TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
                            MouseEventKind::Down(MouseButton::Left),
                            column,
                            0,
                        ))
                    },
                )))
                .collect::<Vec<_>>()
                .await;
        });
        PRESSED.load(Ordering::SeqCst)
    }

    /// Three margin columns, then three buttons of eight columns each, so they
    /// occupy 3 to 10, 11 to 18 and 19 to 26.
    #[test]
    fn each_transport_button_answers_its_own_column() {
        assert_eq!(click(3), 1, "skip back, first column");
        assert_eq!(click(10), 1, "skip back, last column");
        assert_eq!(click(11), 2, "play or pause, first column");
        assert_eq!(click(18), 2, "play or pause, last column");
        assert_eq!(click(19), 3, "skip forward, first column");
        assert_eq!(click(26), 3, "skip forward, last column");
    }

    #[test]
    fn a_click_in_the_margins_does_nothing() {
        assert_eq!(click(0), 0, "the left margin is not a button");
        assert_eq!(click(2), 0, "the column before the first button is not one");
        assert_eq!(click(27), 0, "the column after the last button is not one");
        assert_eq!(click(29), 0, "the right margin is not a button");
    }
}
