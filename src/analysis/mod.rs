//! Turning the live audio into numbers the UI can draw.
//!
//! Everything here is measured from the signal. The TypeScript build had to
//! invent motion (`staticJaggedness = Math.sin(i * 13.73 + 4.1)` and a
//! `turbulence` term) because it only had three filtered bands to work with
//! and they all moved together. A real transform produces its own detail.

pub mod bands;
pub mod envelope;
pub mod pitch;

use rustfft::{num_complex::Complex, FftPlanner};

/// 4096 at 44.1kHz is a 93ms window and 10.8Hz per bin, which is what the low
/// bands need. At 2048 the bottom two octaves land inside a single bin.
pub const FFT_SIZE: usize = 4096;

/// Lowest and highest frequency drawn. Above 16kHz an mp3 has been lowpassed
/// away by the encoder, so there is nothing there to show.
const F_MIN: f32 = 40.0;
const F_MAX: f32 = 16_000.0;

/// Anything this far below full scale is silence as far as the display cares.
const DB_FLOOR: f32 = -78.0;

/// Display shaping only. Spreads the midrange out so the picture uses the full
/// height of a canvas that is just sixteen dots tall. It is applied after the
/// measurement, never to it.
const GAMMA: f32 = 1.35;

/// Noise power bandwidth of a Hann window, in bins. Summing power across a
/// band overcounts by this much, so it is divided back out.
const HANN_NPBW: f32 = 1.5;

pub struct Analyzer {
    planner: FftPlanner<f32>,
    window: Vec<f32>,
    /// Standard bands the spectrum is measured in. Rebuilt when the terminal
    /// resizes, never per frame.
    bands: Vec<bands::Band>,
    /// Which fraction of an octave those bands divide.
    fraction: u32,
    mag: Vec<f32>,
    agc: f32,

    /// Smoothed column heights, 0 to 1, one per canvas dot column.
    pub levels: Vec<f32>,
    /// Peak markers that hold then fall, 0 to 1.
    pub peaks: Vec<f32>,
    /// Pitch of the loudest voiced tone, if there is one.
    pub note: Option<pitch::Note>,
}

impl Analyzer {
    pub fn new(columns: usize) -> Self {
        let window = (0..FFT_SIZE)
            .map(|i| {
                // Hann. Without a window, a tone that does not land exactly on
                // a bin leaks across the whole spectrum.
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos())
            })
            .collect();

        let fraction = bands::best_fraction(columns, F_MIN, F_MAX);

