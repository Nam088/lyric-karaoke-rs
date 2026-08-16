//! Which note is sounding right now.
//!
//! Uses the harmonic product spectrum. A sung or played note puts energy at
//! its fundamental *and* at every multiple of it, and often the second
//! harmonic is louder than the first. Multiplying the spectrum by copies of
//! itself squeezed to 1/2, 1/3 and 1/4 makes all those harmonics line up on
//! the fundamental, which then wins by a wide margin. Taking the loudest bin
//! directly would frequently report the octave above.

/// How many squeezed copies to multiply in.
const HARMONICS: usize = 4;

/// Vocal range, generously bounded. Below this is bass and drums, above it is
/// mostly cymbals and consonants.
const F_MIN: f32 = 75.0;
const F_MAX: f32 = 1_200.0;

/// Below this the result is noise, so report nothing rather than a wrong note.
const CONFIDENCE_FLOOR: f32 = 2.0e-5;

const NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub hz: f32,
    /// Semitones above C, 0 to 11.
    pub pitch_class: usize,
    /// Scientific pitch notation octave, where A440 is in octave 4.
    pub octave: i32,
    /// How far off the exact note, in cents. Negative is flat.
    pub cents: f32,
}

impl Note {
    pub fn name(&self) -> String {
        format!("{}{}", NAMES[self.pitch_class], self.octave)
    }
}

/// Find the fundamental in a magnitude spectrum, or `None` if nothing in the
/// vocal range stands out.
pub fn detect(mag: &[f32], sample_rate: f32, fft_size: usize) -> Option<Note> {
    let bins = mag.len();
    let hz_per_bin = sample_rate / fft_size as f32;

    let lo = ((F_MIN / hz_per_bin) as usize).max(1);
    let hi = ((F_MAX / hz_per_bin) as usize).min(bins / HARMONICS);
    if lo >= hi {
        return None;
    }

    let mut best_bin = lo;
    let mut best = 0.0f32;
    for b in lo..hi {
        let mut product = mag[b];
        for h in 2..=HARMONICS {
            product *= harmonic_energy(mag, b, h);
        }
        if product > best {
            best = product;
            best_bin = b;
        }
    }

    if best < CONFIDENCE_FLOOR {
        return None;
    }

    Some(from_hz(refine(mag, best_bin) * hz_per_bin))
}

/// Energy of the `h`th harmonic of the tone sitting in bin `b`.
///
/// Bin `b` covers a range of frequencies, so a fundamental anywhere inside it
/// puts its `h`th harmonic anywhere within `h/2` bins of `b * h`. Reading
/// `mag[b * h]` alone misses it whenever the fundamental is not centred, and
/// since the harmonics are multiplied together, one miss zeroes the whole
/// candidate.
fn harmonic_energy(mag: &[f32], b: usize, h: usize) -> f32 {
    let centre = b * h;
    let radius = h.div_ceil(2);
    let lo = centre.saturating_sub(radius);
    let hi = (centre + radius + 1).min(mag.len());
    if lo >= hi {
        return 0.0;
    }
    mag[lo..hi].iter().fold(0.0f32, |a, &v| a.max(v))
}

/// Interpolate the true peak between bins by fitting a parabola through the
/// winner and its two neighbours. One bin is about 21Hz at 44.1kHz, which is
/// several semitones down low, so this matters.
fn refine(mag: &[f32], bin: usize) -> f32 {
    if bin == 0 || bin + 1 >= mag.len() {
        return bin as f32;
    }
    let (a, b, c) = (mag[bin - 1], mag[bin], mag[bin + 1]);
    let denom = a - 2.0 * b + c;
    if denom.abs() < 1e-12 {
        return bin as f32;
    }
    bin as f32 + 0.5 * (a - c) / denom
}

/// Convert a frequency to the nearest note, plus how far off it is.
pub fn from_hz(hz: f32) -> Note {
    // Semitones away from A440, which is MIDI note 69.
    let midi_exact = 69.0 + 12.0 * (hz / 440.0).log2();
    let midi = midi_exact.round();
    let cents = (midi_exact - midi) * 100.0;

    let midi_i = midi as i32;
    Note {
        hz,
        pitch_class: midi_i.rem_euclid(12) as usize,
        octave: midi_i / 12 - 1,
        cents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_frequencies_map_to_the_right_names() {
        assert_eq!(from_hz(440.0).name(), "A4");
        assert_eq!(from_hz(261.626).name(), "C4");
        assert_eq!(from_hz(880.0).name(), "A5");
        assert_eq!(from_hz(82.41).name(), "E2");
    }

    #[test]
    fn reports_how_far_out_of_tune_a_pitch_is() {
        // A quarter tone sharp of A4 is +50 cents.
        let sharp = from_hz(440.0 * 2f32.powf(0.5 / 12.0));
        assert!((sharp.cents.abs() - 50.0).abs() < 1.0, "got {}", sharp.cents);
        assert!(from_hz(440.0).cents.abs() < 0.1);
    }

    /// A harmonic stack: fundamental plus overtones, with the second louder
    /// than the first. Picking the largest bin would answer one octave high.
    fn harmonic_spectrum(f0: f32, rate: f32, fft: usize) -> Vec<f32> {
        let hz_per_bin = rate / fft as f32;
        let mut mag = vec![1e-7; fft / 2];
        for (h, amp) in [(1, 0.4), (2, 1.0), (3, 0.7), (4, 0.5), (5, 0.2)] {
            let bin = (f0 * h as f32 / hz_per_bin).round() as usize;
            if bin < mag.len() {
                mag[bin] = amp;
            }
        }
        mag
    }

    #[test]
    fn finds_the_fundamental_not_the_loudest_harmonic() {
        let (rate, fft) = (44_100.0, 2048);
        let mag = harmonic_spectrum(220.0, rate, fft);

        let loudest = mag
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(loudest, 20, "test fixture should peak on the 2nd harmonic");

        let note = detect(&mag, rate, fft).expect("should find a pitch");
        assert_eq!(note.name(), "A3", "detected {}Hz", note.hz);
    }

    #[test]
    fn returns_nothing_for_silence() {
        assert!(detect(&vec![0.0; 1024], 44_100.0, 2048).is_none());
        assert!(detect(&vec![1e-9; 1024], 44_100.0, 2048).is_none());
    }
}
