//! The root component.
//!
//! One clock drives everything. Each frame reads `audio.position_ms()` and
//! derives the entire screen from it. The TypeScript build ran three separate
//! wall clock timers (`useKaraokePlayer`, `IndependentHeaderClock` and
//! `IndependentFooterTimeline`), none of which was the audio, so they drifted
//! apart from each other and from the song.

pub mod footer;
pub mod layout;
pub mod header;
pub mod lyric_line;
pub mod spectrum;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iocraft::prelude::*;

use crate::analysis::{envelope::Envelope, Analyzer, FFT_SIZE};
use crate::audio::Audio;
use crate::config;
use crate::lyrics::{self, Sentence};
use layout::Layout;
use lyric_line::Status;

/// Everything built once at startup and then only read.
pub struct Session {
    pub audio: Audio,
    pub sentences: Vec<Sentence>,
    pub envelope: Envelope,
    pub start_ms: i64,
}

#[derive(Default, Props)]
pub struct AppProps {
    pub session: Option<Arc<Session>>,
}

#[component]
pub fn App(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let session = props.session.clone().expect("session must be provided");

    let (term_w, term_h) = hooks.use_terminal_size();
    let mut frame = hooks.use_state(|| 0u64);
    let mut show_keybinds = hooks.use_state(|| config::SHOW_KEYBINDS);
    let mut show_note = hooks.use_state(|| config::SHOW_NOTE);
    let mut render_past = hooks.use_state(|| config::RENDER_PAST_ON_START);
    let mut should_exit = hooks.use_state(|| false);
    // The config decides where the cycle starts; the S key moves it from there.
    let mut spectrum_style = hooks.use_state(|| {
        if config::SHOW_SPECTRUM {
            spectrum::Style::default()
        } else {
            spectrum::Style::Off
        }
    });

    let analyzer: Arc<Mutex<Analyzer>> =
        hooks.use_const(|| Arc::new(Mutex::new(Analyzer::new(64))));

    // ── The one loop ──
    let s = session.clone();
    let an = analyzer.clone();
    hooks.use_future(async move {
        let mut last = Instant::now();
        loop {
            smol::Timer::after(Duration::from_millis(config::TICK_INTERVAL_MS)).await;

            let dt = last.elapsed().as_secs_f32().min(0.25);
            last = Instant::now();

            an.lock()
                .unwrap()
                .feed(&s.audio.ring().latest(FFT_SIZE), s.audio.sample_rate(), dt);

            // Bumping a counter is what asks iocraft to redraw. The frame
            // number itself is never displayed.
            frame += 1;
        }
    });

    let s = session.clone();
    hooks.use_terminal_events(move |event| {
        let TerminalEvent::Key(KeyEvent { code, kind, .. }) = event else {
            return;
        };
        if kind == KeyEventKind::Release {
            return;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => should_exit.set(true),
            KeyCode::Char('h') | KeyCode::Char('H') => {
                let v = show_keybinds.get();
                show_keybinds.set(!v);
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                spectrum_style.set(spectrum_style.get().next());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let v = show_note.get();
                show_note.set(!v);
            }
            KeyCode::Char(' ') => s.audio.toggle(),
            KeyCode::Left => {
                s.audio.seek_by_ms(-config::SEEK_STEP_MS);
                render_past.set(true);
            }
            KeyCode::Right => {
                s.audio.seek_by_ms(config::SEEK_STEP_MS);
                render_past.set(true);
            }
            _ => {}
        }
    });

    let mut system = hooks.use_context_mut::<SystemContext>();
    // Needed for the transport buttons. Terminals that do not report mouse
    // events simply carry on with the keyboard.
    system.set_mouse_capture(true);
    if should_exit.get() {
        system.exit();
    }

    // ── Derive the frame ──

    let now = session.audio.position_ms();
    let total = session.audio.total_ms();
    let is_playing = session.audio.is_playing();

    let layout =
        Layout::measure_with(term_w as usize, term_h as usize, spectrum_style.get().is_visible());
    let inner = layout.inner_width;

    let note = {
        let mut a = analyzer.lock().unwrap();
        a.resize(inner * 2);
        a.note
    };

    let lines = visible_lines(&session, now, render_past.get(), &layout);

    let spectrum = {
        let a = analyzer.lock().unwrap();
        (layout.spectrum_rows > 0)
            .then(|| spectrum::render(&a, inner, layout.spectrum_rows, spectrum_style.get()))
    };

    element! {
        View(
            width: term_w,
            height: term_h,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        ) {
            View(
                flex_direction: FlexDirection::Column,
                border_style: BorderStyle::Round,
                border_color: config::PRIMARY_BORDER,
                padding_left: 5,
                padding_right: 5,
                padding_top: layout.padding_y(),
                padding_bottom: layout.padding_y(),
                width: layout.box_width as u32,
                align_items: AlignItems::Center,
            ) {
                #(header::render(
                    is_playing,
                    now,
                    note,
                    show_note.get(),
                    show_keybinds.get(),
                    spectrum_style.get(),
                ))
                #(rule(&layout))

                View(
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    width: 100pct,
                    margin_top: layout.lyric_margin(),
                    margin_bottom: layout.lyric_margin(),
                ) {
                    #(lines)
                }

                #(rule(&layout))

                // Percentage rather than a column count. A child pinned to
                // exactly the content width makes a full width sibling
                // overflow, and an overflowing Text wraps once per character,
                // which turns the panel into hundreds of blank rows.
                View(
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    width: 100pct,
                    margin_top: 1,
                ) {
                    #(layout.show_ticker.then(|| footer::ticker(
                        config::SONG_NAME,
                        now,
                        (inner as f32 * config::TICKER_WIDTH_RATIO) as usize,
                    )))
                    #(spectrum)
                    #(footer::timeline(
                        &session.envelope,
                        now,
                        total,
                        (inner as f32 * config::TIMELINE_WIDTH_RATIO) as usize,
                    ))
                    #(layout.show_transport
                        .then(|| footer::transport(session.clone(), now, is_playing)))
                }
            }
        }
    }
}

