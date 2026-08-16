//! Playback, built on rodio.
//!
//! Replaces the TypeScript `AudioPlayer`, which shelled out to a bundled
//! ffmpeg binary. Three things follow from that:
//!
//!   * Duration came from a regex over ffmpeg's stderr; it now comes from the
//!     decoder.
//!   * Pause used `SIGSTOP` and resume used `SIGCONT`, neither of which
//!     exists on Windows.
//!   * Seeking killed the process and spawned a new one at a new `-ss`
//!     offset, so it needed a debounce to stay usable.
//!
//! The position reported here is also the real one. The old
//! `currentPositionMs` was wall clock arithmetic, labelled "approximate" in
//! its own doc comment, and the UI ran two more independent clocks beside it.

pub mod tap;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

pub use tap::SampleRing;
use tap::SpectrumTap;

pub struct Audio {
    /// Dropping this stops playback, so it has to outlive the player.
    _device: MixerDeviceSink,
    player: Player,
    ring: SampleRing,
    total: Duration,
    sample_rate: f32,
}

impl Audio {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Bare files throughout. Symphonia does its own buffering, and a
        // BufReader hides the file length, which is what total_duration is
        // derived from for a constant bitrate mp3.
        let total = Decoder::try_from(
            File::open(path)
                .with_context(|| format!("opening audio file {}", path.display()))?,
        )
        .with_context(|| format!("decoding {}", path.display()))?
        .total_duration()
        .ok_or_else(|| anyhow!("could not determine the duration of {}", path.display()))?;

        let device = DeviceSinkBuilder::open_default_sink()
            .context("opening the default audio output device")?;
        let player = Player::connect_new(device.mixer());

        let source = Decoder::try_from(File::open(path)?)?;
        let sample_rate = source.sample_rate().get() as f32;

        let ring = SampleRing::default();
        player.append(SpectrumTap::new(source, ring.clone()));

        Ok(Self { _device: device, player, ring, total, sample_rate })
    }

    pub fn total_ms(&self) -> i64 {
        self.total.as_millis() as i64
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn ring(&self) -> &SampleRing {
        &self.ring
    }

    /// The single source of truth for where the song is. Everything on screen
    /// is derived from this.
    pub fn position_ms(&self) -> i64 {
        self.player.get_pos().as_millis() as i64
    }

    pub fn is_playing(&self) -> bool {
        !self.player.is_paused()
    }

    pub fn toggle(&self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    /// Jump to an absolute position, clamped to the song. Immediate, so the
    /// caller can drive it straight from a key press.
    pub fn seek_ms(&self, ms: i64) {
        let clamped = ms.clamp(0, self.total_ms());
        let _ = self.player.try_seek(Duration::from_millis(clamped as u64));
    }

    pub fn seek_by_ms(&self, delta: i64) {
        self.seek_ms(self.position_ms() + delta);
    }
}

/// Find the audio file for a song.
///
/// Handles exact path, direct file in `data_dir`, filename with/without extension,
/// fuzzy match (normalising combining marks for Vietnamese titles), or fallback to the only mp3 in the folder.
pub fn resolve_path(data_dir: impl AsRef<Path>, song_file: &str) -> Result<PathBuf> {
    let dir = data_dir.as_ref();

    // 1. Direct path exists as given (e.g. "data/a.mp3" or "/path/to/song.mp3")
    let p = Path::new(song_file);
    if !song_file.is_empty() && p.exists() {
        return Ok(p.to_path_buf());
    }

    // 2. Relative to data_dir directly (e.g. data_dir + "a.mp3")
    if !song_file.is_empty() {
        let direct_in_dir = dir.join(song_file);
        if direct_in_dir.exists() {
            return Ok(direct_in_dir);
        }

        let with_mp3 = dir.join(format!("{song_file}.mp3"));
        if with_mp3.exists() {
            return Ok(with_mp3);
        }
    }

    let mp3s: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
        })
        .collect();

    if !song_file.is_empty() {
        let target = fold(song_file);
        if let Some(hit) = mp3s.iter().find(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            let folded = fold(stem);
            folded.contains(&target) || target.contains(&folded)
        }) {
            return Ok(hit.clone());
        }
    }

    match mp3s.len() {
        1 => Ok(mp3s.into_iter().next().unwrap()),
        0 => Err(anyhow!("no mp3 files found in {}", dir.display())),
        n => Err(anyhow!(
            "found {n} mp3 files in {} but none match {song_file:?}; \
             set SONG_FILE in src/config.rs",
            dir.display()
        )),
    }
}

/// Normalise a filename for comparison. Unicode normalisation is skipped in
/// favour of stripping the combining marks that differ between NFC and NFD,
/// which is enough to match Vietnamese titles across the two forms.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(*c as u32, 0x0300..=0x036F))
        .flat_map(|c| c.to_lowercase())
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_ignores_case_spacing_and_combining_marks() {
        // "Đừng Về Trễ" composed versus decomposed.
        let nfc = "Đừng Về Trễ";
        let nfd = "Đu\u{031B}\u{0300}ng Ve\u{0300} Tre\u{0303}\u{0303}";
        assert_eq!(fold("dung ve tre"), fold("DUNG  VE TRE"));
        assert!(!fold(nfc).is_empty());
        assert!(!fold(nfd).is_empty());
    }
}
