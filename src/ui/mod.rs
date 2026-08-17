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
pub mod playlist_modal;
pub mod spectrum;

use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use anyhow::Result;
use iocraft::prelude::*;

use crate::analysis::{envelope::Envelope, Analyzer, FFT_SIZE};
use crate::audio::Audio;
use crate::color;
use crate::config;
use crate::lyrics::{self, Sentence};
use crate::playlist::{Playlist, Track};
use layout::Layout;
use lyric_line::Status;

/// Multi-track session with dynamic playlist, audio, lyrics, and envelope.
pub struct Session {
    pub audio: Audio,
    pub playlist: Arc<RwLock<Playlist>>,
    pub track_index: Arc<RwLock<usize>>,
    pub current_track: Arc<RwLock<Track>>,
    pub sentences: Arc<RwLock<Vec<Sentence>>>,
    pub envelope: Arc<RwLock<Envelope>>,
    pub start_ms: i64,
}

impl Session {
    pub fn new(
        playlist: Playlist,
        track_index: usize,
        current_track: Track,
        audio: Audio,
        sentences: Vec<Sentence>,
        envelope: Envelope,
        start_ms: i64,
    ) -> Self {
        Self {
            audio,
            playlist: Arc::new(RwLock::new(playlist)),
            track_index: Arc::new(RwLock::new(track_index)),
            current_track: Arc::new(RwLock::new(current_track)),
            sentences: Arc::new(RwLock::new(sentences)),
            envelope: Arc::new(RwLock::new(envelope)),
            start_ms,
        }
    }

    pub fn switch_track(&self, index: usize) -> Result<()> {
        let (track, total_tracks) = {
            let p = self.playlist.read().unwrap();
            if p.is_empty() {
                return Ok(());
            }
            let idx = index % p.len();
            (p.get(idx).cloned().unwrap(), p.len())
        };

        let audio_path = crate::audio::resolve_path(config::DATA_DIR, &track.audio)?;
        let sentences = if track.lyrics.is_empty() {
            Vec::new()
        } else if let Ok(lyric_path) = crate::audio::resolve_path(config::DATA_DIR, &track.lyrics) {
            crate::lyrics::load(&lyric_path).unwrap_or_default()
        } else {
            Vec::new()
        };
        let envelope = crate::analysis::envelope::Envelope::scan(audio_path.clone());

        self.audio.load_file(&audio_path)?;

        {
            let mut s = self.sentences.write().unwrap();
            *s = sentences;
        }
        {
            let mut e = self.envelope.write().unwrap();
            *e = envelope;
        }
        {
            let mut t = self.current_track.write().unwrap();
            *t = track;
        }
        {
            let mut ti = self.track_index.write().unwrap();
            *ti = index % total_tracks;
        }

        Ok(())
    }

    pub fn next_track(&self) -> Result<()> {
        let next_idx = {
            let p = self.playlist.read().unwrap();
            let curr = *self.track_index.read().unwrap();
            p.next_index(curr)
        };
        self.switch_track(next_idx)
    }

    pub fn prev_track(&self) -> Result<()> {
        let prev_idx = {
            let p = self.playlist.read().unwrap();
            let curr = *self.track_index.read().unwrap();
            p.prev_index(curr)
        };
        self.switch_track(prev_idx)
    }

    pub fn add_music_folder(&self, folder_path: impl AsRef<std::path::Path>) -> Result<usize> {
        let db_path = std::path::PathBuf::from(config::DATA_DIR).join("library.db");
        let conn = rusqlite::Connection::open(&db_path)?;
        crate::db::add_folder(&conn, &folder_path)?;
        let tracks = crate::db::load_all_tracks_from_db_folders(&conn)?;
        let count = tracks.len();
        {
            let mut p = self.playlist.write().unwrap();
            p.tracks = tracks;
        }
        Ok(count)
    }