/// A horizontal rule, when there is room for one.
fn rule(layout: &Layout) -> Option<AnyElement<'static>> {
    layout.show_rules.then(|| {
        element! {
            Text(
                color: config::TIMELINE_REMAINING,
                content: config::SEPARATOR_HORIZONTAL.repeat(layout.inner_width),
            )
        }
        .into()
    })
}

/// The window of lyric lines centred on the active one.
fn visible_lines(
    session: &Session,
    now: i64,
    render_past: bool,
    layout: &Layout,
) -> Vec<AnyElement<'static>> {
    let sentences = &session.sentences;
    if sentences.is_empty() {
        return vec![element! {
            Text(color: config::PAUSED_INDICATOR, content: "No lyrics loaded.")
        }
        .into()];
    }

    let active_float = lyrics::active_index(sentences, now);
    let active = active_float.floor() as i64;
    let half = layout.half_window();
    let gap_is_active = sentences
        .get(active as usize)
        .is_some_and(|s| s.is_gap);

    let spacing = layout.line_spacing as u32;
    let blank = move || -> AnyElement<'static> {
        element! { View(margin_bottom: spacing) { Text(content: " ") } }.into()
    };

    // Lines after an upcoming instrumental break stay hidden, so the break
    // reads as a pause rather than a preview of what follows.
    let mut hit_future_gap = false;

    (active - half..=active + half)
        .map(|idx| {
            let Some(sentence) = usize::try_from(idx).ok().and_then(|i| sentences.get(i)) else {
                return blank();
            };

            let is_future = idx > active;
            if is_future && sentence.is_gap {
                hit_future_gap = true;
            }
            if hit_future_gap && is_future && !sentence.is_gap {
                return blank();
            }

            // Before the first manual seek, lines that finished before the
            // configured start time are left blank rather than shown as
            // already sung.
            if !render_past
                && !sentence.is_gap
                && sentence.end() < session.start_ms
            {
                return blank();
            }

            let status = match idx.cmp(&active) {
                std::cmp::Ordering::Less => Status::Past,
                std::cmp::Ordering::Equal => Status::Active,
                std::cmp::Ordering::Greater => Status::Future,
            };

            lyric_line::render(
                sentence,
                now,
                status,
                (idx as f32 - active_float).abs(),
                gap_is_active,
                spacing,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render an element and report how many terminal rows it occupies.
    fn height(mut e: AnyElement<'static>, width: usize) -> usize {
        let mut buf = Vec::new();
        e.render(Some(width)).write(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).lines().count()
    }

    /// A child pinned to exactly the panel's content width used to make a
    /// full width sibling overflow. The overflowing `Text` then wrapped once
    /// per character, so a two row footer became seventy, the panel grew past
    /// two hundred rows, and everything the terminal could show was blank.
    #[test]
    fn a_full_width_child_beside_a_full_width_rule_stays_one_row_each() {
        let panel = |child: AnyElement<'static>| {
            element! {
                View(
                    border_style: BorderStyle::Round,
                    width: 78u32,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding_left: 5,
                    padding_right: 5,
                ) {
                    Text(content: "─".repeat(66))
                    #(child)
                }
            }
            .into()
        };

        let percent = panel(element! { View(width: 100pct) { Text(content: "x") } }.into());
        // 2 border rows, the rule, and the child.
        assert_eq!(height(percent, 100), 4);
    }

    /// The measured layout has to agree with what the elements actually
    /// occupy, or the panel silently overflows again.
    /// The spectrum sits next to a rule that spans the full content width.
    /// Give it a width of its own that equals that content width and the pair
    /// overflows, the rule wraps one character per column, and the panel turns
    /// into hundreds of blank rows with nothing visible in the terminal.
    ///
    /// This has now happened twice, so it is checked against the real
    /// component rather than a stand in.
    #[test]
    fn no_spectrum_style_can_blow_up_the_panel() {
        use crate::analysis::Analyzer;
        use spectrum::Style;

        let l = Layout::measure_with(80, 24, true);
        let mut a = Analyzer::new(l.inner_width * 2);
        a.levels = vec![0.5; l.inner_width * 2];
        a.peaks = vec![0.9; l.inner_width * 2];

        for style in Style::DRAWN {
            let panel = element! {
                View(
                    border_style: BorderStyle::Round,
                    width: l.box_width as u32,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding_left: 5,
                    padding_right: 5,
                ) {
                    Text(content: "─".repeat(l.inner_width))
                    #(spectrum::render(&a, l.inner_width, l.spectrum_rows, style))
                }
            };

            // Two border rows, the rule, and the spectrum. Nothing else.
            assert_eq!(
                height(panel.into(), 100),
                3 + l.spectrum_rows,
                "{style:?} overflowed the panel"
            );
        }
    }

    /// Why the panel keeps a spare column.
    ///
    /// A spectrum row drawn at exactly the width of its container competes for
    /// the last column with any full width sibling, and whichever loses is
    /// squeezed to one column and wraps once per character. That turns four
    /// rows of spectrum into sixty odd, and since the panel is centred
    /// vertically the extra height pushes up over the title.
    ///
    /// One spare column and nothing has to compete.
    #[test]
    fn a_spare_column_keeps_the_spectrum_the_height_it_should_be() {
        use crate::analysis::Analyzer;
        use spectrum::Style;

        let width = 40usize;
        let rows = 3usize;

        let mut a = Analyzer::new(width * 2);
        a.levels = vec![0.7; width * 2];
        a.peaks = vec![0.95; width * 2];

        for style in Style::DRAWN {
            let with_slack = element! {
                View(width: (width + 1) as u32, flex_direction: FlexDirection::Column) {
                    Text(content: "\u{2500}".repeat(width))
                    #(spectrum::render(&a, width, rows, style))
                }
            };
            assert_eq!(
                height(with_slack.into(), 100),
                1 + rows,
                "{style:?} did not fit even with a column to spare"
            );
        }
    }

    #[test]
    fn the_measured_height_matches_what_gets_drawn() {
        let l = Layout::measure_with(80, 24, true);

        let chrome = element! {
            View(
                border_style: BorderStyle::Round,
                width: l.box_width as u32,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding_left: 5,
                padding_right: 5,
                padding_top: l.padding_y(),
                padding_bottom: l.padding_y(),
            ) {
                Text(content: "header")
                #(rule(&l))
                View(
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    width: 100pct,
                    margin_top: l.lyric_margin(),
                    margin_bottom: l.lyric_margin(),
                ) {
                    #((0..l.window).map(|_| element! {
                        View(margin_bottom: l.line_spacing as u32) { Text(content: "lyric") }
                    }))
                }
                #(rule(&l))
                View(flex_direction: FlexDirection::Column, width: 100pct, margin_top: 1) {
                    #(l.show_ticker.then(|| -> AnyElement<'static> {
                        element! { Text(content: "ticker") }.into()
                    }))
                    #((0..l.spectrum_rows).map(|_| element! { Text(content: "bars") }))
                    #((0..footer::timeline_rows()).map(|_| element! { Text(content: "timeline") }))
                    #(l.show_transport.then(|| -> AnyElement<'static> {
                        element! { View(margin_top: 1) { Text(content: "transport") } }.into()
                    }))
                }
            }
        };

        assert_eq!(height(chrome.into(), 100), l.rows_needed());
        assert!(l.rows_needed() <= 24);
    }
}
