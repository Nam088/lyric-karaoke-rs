//! Terminal karaoke player.
//!
//! A Rust port of the React + Ink build. The audio path no longer shells out
//! to ffmpeg, which is what made the original macOS only: it played through
//! `-f audiotoolbox` and paused with `SIGSTOP`, neither of which exists on
//! Windows.

mod analysis;
mod audio;
mod braille;
mod color;
mod config;
mod lyrics;
mod ui;

use std::sync::Arc;

use anyhow::{Context, Result};
use iocraft::prelude::*;

use analysis::envelope::Envelope;
use audio::Audio;
use ui::{App, Session};

fn main() -> Result<()> {
    let session = load().context("starting up")?;

    // `--frame` draws one pass to stdout and exits, for checking the layout
    // without taking over the terminal.
    if std::env::args().any(|a| a == "--frame") {
        let width = std::env::args()
            .nth(2)
            .and_then(|w| w.parse().ok())
            .unwrap_or(100);
        return element!(App(session: Some(session)))
            .render(Some(width))
            .write_ansi(std::io::stdout())
            .map_err(Into::into);
    }

    smol::block_on(element!(App(session: Some(session))).fullscreen())?;

    Ok(())
}

fn load() -> Result<Arc<Session>> {
    let sentences = lyrics::load(config::LYRIC_JSON)?;
    let path = audio::resolve_path(config::DATA_DIR, config::SONG_FILE)?;

    // Kicked off before the audio device is opened so the scan overlaps
    // startup. The timeline draws a plain bar until it lands.
    let envelope = Envelope::scan(path.clone());

    let audio = Audio::open(&path)?;

    let start_ms = lyrics::parse_time(config::START_TIME);
    if start_ms > 0 {
        audio.seek_ms(start_ms);
    }

    Ok(Arc::new(Session { audio, sentences, envelope, start_ms }))
}