    pub fn remove_music_folder(&self, folder_path: impl AsRef<std::path::Path>) -> Result<usize> {
        let db_path = std::path::PathBuf::from(config::DATA_DIR).join("library.db");
        let conn = rusqlite::Connection::open(&db_path)?;
        crate::db::remove_folder(&conn, &folder_path)?;
        let tracks = crate::db::load_all_tracks_from_db_folders(&conn)?;
        let count = tracks.len();
        {
            let mut p = self.playlist.write().unwrap();
            p.tracks = tracks;
        }
        Ok(count)
    }

    pub fn list_music_folders(&self) -> Vec<std::path::PathBuf> {
        let db_path = std::path::PathBuf::from(config::DATA_DIR).join("library.db");
        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            crate::db::list_folders(&conn).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn rescan_music_folders(&self) -> Result<usize> {
        let db_path = std::path::PathBuf::from(config::DATA_DIR).join("library.db");
        let conn = rusqlite::Connection::open(&db_path)?;
        let tracks = crate::db::load_all_tracks_from_db_folders(&conn)?;
        let count = tracks.len();
        {
            let mut p = self.playlist.write().unwrap();
            p.tracks = tracks;
        }
        Ok(count)
    }
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
    let mut show_playlist = hooks.use_state(|| false);
    let mut modal_tab = hooks.use_state(playlist_modal::ModalTab::default);
    let mut folder_cursor = hooks.use_state(|| 0usize);
    let mut is_adding_folder = hooks.use_state(|| false);
    let mut folder_input = hooks.use_state(String::new);
    let mut playlist_cursor = hooks.use_state(|| *session.track_index.read().unwrap());
    let mut render_past = hooks.use_state(|| config::RENDER_PAST_ON_START);
    let mut should_exit = hooks.use_state(|| false);
    let mut color_theme = hooks.use_state(|| config::DEFAULT_THEME);
    let mut lang = hooks.use_state(crate::i18n::Language::default);
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

            // Auto-advance to next track when song finishes
            if s.audio.is_ended() && s.audio.position_ms() >= s.audio.total_ms() - 200 {
                let _ = s.next_track();
            }

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

        let is_modal = show_playlist.get();
        if is_modal {
            if is_adding_folder.get() {
                match code {
                    KeyCode::Esc => {
                        is_adding_folder.set(false);
                        folder_input.set(String::new());
                    }
                    KeyCode::Enter => {
                        let path_str = folder_input.read().clone();
                        if !path_str.trim().is_empty() {
                            let _ = s.add_music_folder(path_str.trim());
                        }
                        is_adding_folder.set(false);
                        folder_input.set(String::new());
                    }
                    KeyCode::Backspace => {
                        folder_input.write().pop();
                    }
                    KeyCode::Char(c) => {
                        folder_input.write().push(c);
                    }
                    _ => {}
                }
                return;
            }

            match modal_tab.get() {
                playlist_modal::ModalTab::Folders => {
                    match code {
                        KeyCode::Esc | KeyCode::Char('l') | KeyCode::Char('L') => {
                            show_playlist.set(false);
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Char('1') => {
                            modal_tab.set(playlist_modal::ModalTab::Tracks);
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            folder_input.set(String::new());
                            is_adding_folder.set(true);
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            let folders = s.list_music_folders();
                            let fc = folder_cursor.get();
                            if let Some(f) = folders.get(fc) {
                                let _ = s.remove_music_folder(f);
                            }
                            let remaining = s.list_music_folders().len();
                            if folder_cursor.get() >= remaining && remaining > 0 {
                                folder_cursor.set(remaining - 1);
                            }
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            let _ = s.rescan_music_folders();
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            let f_len = s.list_music_folders().len();
                            if f_len > 0 {
                                let curr = folder_cursor.get();
                                let prev = if curr == 0 { f_len - 1 } else { curr - 1 };
                                folder_cursor.set(prev);
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            let f_len = s.list_music_folders().len();
                            if f_len > 0 {
                                let curr = folder_cursor.get();
                                let next = (curr + 1) % f_len;
                                folder_cursor.set(next);
                            }
                        }
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            lang.set(lang.get().next());
                        }
                        KeyCode::Char(' ') => s.audio.toggle(),
                        KeyCode::Char('q') | KeyCode::Char('Q') => should_exit.set(true),
                        _ => {}
                    }
                }
                playlist_modal::ModalTab::Tracks => {
                    match code {
                        KeyCode::Esc | KeyCode::Char('l') | KeyCode::Char('L') => {
                            show_playlist.set(false);
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Char('2') => {
                            modal_tab.set(playlist_modal::ModalTab::Folders);
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            let p_len = s.playlist.read().unwrap().len();
                            if p_len > 0 {
                                let curr = playlist_cursor.get();
                                let prev = if curr == 0 { p_len - 1 } else { curr - 1 };
                                playlist_cursor.set(prev);
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            let p_len = s.playlist.read().unwrap().len();
                            if p_len > 0 {
                                let curr = playlist_cursor.get();
                                let next = (curr + 1) % p_len;
                                playlist_cursor.set(next);
                            }
                        }
                        KeyCode::Enter => {
                            let target = playlist_cursor.get();
                            let _ = s.switch_track(target);
                            show_playlist.set(false);
                        }
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            lang.set(lang.get().next());
                        }
                        KeyCode::Char(' ') => s.audio.toggle(),
                        KeyCode::Char('q') | KeyCode::Char('Q') => should_exit.set(true),
                        _ => {}
                    }
                }
            }
            return;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => should_exit.set(true),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                modal_tab.set(playlist_modal::ModalTab::Folders);
                folder_cursor.set(0);
                show_playlist.set(true);
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                modal_tab.set(playlist_modal::ModalTab::Tracks);
                playlist_cursor.set(*s.track_index.read().unwrap());
                show_playlist.set(true);
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                let v = show_keybinds.get();
                show_keybinds.set(!v);
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                spectrum_style.set(spectrum_style.get().next());
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                color_theme.set(color_theme.get().next());
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                lang.set(lang.get().next());
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let v = show_note.get();
                show_note.set(!v);
            }
            KeyCode::Char('[') | KeyCode::Char('p') | KeyCode::Char('P') => {
                let _ = s.prev_track();
            }
            KeyCode::Char(']') | KeyCode::Char('o') | KeyCode::Char('O') => {
                let _ = s.next_track();
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

    let theme_preset = color_theme.get();
    let theme = theme_preset.theme();
    let current_lang = lang.get();

    let layout =
        Layout::measure_with(term_w as usize, term_h as usize, spectrum_style.get().is_visible());
    let inner = layout.inner_width;

    let note = {
        let mut a = analyzer.lock().unwrap();
        a.resize(inner * 2);
        a.note
    };

    let lines = visible_lines(
        session.clone(),
        now,
        render_past.get(),
        current_lang,
        &layout,
        &theme,
    );

    let spectrum = {
        let a = analyzer.lock().unwrap();
        (layout.spectrum_rows > 0)
            .then(|| spectrum::render(&a, inner, layout.spectrum_rows, spectrum_style.get(), &theme))
    };

    let track_display = session.current_track.read().unwrap().display_name();
    let envelope = session.envelope.read().unwrap().clone();

    let s_toggle = session.clone();
    let s_select = session.clone();
    let s_rescan = session.clone();
    let s_del = session.clone();

    let modal_overlay = show_playlist.get().then(|| {
        let input_str = folder_input.read().clone();
        element! {
            View(
                position: Position::Absolute,
                width: term_w,
                height: term_h,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            ) {
                #(playlist_modal::render(
                    session.clone(),
                    modal_tab.get(),
                    playlist_cursor.get(),
                    folder_cursor.get(),
                    is_adding_folder.get(),
                    &input_str,
                    layout.box_width.saturating_sub(10),
                    current_lang,
                    &theme,
                    move |idx| {
                        let _ = s_select.switch_track(idx);
                        playlist_cursor.set(idx);
                        show_playlist.set(false);
                    },
                    move || {
                        show_playlist.set(false);
                    },
                    move |tab| {
                        modal_tab.set(tab);
                    },
                    move || {
                        folder_input.set(String::new());
                        is_adding_folder.set(true);
                    },
                    move || {
                        let _ = s_rescan.rescan_music_folders();
                    },
                    move |idx| {
                        let folders = s_del.list_music_folders();
                        if let Some(f) = folders.get(idx) {
                            let _ = s_del.remove_music_folder(f);
                        }
                        let remaining = s_del.list_music_folders().len();
                        if folder_cursor.get() >= remaining && remaining > 0 {
                            folder_cursor.set(remaining - 1);
                        }
                    },
                ))
            }
        }
    });

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
                border_color: theme.border,
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
                    theme_preset,
                    current_lang,
                    &theme,
                    Some(session.clone()),
                ))
                #(rule(&layout, &theme))

