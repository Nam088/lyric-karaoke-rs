//! The whole song's loudness, one number per slot, for the waveform timeline.
//!
//! Decoding four minutes of mp3 takes a moment, so it happens on a background
//! thread and the UI simply draws a plain bar until the result arrives. The
//! TypeScript build did the equivalent scan up front and blocked the screen
//! behind an "Analyzing Audio Waves..." message.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use rodio::{Decoder, Source};

/// Horizontal resolution of the stored envelope. Resampled down to whatever
/// the terminal is actually wide at draw time.
pub const SLOTS: usize = 1024;

/// Percentiles the strip is stretched between. Trimming the extremes stops a
/// single silent gap or one clipped transient from setting the whole scale.
const LOW_PERCENTILE: f32 = 0.05;
const HIGH_PERCENTILE: f32 = 0.95;

/// Below this the track is treated as having no dynamics at all.
const MIN_SPAN_DB: f32 = 3.0;

/// Height used for a track with nothing to plot.
const FLAT_LEVEL: f32 = 0.7;

/// Handle to a scan that may or may not have finished.
#[derive(Clone, Default)]
pub struct Envelope(Arc<Mutex<Option<Vec<f32>>>>);

impl Envelope {
    /// Start scanning in the background. Returns immediately.
    pub fn scan(path: PathBuf) -> Self {
        let slot: Arc<Mutex<Option<Vec<f32>>>> = Arc::default();
        let out = slot.clone();

        thread::spawn(move || {
            if let Some(data) = compute(&path) {
                *out.lock().unwrap() = Some(data);
            }
        });

        Self(slot)
    }

    /// The envelope resampled to `width` points, or `None` while the scan is
    /// still running.
    ///
    /// Averages rather than taking the peak of each span. The stored values
    /// are already stretched to fill the strip, so a maximum over a dozen of
    /// them puts almost every column at the ceiling and the shape disappears.
    pub fn resampled(&self, width: usize) -> Option<Vec<f32>> {
        let guard = self.0.lock().unwrap();
        let data = guard.as_ref()?;
        if width == 0 {
            return Some(Vec::new());
        }

        Some(
            (0..width)
                .map(|i| {
                    let lo = i * data.len() / width;
                    let hi = ((i + 1) * data.len() / width).max(lo + 1).min(data.len());
                    let span = &data[lo..hi];
                    span.iter().sum::<f32>() / span.len() as f32
                })
                .collect(),
        )
    }
}

fn compute(path: &PathBuf) -> Option<Vec<f32>> {
    // A bare File, deliberately. Symphonia buffers internally, and wrapping
    // the file in a BufReader hides its length, which makes total_duration
    // come back as None and the whole scan give up.
    let file = std::fs::File::open(path).ok()?;
    let source = Decoder::try_from(file).ok()?;

    let channels = source.channels().get() as usize;
    let rate = source.sample_rate().get() as usize;
    let total = source.total_duration()?;

    let frames = (total.as_secs_f64() * rate as f64) as usize;
    let per_slot = (frames / SLOTS).max(1);

    // RMS, not peak. A quarter second of a modern pop master hits full scale
    // almost everywhere, so a peak envelope is a solid block with no shape to
    // it. Average power tracks how loud a passage actually feels.
    let mut out = Vec::with_capacity(SLOTS);
    let mut sum_sq = 0.0f64;
    let mut counted = 0usize;
    let mut frame = 0usize;
    let mut chan = 0usize;

    for s in source {
        sum_sq += (s as f64) * (s as f64);
        counted += 1;

        chan += 1;
        if chan < channels {
            continue;
        }
        chan = 0;

        frame += 1;
        if frame >= per_slot {
            out.push((sum_sq / counted.max(1) as f64).sqrt() as f32);
            sum_sq = 0.0;
            counted = 0;
            frame = 0;
            if out.len() == SLOTS {
                break;
            }
        }
    }

    stretch(&mut out);

    (!out.is_empty()).then_some(out)
}