        Self {
            planner: FftPlanner::new(),
            window,
            bands: bands::bands(fraction, F_MIN, F_MAX),
            fraction,
            mag: vec![0.0; FFT_SIZE / 2],
            agc: 0.35,
            levels: vec![0.0; columns],
            peaks: vec![0.0; columns],
            note: None,
        }
    }

    /// Resize when the terminal does. Levels are kept where they overlap so
    /// the display does not blink back to zero.
    pub fn resize(&mut self, columns: usize) {
        if columns == self.levels.len() {
            return;
        }
        self.levels.resize(columns, 0.0);
        self.peaks.resize(columns, 0.0);

        let fraction = bands::best_fraction(columns, F_MIN, F_MAX);
        if fraction != self.fraction {
            self.fraction = fraction;
            self.bands = bands::bands(fraction, F_MIN, F_MAX);
        }
    }

    /// Average energy in the lowest frequency bands (Bass/Kick), normalized 0.0 to 1.0.
    pub fn bass_energy(&self) -> f32 {
        if self.levels.is_empty() {
            return 0.0;
        }
        let bass_count = (self.levels.len() / 8).max(2).min(self.levels.len());
        let sum: f32 = self.levels[..bass_count].iter().sum();
        (sum / bass_count as f32).clamp(0.0, 1.0)
    }

    /// Advance one frame. `dt` is seconds since the previous call, so the
    /// smoothing behaves the same whether the terminal is keeping up or not.
    pub fn feed(&mut self, samples: &[f32], sample_rate: f32, dt: f32) {
        if samples.len() < FFT_SIZE {
            // Still filling after a seek. Let everything fall rather than
            // freezing mid air.
            self.decay(dt);
            return;
        }

        let mut buf: Vec<Complex<f32>> = samples[samples.len() - FFT_SIZE..]
            .iter()
            .zip(&self.window)
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();
        self.planner.plan_fft_forward(FFT_SIZE).process(&mut buf);

        let bins = FFT_SIZE / 2;
        // A full scale sine through a Hann window peaks at N/4. Dividing by
        // that puts the magnitudes back on a 0 dBFS reference.
        let scale = FFT_SIZE as f32 / 4.0;

        for (m, c) in self.mag.iter_mut().zip(&buf[..bins]) {
            *m = c.norm() / scale;
        }

        self.note = pitch::detect(&self.mag, sample_rate, FFT_SIZE);
        self.update_levels(sample_rate, dt);
    }

    fn update_levels(&mut self, sample_rate: f32, dt: f32) {
        let cols = self.levels.len();
        if cols == 0 {
            return;
        }

        let hz_per_bin = sample_rate / FFT_SIZE as f32;

        // Level of every standard band, in dB relative to full scale.
        let band_db: Vec<f32> = self
            .bands
            .iter()
            .map(|b| band_level_db(&self.mag, *b, hz_per_bin))
            .collect();

        // Bands are the measurement, columns are the display. Stretch one onto
        // the other rather than bending the analysis to fit the terminal.
        let raw: Vec<f32> = (0..cols)
            .map(|x| {
                let db = sample(&band_db, x as f32 / (cols.max(2) - 1) as f32);
                ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0)
            })
            .collect();

        // Automatic gain. Rises quickly so a chorus does not clip off the top,
        // falls slowly so a quiet passage opens up gradually instead of
        // pumping between frames.
        let frame_max = raw.iter().fold(0.0f32, |a, &b| a.max(b));
        let rate = if frame_max > self.agc { 6.0 } else { 0.35 };
        self.agc += (frame_max - self.agc) * (rate * dt).min(1.0);
        let gain = 1.0 / self.agc.max(0.25);

        for ((level, peak), &r) in self.levels.iter_mut().zip(&mut self.peaks).zip(&raw) {
            let target = (r * gain).clamp(0.0, 1.0).powf(GAMMA);
            let rate = if target > *level { 14.0 } else { 5.0 };
            *level += (target - *level) * (rate * dt).min(1.0);
            *peak = (*peak - dt * 0.5).max(*level);
        }
    }

    fn decay(&mut self, dt: f32) {
        for (level, peak) in self.levels.iter_mut().zip(&mut self.peaks) {
            *level = (*level - dt * 2.0).max(0.0);
            *peak = (*peak - dt * 0.5).max(*level);
        }
        self.note = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pure tone at `hz`, long enough to fill one FFT window.
    fn tone(hz: f32, rate: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / rate).sin())
            .collect()
    }

    #[test]
    fn a_low_tone_lights_the_left_and_a_high_tone_the_right() {
        let rate = 44_100.0;

        let mut low = Analyzer::new(64);
        let mut high = Analyzer::new(64);
        for _ in 0..40 {
            low.feed(&tone(80.0, rate, FFT_SIZE), rate, 0.033);
            high.feed(&tone(8_000.0, rate, FFT_SIZE), rate, 0.033);
        }

        let peak = |a: &Analyzer| {
            a.levels
                .iter()
                .enumerate()
                .max_by(|x, y| x.1.total_cmp(y.1))
                .map(|(i, _)| i)
                .unwrap()
        };

        assert!(peak(&low) < 16, "80Hz landed at column {}", peak(&low));
        assert!(peak(&high) > 48, "8kHz landed at column {}", peak(&high));
    }

    #[test]
    fn silence_falls_to_zero() {
        let rate = 44_100.0;
        let mut a = Analyzer::new(32);
        for _ in 0..30 {
            a.feed(&tone(440.0, rate, FFT_SIZE), rate, 0.033);
        }
        assert!(a.levels.iter().any(|&v| v > 0.1));

        for _ in 0..120 {
            a.feed(&vec![0.0; FFT_SIZE], rate, 0.033);
        }
        assert!(
            a.levels.iter().all(|&v| v < 0.05),
            "still lit: {:?}",
            a.levels.iter().fold(0.0f32, |m, &v| m.max(v))
        );
    }

    #[test]
    fn agc_lifts_a_quiet_signal_to_the_same_height_as_a_loud_one() {
        let rate = 44_100.0;
        let peak_of = |amp: f32| {
            let mut a = Analyzer::new(48);
            let sig: Vec<f32> = tone(440.0, rate, FFT_SIZE).iter().map(|s| s * amp).collect();
            for _ in 0..200 {
                a.feed(&sig, rate, 0.033);
            }
            a.levels.iter().fold(0.0f32, |m, &v| m.max(v))
        };

        let loud = peak_of(1.0);
        let quiet = peak_of(0.02);
        assert!(loud > 0.85, "loud only reached {loud}");
        assert!(quiet > 0.85, "quiet only reached {quiet}");
    }

    #[test]
    fn resizing_keeps_the_overlapping_columns() {
        let mut a = Analyzer::new(8);
        a.levels = vec![0.5; 8];
        a.resize(12);
        assert_eq!(a.levels.len(), 12);
        assert_eq!(a.levels[0], 0.5);
        assert_eq!(a.levels[11], 0.0);
        a.resize(4);
        assert_eq!(a.levels.len(), 4);
    }

    #[test]
    fn a_short_buffer_decays_instead_of_panicking() {
        let mut a = Analyzer::new(16);
        a.levels = vec![1.0; 16];
        a.feed(&[0.0; 10], 44_100.0, 0.033);
        assert!(a.levels[0] < 1.0);
    }
}

