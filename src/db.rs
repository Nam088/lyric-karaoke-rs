//! SQLite database management for configured music folders and dynamic scanning.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::playlist::Track;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac"];

/// Parse track title and artist cleanly from a filename stem (without extension).
///
/// Handles standard music naming conventions:
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

/// Normalize path for consistent SQLite storage and comparison.
pub fn normalize_folder_path(folder_path: impl AsRef<Path>) -> PathBuf {
    let p = folder_path.as_ref();
    if let Ok(canonical) = p.canonicalize() {
        canonical
    } else {
        p.to_path_buf()
    }
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
    let norm = normalize_folder_path(folder_path);
    let path_str = norm.to_string_lossy().to_string();
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
    let p = folder_path.as_ref();
    let p_str = p.to_string_lossy().to_string();
    let norm_str = normalize_folder_path(p).to_string_lossy().to_string();
    conn.execute("DELETE FROM folders WHERE path = ?1 OR path = ?2", params![p_str, norm_str])?;
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
    let mut seen = HashSet::new();

    for f in folder_rows.flatten() {
        let norm = normalize_folder_path(&f);
        if seen.insert(norm.clone()) {
            folders.push(norm);
        }
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

    let mut tracks: Vec<Track> = Vec::new();

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
/// Only filters out duplicate occurrences of the exact same file path on disk.
pub fn load_all_tracks_from_db_folders(conn: &Connection) -> Result<Vec<Track>> {
    let folders = list_folders(conn)?;
    let mut all_tracks: Vec<Track> = Vec::new();
    let mut seen_canonical_audio: HashSet<PathBuf> = HashSet::new();

    for folder in folders {
        if let Ok(tracks) = scan_folder_for_tracks(&folder) {
            for track in tracks {
                let can_path = std::fs::canonicalize(&track.audio)
                    .unwrap_or_else(|_| PathBuf::from(&track.audio));
                if seen_canonical_audio.insert(can_path) {
                    all_tracks.push(track);
                }
            }
        }
    }

    all_tracks.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(all_tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_filename_conventions() {
        let (t, a) = parse_title_and_artist("Son Tung M-TP - Dung Ve Tre");
        assert_eq!(t, "Dung Ve Tre");
        assert_eq!(a, "Son Tung M-TP");

        let (t, a) = parse_title_and_artist("01. Vu - Dong Kiem Em");
        assert_eq!(t, "Dong Kiem Em");
        assert_eq!(a, "Vu");

        let (t, a) = parse_title_and_artist("hqhuy - mua he nam ay (2)");
        assert_eq!(t, "mua he nam ay (2)");
        assert_eq!(a, "hqhuy");

        let (t, a) = parse_title_and_artist("Dreamers (9)");
        assert_eq!(t, "Dreamers (9)");
        assert_eq!(a, "Unknown Artist");

        let (t, a) = parse_title_and_artist("SimpleSong");
        assert_eq!(t, "SimpleSong");
        assert_eq!(a, "Unknown Artist");
    }

    #[test]
    fn sqlite_manage_folders_and_dynamic_scan() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let _ = remove_folder(&conn, "data");

        let temp_dir = std::env::temp_dir().join("test_karaoke_music_scan");
        let _ = std::fs::create_dir_all(&temp_dir);

        let audio1 = temp_dir.join("Artist - Song One.mp3");
        let _ = std::fs::write(&audio1, b"fake mp3 data");

        let audio2 = temp_dir.join("Other - Song Two.wav");
        let _ = std::fs::write(&audio2, b"fake wav data");

        add_folder(&conn, &temp_dir).unwrap();

        let folders = list_folders(&conn).unwrap();
        assert!(folders.iter().any(|f| f.ends_with("test_karaoke_music_scan")));

        let tracks = load_all_tracks_from_db_folders(&conn).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Song One");
        assert_eq!(tracks[1].title, "Song Two");

        remove_folder(&conn, &temp_dir).unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
