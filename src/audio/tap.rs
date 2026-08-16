//! A pass through audio node that keeps a copy of what is playing.
//!
//! `SpectrumTap` wraps a rodio `Source`, forwards every sample untouched, and
//! writes a mono copy into a shared ring buffer. The analyser then reads that
//! buffer, which means the spectrum is computed from the audio the speakers
//! are producing right now rather than from a scan of the file done at
//! startup.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, SampleRate, Source};

/// Roughly 190ms at 44.1kHz, comfortably more than one FFT window so a frame
/// never reads a partly refilled buffer.
const RING_CAPACITY: usize = 8192;

/// Shared window of the most recent mono samples.
#[derive(Clone, Default)]
pub struct SampleRing(Arc<Mutex<VecDeque<f32>>>);

impl SampleRing {
    fn push(&self, s: f32) {
        let mut buf = self.0.lock().unwrap();
        if buf.len() == RING_CAPACITY {
            buf.pop_front();
        }
        buf.push_back(s);
    }

    /// The newest `n` samples in chronological order. Returns fewer than `n`
    /// only while the buffer is still filling.
    pub fn latest(&self, n: usize) -> Vec<f32> {
        let buf = self.0.lock().unwrap();
        buf.iter().rev().take(n).rev().copied().collect()
    }

    /// Drop everything. Called on seek so the analyser does not blend audio
    /// from two different parts of the song.
    pub fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

pub struct SpectrumTap<S> {
    inner: S,
    ring: SampleRing,
    channels: usize,
    acc: f32,
    n: usize,
}

impl<S: Source> SpectrumTap<S> {
    pub fn new(inner: S, ring: SampleRing) -> Self {
        let channels = inner.channels().get() as usize;
        Self { inner, ring, channels, acc: 0.0, n: 0 }
    }
}

impl<S: Source> Iterator for SpectrumTap<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let s = self.inner.next()?;

        // Average the channels before storing. Handing interleaved stereo to
        // an FFT makes it read as a signal at twice the sample rate, which
        // smears every frequency across the spectrum.
        self.acc += s;
        self.n += 1;
        if self.n >= self.channels {
            self.ring.push(self.acc / self.channels as f32);
            self.acc = 0.0;
            self.n = 0;
        }

        Some(s)
    }
}

impl<S: Source> Source for SpectrumTap<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)?;
        self.ring.clear();
        self.acc = 0.0;
        self.n = 0;
        Ok(())
    }
}