/// Energy in one band, in dB relative to full scale.
///
/// A band level is the *sum* of the power in it, not the loudest bin. That
/// distinction is the whole point: a band an octave up is twice as wide, so it
/// collects twice the power from a signal of equal density. Reading the peak
/// bin instead reports the same number for both, which is why an analyser
/// built that way needs a tilt invented to make music look level.
///
/// Bins at the edges count for the fraction of themselves that falls inside
/// the band. Rounding the edges to whole bins instead quantises the narrow low
/// frequency bands badly enough to flatten the three decibel per octave slope
/// that white noise is supposed to show.
fn band_level_db(mag: &[f32], band: bands::Band, hz_per_bin: f32) -> f32 {
    let bins = mag.len();

    // Bin k spans [(k - 0.5)*df, (k + 0.5)*df].
    let first = ((band.low / hz_per_bin - 0.5).floor().max(0.0)) as usize;
    let last = ((band.high / hz_per_bin + 0.5).ceil() as usize).min(bins);

    let power: f32 = mag[first..last]
        .iter()
        .enumerate()
        .map(|(offset, &m)| {
            let k = (first + offset) as f32;
            let bin_lo = (k - 0.5) * hz_per_bin;
            let bin_hi = (k + 0.5) * hz_per_bin;
            let overlap = band.high.min(bin_hi) - band.low.max(bin_lo);
            if overlap > 0.0 {
                m * m * (overlap / hz_per_bin)
            } else {
                0.0
            }
        })
        .sum();

    // A Hann window spreads one tone across 1.5 bins, so a plain sum counts it
    // one and a half times over.
    10.0 * (power / HANN_NPBW + 1e-12).log10()
}

/// Read a value part way along a series, interpolating between neighbours.
fn sample(values: &[f32], position: f32) -> f32 {
    match values.len() {
        0 => DB_FLOOR,
        1 => values[0],
        n => {
            let pos = position.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = pos.floor() as usize;
            let t = pos - i as f32;
            if i + 1 < n {
                values[i] * (1.0 - t) + values[i + 1] * t
            } else {
                values[n - 1]
            }
        }
    }
}

#[cfg(test)]
mod band_tests {
    use super::*;

    const RATE: f32 = 44_100.0;

    fn hz_per_bin() -> f32 {
        RATE / FFT_SIZE as f32
    }

    /// A magnitude spectrum whose power follows `power(f)`.
    fn spectrum(power: impl Fn(f32) -> f32) -> Vec<f32> {
        (0..FFT_SIZE / 2)
            .map(|k| {
                let f = k as f32 * hz_per_bin();
                if f <= 0.0 { 0.0 } else { power(f).sqrt() }
            })
            .collect()
    }

    fn levels(mag: &[f32], fraction: u32) -> Vec<(f32, f32)> {
        bands::bands(fraction, F_MIN, F_MAX)
            .into_iter()
            .map(|b| (b.center, band_level_db(mag, b, hz_per_bin())))
            .collect()
    }

    /// The defining property of fractional octave bands, and the reason every
    /// analyser uses them: pink noise carries equal energy per octave, and a
    /// band an octave up is twice as wide, so the trace comes out level.
    ///
    /// The previous implementation read the loudest bin in each band, which
    /// gave a falling trace for pink noise and needed an invented tilt to hide
    /// it.
    #[test]
    fn pink_noise_reads_flat() {
        let pink = spectrum(|f| 1.0 / f);
        let got = levels(&pink, 12);

        // Ignore the bottom octave, where the transform cannot resolve a
        // twelfth of an octave and the level is interpolated.
        let usable: Vec<f32> = got
            .iter()
            .filter(|(c, _)| *c > 120.0)
            .map(|(_, db)| *db)
            .collect();

        let mean = usable.iter().sum::<f32>() / usable.len() as f32;
        for db in &usable {
            assert!(
                (db - mean).abs() < 1.0,
                "pink noise deviated {:.2}dB from flat",
                db - mean
            );
        }
    }

