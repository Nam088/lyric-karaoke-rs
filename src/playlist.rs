//! Playlist and track management.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    #[serde(default = "default_artist")]
    pub artist: String,
    pub audio: String,
    pub lyrics: String,
}

fn default_artist() -> String {
    "Unknown Artist".to_string()
}

impl Track {
    pub fn display_name(&self) -> String {
        if self.artist.is_empty() || self.artist == "Unknown Artist" {
            self.title.clone()
        } else {
            format!("{} - {}", self.title, self.artist)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Playlist {
    pub tracks: Vec<Track>,
}

impl Playlist {
    #[allow(dead_code)]
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading playlist from {}", path.display()))?;
        let tracks: Vec<Track> = serde_json::from_str(&raw)
            .with_context(|| format!("parsing playlist from {}", path.display()))?;
        Ok(Self { tracks })
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn get(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    pub fn next_index(&self, current: usize) -> usize {
        if self.tracks.is_empty() {
            0
        } else {
            (current + 1) % self.tracks.len()
        }
    }

    pub fn prev_index(&self, current: usize) -> usize {
        if self.tracks.is_empty() {
            0
        } else if current == 0 {
            self.tracks.len().saturating_sub(1)
        } else {
            current - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_navigation_cycles() {
        let p = Playlist {
            tracks: vec![
                Track {
                    id: "1".into(),
                    title: "Track 1".into(),
                    artist: "Artist 1".into(),
                    audio: "1.mp3".into(),
                    lyrics: "1.json".into(),
                },
                Track {
                    id: "2".into(),
                    title: "Track 2".into(),
                    artist: "Artist 2".into(),
                    audio: "2.mp3".into(),
                    lyrics: "2.json".into(),
                },
            ],
        };

        assert_eq!(p.next_index(0), 1);
        assert_eq!(p.next_index(1), 0);
        assert_eq!(p.prev_index(0), 1);
        assert_eq!(p.prev_index(1), 0);
        assert_eq!(p.get(0).unwrap().display_name(), "Track 1 - Artist 1");
    }
}