/// Stretch the envelope so its own quiet and loud passages span the strip.
///
/// A fixed decibel window does not work here. A modern master is compressed
/// into a handful of decibels, so any window wide enough for a live recording
/// flattens a pop track into a solid block. Taking the track's own 5th and
/// 95th percentile adapts to whatever it was mastered like.
fn stretch(values: &mut [f32]) {
    if values.is_empty() {
        return;
    }

    let db: Vec<f32> = values
        .iter()
        .map(|&v| 20.0 * v.max(1e-6).log10())
        .collect();

    let mut sorted = db.clone();
    sorted.sort_by(f32::total_cmp);
    let lo = sorted[((sorted.len() - 1) as f32 * LOW_PERCENTILE) as usize];
    let hi = sorted[((sorted.len() - 1) as f32 * HIGH_PERCENTILE) as usize];

    // A track with no dynamics worth plotting gets a uniform band. Stretching
    // a fraction of a decibel across the whole strip would turn dither noise
    // into a mountain range, and mapping it to zero would draw nothing at all.
    let span = hi - lo;
    if span < MIN_SPAN_DB {
        values.fill(FLAT_LEVEL);
        return;
    }

    for (v, d) in values.iter_mut().zip(db) {
        *v = ((d - lo) / span).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(data: Vec<f32>) -> Envelope {
        Envelope(Arc::new(Mutex::new(Some(data))))
    }

    #[test]
    fn an_unfinished_scan_reports_nothing() {
        assert!(Envelope::default().resampled(40).is_none());
    }

    #[test]
    fn resampling_averages_each_span() {
        let e = ready(vec![0.0, 1.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0]);
        assert_eq!(e.resampled(4).unwrap(), vec![0.5, 0.0, 0.5, 0.0]);
    }

    #[test]
    fn resampling_keeps_the_contrast_of_the_source() {
        // A quiet half and a loud half must not both come out at the top.
        let mut data = vec![0.2f32; 512];
        data.extend(std::iter::repeat_n(0.9f32, 512));
        let out = ready(data).resampled(8).unwrap();
        assert!(out[0] < 0.3, "quiet half came out at {}", out[0]);
        assert!(out[7] > 0.8, "loud half came out at {}", out[7]);
    }

    #[test]
    fn resampling_up_never_drops_a_slot() {
        let e = ready(vec![0.2, 0.8]);
        let out = e.resampled(6).unwrap();
        assert_eq!(out.len(), 6);
        assert!(out.iter().all(|&v| v == 0.2 || v == 0.8));
    }

    #[test]
    fn stretching_spans_the_full_strip() {
        // Quiet intro through to a loud chorus, about 16dB apart.
        let mut v: Vec<f32> = (0..100).map(|i| 0.15 + i as f32 * 0.0085).collect();
        stretch(&mut v);
        assert!(v.iter().fold(0.0f32, |a, &b| a.max(b)) > 0.9, "loud end too low");
        assert!(v.iter().fold(1.0f32, |a, &b| a.min(b)) < 0.1, "quiet end too high");
    }

    #[test]
    fn a_track_with_no_dynamics_draws_a_band_not_a_void() {
        let mut v: Vec<f32> = (0..100).map(|i| 0.80 + i as f32 * 0.0002).collect();
        stretch(&mut v);
        assert!(v.iter().all(|&x| x == FLAT_LEVEL), "expected a uniform band");
    }

    #[test]
    fn stretching_survives_a_flat_track_and_an_empty_one() {
        let mut flat = vec![0.5f32; 40];
        stretch(&mut flat);
        assert!(flat.iter().all(|v| v.is_finite()));

        let mut none: Vec<f32> = Vec::new();
        stretch(&mut none);
    }

    #[test]
    fn one_clipped_transient_does_not_set_the_scale() {
        // A quiet body with a single loud spike. Without percentile trimming
        // the spike would define the top of the range and flatten everything.
        let mut with_spike: Vec<f32> = (0..100).map(|i| 0.15 + i as f32 * 0.0085).collect();
        let mut without = with_spike.clone();
        with_spike[50] = 40.0;

        stretch(&mut with_spike);
        stretch(&mut without);

        for i in [0usize, 25, 75, 99] {
            assert!(
                (with_spike[i] - without[i]).abs() < 0.05,
                "slot {i}: {} with the spike vs {} without",
                with_spike[i],
                without[i]
            );
        }
    }

    #[test]
    fn zero_width_is_not_a_panic() {
        assert_eq!(ready(vec![1.0]).resampled(0).unwrap(), Vec::<f32>::new());
    }
}
