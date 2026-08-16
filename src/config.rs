//! Everything tunable lives here, ported from the TypeScript `constants.ts`.
//!
//! The original file mixed live settings with a few values nothing read any
//! more (`PATHS.AUDIO_FILE`, which `app.tsx` bypassed entirely). Those are
//! gone. What remains is only what the app actually uses.

// ── Strings ────────────────────────────────────────────────────────────

pub const APP_NAME: &str = "Karaoke";
pub const LIVE_LABEL: &str = "● LIVE";
pub const PAUSED_LABEL: &str = "⏸ PAUSED";
pub const SONG_NAME: &str = "Tìm Em - Hngle, Bảo Anh";
pub const SONG_FILE: &str = "b.mp3";
pub const GAP_TEXT: &str = "♫  ♪  ♫  ♪  ♫  ♪  ♫";
pub const GAP_ALT_TEXT: &str = "♪  ♫  ♪  ♫  ♪  ♫  ♪";

// ── Colours & Themes ───────────────────────────────────────────────────

/// Default color theme on startup. Cycled live with the `C` key.
pub const DEFAULT_THEME: crate::color::ThemePreset = crate::color::ThemePreset::Emerald;

// Lyric animation timings
pub const AFTERGLOW_DURATION_MS: i64 = 240;
pub const ANTICIPATION_MS: i64 = 150;

// ── Timings, all milliseconds ──────────────────────────────────────────

/// Where playback starts. Format is `H:MM:SS` or `MM:SS`.
pub const START_TIME: &str = "0:00:00";

/// One frame of the render loop. Also the character fill resolution.
pub const TICK_INTERVAL_MS: u64 = 30;

pub const LIVE_BLINK_MS: f64 = 500.0;
pub const GAP_PATTERN_MS: f64 = 800.0;
pub const DANCING_INDICATOR_MS: f64 = 450.0;
pub const TICKER_SCROLL_MS: f64 = 350.0;
pub const SCROLL_TRANSITION_MS: f64 = 500.0;

pub const GAP_INITIAL_THRESHOLD_MS: i64 = 3000;
pub const GAP_INTER_THRESHOLD_MS: i64 = 2000;
pub const GAP_BUFFER_MS: i64 = 500;
pub const SEEK_STEP_MS: i64 = 5000;

// The TypeScript build also had SEEK_DEBOUNCE_MS. It existed only because
// seeking meant killing an ffmpeg process and spawning a new one, which was
// far too expensive to do on every arrow key. `Player::try_seek` is
// immediate, so there is nothing left to debounce.

// ── Animation ──────────────────────────────────────────────────────────
//
// Every effect here is derived from the playback clock (`now`), so it
// costs nothing beyond arithmetic that was already running.

/// Breathing glow: the active line subtly pulses brighter.
pub const BREATHING_SPEED_MS: f64 = 600.0;
pub const BREATHING_INTENSITY: f32 = 0.15;

/// Wave ripple across future graphemes on the active line.
pub const WAVE_SPEED_MS: f64 = 400.0;
pub const WAVE_PHASE_OFFSET: f64 = 0.3;
pub const WAVE_INTENSITY: f32 = 0.12;

/// Twinkling for active instrumental break characters.
pub const GAP_TWINKLE_SPEED_MS: f64 = 300.0;
pub const GAP_TWINKLE_PHASE_OFFSET: f64 = 0.7;
pub const GAP_TWINKLE_INTENSITY: f32 = 0.4;

/// Per-character phase offset for the header shimmer sweep.
pub const SHIMMER_CHAR_OFFSET: f64 = 0.4;

// ── Symbols ────────────────────────────────────────────────────────────

pub const MUSIC_NOTES: [&str; 1] = [" ♪ "];
pub const SEPARATOR_HORIZONTAL: &str = "─";
pub const VBAR: &str = " │ ";

// Progress bar.
pub const TIMELINE_FILLED: char = '━';
pub const TIMELINE_EMPTY: char = '─';
pub const TIMELINE_MARKER: char = '●';
pub const TIMELINE_CAP_LEFT: &str = " ╶";
pub const TIMELINE_CAP_RIGHT: &str = "╴ ";

/// Draw the song's loudness over time instead of a plain bar.
///
/// Off by default. It carries more information, but at two rows of braille it
/// reads as texture rather than as a position you can judge at a glance, which
/// is what a progress bar is for.
pub const WAVEFORM_TIMELINE: bool = false;

// ── Layout ─────────────────────────────────────────────────────────────

/// How many lyric lines are on screen at once. Must be odd so one sits dead
/// centre.
pub const WINDOW_SIZE: usize = 7;
pub const MAX_BOX_WIDTH: usize = 120;

/// Blank rows between lyric lines (0 = compact/adjacent, 1 = empty row between each).
pub const LINE_SPACING: usize = 1;

/// Columns left empty either side of the panel.
///
/// Insurance against glyphs the terminal draws wider than the layout measured
/// them. Musical notes, `●`, `◄`, `►` and `▌` are all East Asian Ambiguous:
/// the width tables call them one column, and a terminal configured for CJK
/// draws them as two. Filling right up to the edge means one such glyph wraps
/// the line, which shifts every row below it and makes the next repaint land
/// on the wrong rows.
pub const SAFE_MARGIN: usize = 4;
pub const TICKER_WIDTH_RATIO: f32 = 0.8;
pub const TIMELINE_WIDTH_RATIO: f32 = 0.7;
pub const SHOW_KEYBINDS: bool = false;
pub const RENDER_PAST_ON_START: bool = false;

/// Whether the detected note is in the header at startup. Off by default,
/// like the spectrum: useful when you want it, clutter when you do not.
/// Toggled with `N`.
pub const SHOW_NOTE: bool = false;

/// Which end of the `S` cycle the app starts on.
///
/// Off by default: the spectrum is decoration, the lyrics are the point, and
/// the rows it would occupy go back to them. Press `S` to bring it up.
pub const SHOW_SPECTRUM: bool = false;

/// Terminal rows given to the spectrum. Each row is 4 braille pixels tall.
/// The layout gives some of these up before it drops a lyric line.
pub const SPECTRUM_ROWS: usize = 4;

// ── Paths ──────────────────────────────────────────────────────────────

pub const LYRIC_JSON: &str = "data/lr.json";
pub const DATA_DIR: &str = "data";