                View(
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    width: 100pct,
                    margin_top: layout.lyric_margin(),
                    margin_bottom: layout.lyric_margin(),
                ) {
                    #(lines)
                }

                #(rule(&layout, &theme))

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
                        &track_display,
                        now,
                        (inner as f32 * config::TICKER_WIDTH_RATIO) as usize,
                        &theme,
                    )))
                    #(spectrum)
                    #(footer::timeline(
                        &envelope,
                        now,
                        total,
                        (inner as f32 * config::TIMELINE_WIDTH_RATIO) as usize,
                        &theme,
                        Some(session.clone()),
                    ))
                    #(layout.show_transport
                        .then(|| footer::transport(
                            session.clone(),
                            now,
                            is_playing,
                            &theme,
                            move || {
                                let v = show_playlist.get();
                                if !v {
                                    playlist_cursor.set(*s_toggle.track_index.read().unwrap());
                                }
                                show_playlist.set(!v);
                            },
                        )))
                }
            }

            #(modal_overlay)
        }
    }
}

/// A horizontal rule, when there is room for one.
fn rule(layout: &Layout, theme: &color::Theme) -> Option<AnyElement<'static>> {
    layout.show_rules.then(|| {
        element! {
            Text(
                color: theme.remaining,
                content: config::SEPARATOR_HORIZONTAL.repeat(layout.inner_width),
            )
        }
        .into()
    })
}

