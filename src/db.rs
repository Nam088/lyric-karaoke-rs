//! SQLite database management for configured music folders and dynamic scanning.

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

/// Initialize SQLite schema for managing allowed/configured music folders.
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        );",
    )
    .context("initializing sqlite folders table")?;

    // Seed default "data" folder if the table is currently empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM folders", [], |r| r.get(0))?;
    if count == 0 {
        add_folder(conn, "data")?;
    }

    Ok(())
}

/// Add an allowed music folder to the SQLite database.
pub fn add_folder(conn: &Connection, folder_path: impl AsRef<Path>) -> Result<()> {
    let path_str = folder_path.as_ref().to_string_lossy().to_string();
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO folders (path, enabled, created_at)
         VALUES (?1, 1, ?2)
         ON CONFLICT(path) DO UPDATE SET enabled = 1",
        params![path_str, now_ts],
    )?;

    Ok(())
}

/// Remove a configured music folder from SQLite.
pub fn remove_folder(conn: &Connection, folder_path: impl AsRef<Path>) -> Result<()> {
    let path_str = folder_path.as_ref().to_string_lossy().to_string();
    conn.execute("DELETE FROM folders WHERE path = ?1", params![path_str])?;
    Ok(())
}

/// List all enabled music folders configured in SQLite.
pub fn list_folders(conn: &Connection) -> Result<Vec<PathBuf>> {
    init_db(conn)?;

    let mut stmt = conn.prepare("SELECT path FROM folders WHERE enabled = 1 ORDER BY id ASC")?;
    let folder_rows = stmt.query_map([], |row| {
        let path_str: String = row.get(0)?;
        Ok(PathBuf::from(path_str))
    })?;

    let mut folders = Vec::new();
    for f in folder_rows.flatten() {
        folders.push(f);
    }
    Ok(folders)
}

/// Scan a single folder on disk dynamically for audio files.
/// Does NOT write songs to DB; directly builds in-memory Track list.
pub fn scan_folder_for_tracks(folder: impl AsRef<Path>) -> Result<Vec<Track>> {
    let folder = folder.as_ref();
    if !folder.exists() || !folder.is_dir() {
        return Ok(Vec::new());
    }

    // Check if playlist.json exists in this folder for optional predefined metadata
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

    let mut tracks = Vec::new();

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

        // Detect matching lyrics file in the same folder (.json or .lrc)
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
                    candidate.to_string_lossy().to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else if json_lyric.exists() {
            json_lyric.to_string_lossy().to_string()
        } else if lrc_lyric.exists() {
            lrc_lyric.to_string_lossy().to_string()
        } else {
            String::new()
        };

        tracks.push(Track {
            id,
            title,
            artist,
            audio: path.to_string_lossy().to_string(),
            lyrics: lyrics_path,
        });
    }

    tracks.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(tracks)
}

/// Read all configured folders from SQLite and dynamically scan each to load the complete playlist.
pub fn load_all_tracks_from_db_folders(conn: &Connection) -> Result<Vec<Track>> {
    let folders = list_folders(conn)?;
    let mut all_tracks = Vec::new();

    for folder in folders {
        if let Ok(mut tracks) = scan_folder_for_tracks(&folder) {
            all_tracks.append(&mut tracks);
        }
    }

    Ok(all_tracks)
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
    fn sqlite_manage_folders_and_dynamic_scan() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let temp_dir = std::env::temp_dir().join("karaoke_test_folder_mgr");
        let _ = std::fs::create_dir_all(&temp_dir);

        let song1 = temp_dir.join("Singer - Test Song.mp3");
        let song2 = temp_dir.join("Instrumental Track.wav");
        std::fs::write(&song1, b"dummy audio").unwrap();
        std::fs::write(&song2, b"dummy audio").unwrap();

        // Add custom folder to SQLite
        add_folder(&conn, &temp_dir).unwrap();

        let folders = list_folders(&conn).unwrap();
        assert!(folders.contains(&temp_dir));

        let tracks = load_all_tracks_from_db_folders(&conn).unwrap();
        assert!(tracks.iter().any(|t| t.title == "Test Song" && t.artist == "Singer"));
        assert!(tracks.iter().any(|t| t.title == "Instrumental Track" && t.lyrics.is_empty()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
