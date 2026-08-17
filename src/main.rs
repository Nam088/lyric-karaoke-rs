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
mod playlist;
mod ui;

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use iocraft::prelude::*;

use analysis::envelope::Envelope;
use audio::Audio;
use playlist::{Playlist, Track};
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
    let playlist_path = Path::new(config::DATA_DIR).join(config::PLAYLIST_FILE);
    let playlist = if playlist_path.exists() {
        Playlist::load(&playlist_path)?
    } else {
        Playlist {
            tracks: vec![Track {
                id: "default".into(),
                title: config::SONG_NAME.into(),
                artist: "Unknown Artist".into(),
                audio: config::SONG_FILE.into(),
                lyrics: config::LYRIC_JSON.into(),
            }],
        }
    };

    let track_idx = 0;
    let track = playlist.get(track_idx).cloned().unwrap_or_else(|| Track {
        id: "default".into(),
        title: config::SONG_NAME.into(),
        artist: "Unknown Artist".into(),
        audio: config::SONG_FILE.into(),
        lyrics: config::LYRIC_JSON.into(),
    });

    let lyric_path = audio::resolve_path(config::DATA_DIR, &track.lyrics)?;
    let sentences = lyrics::load(&lyric_path)?;

    let audio_path = audio::resolve_path(config::DATA_DIR, &track.audio)?;
    let envelope = Envelope::scan(audio_path.clone());
    let audio = Audio::open(&audio_path)?;

    let start_ms = lyrics::parse_time(config::START_TIME);
    if start_ms > 0 {
        audio.seek_ms(start_ms);
    }

    Ok(Arc::new(Session::new(
        playlist,
        track_idx,
        track,
        audio,
        sentences,
        envelope,
        start_ms,
    )))
}