    /// White noise has constant power per hertz, so a band twice as wide
    /// collects twice the energy: three decibels per octave, rising.
    #[test]
    fn white_noise_rises_three_decibels_per_octave() {
        let white = spectrum(|_| 1.0);
        let got = levels(&white, 12);

        let at = |target: f32| {
            got.iter()
                .min_by(|a, b| {
                    (a.0 - target).abs().total_cmp(&(b.0 - target).abs())
                })
                .unwrap()
                .1
        };

        for (lo, hi) in [(250.0, 500.0), (500.0, 1000.0), (1000.0, 2000.0), (2000.0, 4000.0)] {
            let step = at(hi) - at(lo);
            assert!(
                (step - 3.01).abs() < 0.4,
                "{lo}Hz to {hi}Hz rose {step:.2}dB, expected 3.01dB"
            );
        }
    }

    /// A full scale sine should read 0 dBFS in its band. Without the window's
    /// noise power bandwidth divided back out it reads high by 10*log10(1.5),
    /// about 1.8dB.
    ///
    /// The tone goes on the band centre. On a band edge its energy genuinely
    /// splits across the two neighbours, which is correct behaviour and not
    /// what this is measuring.
    #[test]
    fn a_full_scale_tone_reads_zero_dbfs() {
        let band = bands::bands(12, F_MIN, F_MAX)
            .into_iter()
            .min_by(|x, y| (x.center - 1000.0).abs().total_cmp(&(y.center - 1000.0).abs()))
            .unwrap();

        let signal: Vec<f32> = (0..FFT_SIZE)
            .map(|i| (2.0 * std::f32::consts::PI * band.center * i as f32 / RATE).sin())
            .collect();

        let mut a = Analyzer::new(64);
        a.feed(&signal, RATE, 0.033);

        let db = band_level_db(&a.mag, band, hz_per_bin());
        assert!(db.abs() < 1.0, "a full scale tone read {db:.2}dBFS");
    }

    /// Energy is not lost at a band edge, it is shared. A tone sitting on the
    /// boundary should still add up to full scale across the pair.
    #[test]
    fn a_tone_on_a_boundary_is_shared_not_dropped() {
        let all = bands::bands(12, F_MIN, F_MAX);
        let i = all
            .iter()
            .position(|b| b.center > 1000.0)
            .expect("a band above 1kHz");
        let edge = all[i].low;

        let signal: Vec<f32> = (0..FFT_SIZE)
            .map(|n| (2.0 * std::f32::consts::PI * edge * n as f32 / RATE).sin())
            .collect();

        let mut a = Analyzer::new(64);
        a.feed(&signal, RATE, 0.033);

        let power: f32 = [all[i - 1], all[i]]
            .iter()
            .map(|b| 10f32.powf(band_level_db(&a.mag, *b, hz_per_bin()) / 10.0))
            .sum();

        let db = 10.0 * power.log10();
        assert!(db.abs() < 1.0, "the pair totalled {db:.2}dBFS");
    }

    /// The narrow band path, used where the transform cannot resolve a whole
    /// band, has to hand back levels on the same scale as the summing path or
    /// there is a visible step in the middle of the spectrum.
    #[test]
    fn the_two_paths_agree_where_they_meet() {
        let pink = spectrum(|f| 1.0 / f);
        let per_bin = hz_per_bin();

        let mut narrow = Vec::new();
        let mut wide = Vec::new();
        for b in bands::bands(12, F_MIN, F_MAX) {
            let db = band_level_db(&pink, b, per_bin);
            if (b.high - b.low) < per_bin {
                narrow.push(db);
            } else if b.center < 600.0 {
                wide.push(db);
            }
        }

        assert!(!narrow.is_empty() && !wide.is_empty(), "both paths should be exercised");
        let avg = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
        assert!(
            (avg(&narrow) - avg(&wide)).abs() < 2.0,
            "interpolated bands sit {:.2}dB away from summed ones",
            avg(&narrow) - avg(&wide)
        );
    }

    #[test]
    fn silence_sits_on_the_floor() {
        let quiet = vec![0.0f32; FFT_SIZE / 2];
        for (_, db) in levels(&quiet, 12) {
            assert!(db < DB_FLOOR, "silence read {db}dB");
        }
    }
}

