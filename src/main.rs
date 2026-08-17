//! Terminal karaoke player with SQLite music folders manager.

mod analysis;
mod audio;
mod braille;
mod color;
mod config;
mod db;
mod i18n;
mod lyrics;
mod playlist;
mod ui;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use iocraft::prelude::*;
use rusqlite::Connection;

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
    let args: Vec<String> = std::env::args().collect();
    let mut custom_dir: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--frame" {
            i += 2;
        } else if !args[i].starts_with('-') {
            custom_dir = Some(PathBuf::from(&args[i]));
            break;
        } else {
            i += 1;
        }
    }

    let _ = std::fs::create_dir_all(config::DATA_DIR);
    let db_path = PathBuf::from(config::DATA_DIR).join("library.db");

    // Open SQLite database to manage allowed music folders
    let conn = Connection::open(&db_path)
        .with_context(|| format!("opening SQLite database at {}", db_path.display()))?;

    db::init_db(&conn)?;

    if let Some(dir) = custom_dir {
        db::add_folder(&conn, &dir)?;
    }

    // Dynamically scan all allowed folders in DB to build playlist
    let scanned_tracks = db::load_all_tracks_from_db_folders(&conn)?;

    let playlist = if !scanned_tracks.is_empty() {
        Playlist {
            tracks: scanned_tracks,
        }
    } else {
        let playlist_path = Path::new(config::DATA_DIR).join(config::PLAYLIST_FILE);
        if playlist_path.exists() {
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

    let sentences = if track.lyrics.is_empty() {
        Vec::new()
    } else if let Ok(lyric_path) = audio::resolve_path(config::DATA_DIR, &track.lyrics) {
        lyrics::load(&lyric_path).unwrap_or_default()
    } else {
        Vec::new()
    };

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
