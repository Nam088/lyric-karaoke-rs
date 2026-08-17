//! SQLite database management and music directory auto-discovery.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::playlist::Track;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac"];

/// Parse track title and artist cleanly from a filename stem (without extension).
///
/// Handles common music naming conventions:
/// - "Artist - Title" => (Title, Artist)
/// - "01. Artist - Title" => (Title, Artist)
/// - "01 - Title" => (Title, Unknown Artist)
/// - "Song_Name" => (Song Name, Unknown Artist)
pub fn parse_title_and_artist(file_stem: &str) -> (String, String) {
    let mut clean = file_stem.replace('_', " ").trim().to_string();

    // Strip leading track number like "01. ", "01 - ", "01 "
    if let Some(pos) = clean.find(['.', '-', ' ']) {
        let prefix = &clean[..pos];
        if prefix.chars().all(|c| c.is_ascii_digit()) && prefix.len() <= 3 {
            clean = clean[pos + 1..].trim_start_matches(['.', '-', ' ']).trim().to_string();
        }
    }

    if let Some((part1, part2)) = clean.split_once(" - ") {
        let p1 = part1.trim().to_string();
        let p2 = part2.trim().to_string();
        if !p1.is_empty() && !p2.is_empty() {
            // By music convention "Artist - Title"
            return (p2, p1);
        }
    }

    (clean, "Unknown Artist".to_string())
}

/// Initialize SQLite schema for the music library.
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS songs (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            audio_path TEXT NOT NULL UNIQUE,
            lyrics_path TEXT,
            duration_ms INTEGER DEFAULT 0,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_songs_audio_path ON songs(audio_path);
        CREATE INDEX IF NOT EXISTS idx_songs_title ON songs(title);",
    )
    .context("initializing sqlite songs table")?;
    Ok(())
}

/// Scan a folder for audio files, detect matching lyrics (if any),
/// upsert into the SQLite database, and return the updated library.
pub fn scan_and_sync_folder(conn: &Connection, folder: impl AsRef<Path>) -> Result<Vec<Track>> {
    init_db(conn)?;

    let folder = folder.as_ref();
    if !folder.exists() || !folder.is_dir() {
        return Ok(Vec::new());
    }

    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Check if a playlist.json exists in the folder for predefined rich metadata
    let playlist_manifest = folder.join("playlist.json");
    let predefined_tracks: Vec<Track> = if playlist_manifest.exists() {
        std::fs::read_to_string(&playlist_manifest)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let entries = std::fs::read_dir(folder)
        .with_context(|| format!("reading directory {}", folder.display()))?;

    let tx = conn.unchecked_transaction()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();

        if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        // Check if predefined in playlist.json
        let predefined = predefined_tracks.iter().find(|t| {
            t.audio == file_name || Path::new(&t.audio).file_name().and_then(|f| f.to_str()) == Some(file_name)
        });

        let (title, artist, id) = if let Some(p) = predefined {
            (p.title.clone(), p.artist.clone(), p.id.clone())
        } else {
            let (t, a) = parse_title_and_artist(stem);
            let safe_id = stem.to_lowercase().replace([' ', '_', '.'], "-");
            (t, a, safe_id)
        };

        // Detect matching lyrics file
        let json_lyric = folder.join(format!("{}.json", stem));
        let lrc_lyric = folder.join(format!("{}.lrc", stem));

        let lyrics_path = if let Some(p) = predefined {
            if !p.lyrics.is_empty() {
                let candidate = if Path::new(&p.lyrics).is_absolute() {
                    PathBuf::from(&p.lyrics)
                } else {
                    folder.join(&p.lyrics)
                };
                if candidate.exists() {
                    Some(candidate.to_string_lossy().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else if json_lyric.exists() {
            Some(json_lyric.to_string_lossy().to_string())
        } else if lrc_lyric.exists() {
            Some(lrc_lyric.to_string_lossy().to_string())
        } else {
            None
        };

        let audio_path_str = path.to_string_lossy().to_string();

        tx.execute(
            "INSERT INTO songs (id, title, artist, audio_path, lyrics_path, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(audio_path) DO UPDATE SET
                 title = excluded.title,
                 artist = excluded.artist,
                 lyrics_path = COALESCE(excluded.lyrics_path, songs.lyrics_path),
                 updated_at = excluded.updated_at",
            params![id, title, artist, audio_path_str, lyrics_path, now_ts],
        )?;
    }

    tx.commit()?;

    // Fetch all tracks from SQLite database
    let mut stmt = conn.prepare(
        "SELECT id, title, artist, audio_path, lyrics_path FROM songs ORDER BY title ASC",
    )?;

    let track_rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let artist: String = row.get(2)?;
        let audio: String = row.get(3)?;
        let lyrics: Option<String> = row.get(4)?;

        Ok(Track {
            id,
            title,
            artist,
            audio,
            lyrics: lyrics.unwrap_or_default(),
        })
    })?;

    let mut tracks = Vec::new();
    for track in track_rows.flatten() {
        tracks.push(track);
    }

    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_filename_conventions() {
        let (title, artist) = parse_title_and_artist("Vu - Mua He Nam Ay");
        assert_eq!(title, "Mua He Nam Ay");
        assert_eq!(artist, "Vu");

        let (title, artist) = parse_title_and_artist("01. Son Tung M-TP - Dung Ve Tre");
        assert_eq!(title, "Dung Ve Tre");
        assert_eq!(artist, "Son Tung M-TP");

        let (title, artist) = parse_title_and_artist("03 - Nothing Without You");
        assert_eq!(title, "Nothing Without You");
        assert_eq!(artist, "Unknown Artist");

        let (title, artist) = parse_title_and_artist("Single_Song_Name");
        assert_eq!(title, "Single Song Name");
        assert_eq!(artist, "Unknown Artist");
    }

    #[test]
    fn sqlite_database_scan_and_sync() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let temp_dir = std::env::temp_dir().join("karaoke_test_scan");
        let _ = std::fs::create_dir_all(&temp_dir);

        let song1 = temp_dir.join("Singer - Test Song.mp3");
        let song2 = temp_dir.join("Instrumental Track.wav");
        std::fs::write(&song1, b"dummy audio").unwrap();
        std::fs::write(&song2, b"dummy audio").unwrap();

        let tracks = scan_and_sync_folder(&conn, &temp_dir).unwrap();
        assert_eq!(tracks.len(), 2);
        assert!(tracks.iter().any(|t| t.title == "Test Song" && t.artist == "Singer"));
        assert!(tracks.iter().any(|t| t.title == "Instrumental Track" && t.lyrics.is_empty()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