/// The window of lyric lines centred on the active one.
fn visible_lines(
    session: Arc<Session>,
    now: i64,
    render_past: bool,
    lang: crate::i18n::Language,
    layout: &Layout,
    theme: &color::Theme,
) -> Vec<AnyElement<'static>> {
    let sentences_guard = session.sentences.read().unwrap();
    let sentences = &*sentences_guard;
    if sentences.is_empty() {
        return vec![element! {
            View(
                margin_top: 2,
                margin_bottom: 2,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
            ) {
                Text(
                    color: theme.lyric_singing,
                    weight: Weight::Bold,
                    content: lang.no_lyrics_text().to_string(),
                )
                Text(
                    color: theme.keybinds_dim,
                    content: lang.no_lyrics_hints().to_string(),
                )
            }
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
                theme,
                Some(session.clone()),
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

        let theme = color::Theme::default();
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
                    #(spectrum::render(&a, l.inner_width, l.spectrum_rows, style, &theme))
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

        let theme = color::Theme::default();
        for style in Style::DRAWN {
            let with_slack = element! {
                View(width: (width + 1) as u32, flex_direction: FlexDirection::Column) {
                    Text(content: "─".repeat(width))
                    #(spectrum::render(&a, width, rows, style, &theme))
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
        let theme = color::Theme::default();

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
                #(rule(&l, &theme))
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
                #(rule(&l, &theme))
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
