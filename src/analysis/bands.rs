//! Fractional octave bands, per IEC 61260.
//!
//! This is what a real time analyser divides the spectrum into, and it is not
//! an arbitrary choice. A 1/N octave band is `N`th of an octave wide, so its
//! bandwidth grows in proportion to its centre frequency. Pink noise carries
//! equal energy per octave, so it reads flat across bands like these, and
//! music, which is roughly pink, reads roughly flat too.
//!
//! That is the whole reason for using them. Buckets of equal width in hertz
//! need a tilt bolted on to look right; these do not.

/// The base two system. IEC 61260 also defines a base ten system with
/// `G = 10^(3/10)`; the two agree to within a fraction of a percent and base
/// two is the one audio tools use.
const G: f32 = 2.0;

/// Reference frequency. Every standard band centre is derived from it.
const F_REF: f32 = 1000.0;

/// Fractions the standard names. Anything else is not a band you would find
/// on another analyser.
pub const STANDARD_FRACTIONS: [u32; 5] = [1, 3, 6, 12, 24];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub center: f32,
    pub low: f32,
    pub high: f32,
}

/// Centre frequency of band index `x` in the 1/`fraction` octave system.
///
/// IEC 61260 splits this by parity: odd fractions put a band centre exactly on
/// the reference, even fractions straddle it. Getting this wrong shifts every
/// centre by half a band.
fn center(fraction: u32, x: i32) -> f32 {
    let n = fraction as f32;
    let exponent = if fraction % 2 == 1 {
        x as f32 / n
    } else {
        (2 * x + 1) as f32 / (2.0 * n)
    };
    F_REF * G.powf(exponent)
}

/// Every standard band whose centre falls between `f_min` and `f_max`.
pub fn bands(fraction: u32, f_min: f32, f_max: f32) -> Vec<Band> {
    assert!(fraction > 0, "an octave cannot be split into zero parts");

    let half = G.powf(1.0 / (2.0 * fraction as f32));
    let span = (f_max / f_min).log2();
    // Generous bounds; the filter below decides what actually belongs.
    let reach = (span * fraction as f32).ceil() as i32 + fraction as i32 * 12;

    (-reach..=reach)
        .map(|x| center(fraction, x))
        .filter(|&c| c >= f_min && c <= f_max)
        .map(|c| Band { center: c, low: c / half, high: c * half })
        .collect()
}

/// How many bands the given fraction produces over a range.
pub fn count(fraction: u32, f_min: f32, f_max: f32) -> usize {
    bands(fraction, f_min, f_max).len()
}

/// The standard fraction that comes closest to filling `columns`.
///
/// The analysis stays on standard band centres whatever the terminal width is;
/// only how many of them there are adapts.
pub fn best_fraction(columns: usize, f_min: f32, f_max: f32) -> u32 {
    STANDARD_FRACTIONS
        .into_iter()
        .min_by_key(|&f| count(f, f_min, f_max).abs_diff(columns))
        .unwrap_or(12)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Centres every audio engineer would recognise, from the 1/3 octave
    /// series printed on the front of any analyser.
    #[test]
    fn third_octave_centres_match_the_published_series() {
        let expected = [
            31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ];
        let got = bands(3, 20.0, 20_000.0);

        for want in expected {
            let hit = got
                .iter()
                .any(|b| (b.center - want).abs() / want < 0.03);
            assert!(hit, "no band near {want}Hz in {:?}", got.iter().map(|b| b.center).collect::<Vec<_>>());
        }
    }

    #[test]
    fn a_band_centre_sits_on_the_reference_for_odd_fractions() {
        for fraction in [1u32, 3] {
            let got = bands(fraction, 20.0, 20_000.0);
            assert!(
                got.iter().any(|b| (b.center - 1000.0).abs() < 0.5),
                "1/{fraction} octave should have a band on 1kHz"
            );
        }
    }

    #[test]
    fn even_fractions_straddle_the_reference_instead() {
        // The standard defines it this way. A band centred exactly on 1kHz in
        // a 1/12 octave system would put every other centre half a band out.
        let got = bands(12, 900.0, 1100.0);
        assert!(
            !got.iter().any(|b| (b.center - 1000.0).abs() < 0.5),
            "1/12 octave should straddle 1kHz, got {:?}",
            got.iter().map(|b| b.center).collect::<Vec<_>>()
        );
        // but there is a band immediately either side
        assert!(got.iter().any(|b| b.center < 1000.0 && b.center > 960.0));
        assert!(got.iter().any(|b| b.center > 1000.0 && b.center < 1040.0));
    }

    #[test]
    fn each_band_is_one_fraction_of_an_octave_wide() {
        for fraction in STANDARD_FRACTIONS {
            for b in bands(fraction, 40.0, 16_000.0) {
                let octaves = (b.high / b.low).log2();
                let want = 1.0 / fraction as f32;
                assert!(
                    (octaves - want).abs() < 1e-4,
                    "1/{fraction} octave band spans {octaves} octaves"
                );
            }
        }
    }

    #[test]
    fn neighbouring_bands_meet_without_a_gap_or_an_overlap() {
        let got = bands(12, 40.0, 16_000.0);
        for pair in got.windows(2) {
            let ratio = pair[1].low / pair[0].high;
            assert!((ratio - 1.0).abs() < 1e-4, "gap of {ratio}x between bands");
        }
    }

    #[test]
    fn finer_fractions_give_proportionally_more_bands() {
        let third = count(3, 40.0, 16_000.0);
        let twelfth = count(12, 40.0, 16_000.0);
        assert!(
            (twelfth as f32 / third as f32 - 4.0).abs() < 0.15,
            "1/12 octave gave {twelfth} bands against {third} at 1/3"
        );
    }

    #[test]
    fn the_chosen_fraction_is_always_one_the_standard_names() {
        for columns in [10usize, 40, 90, 132, 200, 400] {
            let f = best_fraction(columns, 40.0, 16_000.0);
            assert!(STANDARD_FRACTIONS.contains(&f), "picked 1/{f} octave");
        }
    }

    #[test]
    fn a_wider_display_earns_a_finer_fraction() {
        let narrow = best_fraction(30, 40.0, 16_000.0);
        let wide = best_fraction(200, 40.0, 16_000.0);
        assert!(wide >= narrow, "1/{wide} should be finer than 1/{narrow}");
    }
}
