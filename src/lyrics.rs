//! Lyric loading and gap injection.
//!
//! Ported from `types.ts` plus `processLyricsGaps` in `app.tsx`. The one
//! behavioural change: the TypeScript version read the file during module
//! import and, on failure, logged to the console and carried on with an empty
//! array, so a typo in a path produced a blank screen rather than an error.
//! Here loading is an explicit call that returns a `Result`.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config;

#[derive(Debug, Clone, Deserialize)]
pub struct Word {
    #[serde(rename = "startTime")]
    pub start_time: i64,
    #[serde(rename = "endTime")]
    pub end_time: i64,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sentence {
    pub words: Vec<Word>,
    /// Injected instrumental break rather than a real lyric line.
    #[serde(default)]
    pub is_gap: bool,
}

impl Sentence {
    pub fn start(&self) -> i64 {
        self.words.first().map_or(0, |w| w.start_time)
    }

    pub fn end(&self) -> i64 {
        self.words.last().map_or(0, |w| w.end_time)
    }

    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.data.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Deserialize)]
struct LyricsData {
    sentences: Vec<Sentence>,
}

#[derive(Debug, Deserialize)]
struct RootLyricJson {
    data: LyricsData,
}

/// Read `lr.json` and inject a musical note line into every instrumental gap.
pub fn load(path: impl AsRef<Path>) -> Result<Vec<Sentence>> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading lyrics from {}", path.display()))?;
    let root: RootLyricJson = serde_json::from_str(&raw)
        .with_context(|| format!("parsing lyrics from {}", path.display()))?;

    let mut sentences = root.data.sentences;
    for s in &mut sentences {
        for w in &mut s.words {
            w.data = w.data.trim().to_string();
        }
    }
    sentences.retain(|s| !s.words.is_empty());

    Ok(inject_gaps(sentences))
}

fn gap_sentence(start: i64, end: i64) -> Sentence {
    Sentence {
        words: vec![Word {
            start_time: start,
            end_time: end,
            data: config::GAP_TEXT.to_string(),
        }],
        is_gap: true,
    }
}

/// Insert a gap line before the first lyric and between any two lines far
/// enough apart, so the display has something to show during instrumentals.
fn inject_gaps(input: Vec<Sentence>) -> Vec<Sentence> {
    if input.is_empty() {
        return input;
    }

    let mut out = Vec::with_capacity(input.len() + 8);

    let first_start = input[0].start();
    if first_start > config::GAP_INITIAL_THRESHOLD_MS {
        out.push(gap_sentence(0, first_start - config::GAP_BUFFER_MS));
    }

    for i in 0..input.len() {
        let end = input[i].end();
        out.push(input[i].clone());

        if let Some(next) = input.get(i + 1) {
            let next_start = next.start();
            if next_start - end > config::GAP_INTER_THRESHOLD_MS {
                out.push(gap_sentence(
                    end + config::GAP_BUFFER_MS,
                    next_start - config::GAP_BUFFER_MS,
                ));
            }
        }
    }

    out
}

/// Index of the line that should be centred, as a float so the window can
/// glide rather than jump. The fractional part is an eased roll toward the
/// next line during the last `SCROLL_TRANSITION_MS` before it starts.
pub fn active_index(sentences: &[Sentence], now: i64) -> f32 {
    if sentences.is_empty() {
        return 0.0;
    }

    let mut base = 0usize;
    for (i, s) in sentences.iter().enumerate() {
        if now >= s.start() {
            base = i;
        } else {
            break;
        }
    }

    if let Some(next) = sentences.get(base + 1) {
        let until = (next.start() - now) as f64;
        if until > 0.0 && until <= config::SCROLL_TRANSITION_MS {
            let p = 1.0 - until / config::SCROLL_TRANSITION_MS;
            // easeInOutQuad, same curve the TypeScript build used.
            let eased = if p < 0.5 {
                2.0 * p * p
            } else {
                1.0 - (-2.0 * p + 2.0).powi(2) / 2.0
            };
            return base as f32 + eased as f32;
        }
    }

    base as f32
}

/// How far into a line the skip back button restarts it instead of moving to
/// the one before. Every music player behaves this way.
const RESTART_WINDOW_MS: i64 = 1_500;

/// Where the skip back button should jump to.
pub fn previous_line_start(sentences: &[Sentence], now: i64) -> i64 {
    let starts: Vec<i64> = sentences.iter().map(Sentence::start).collect();
    let current = starts.iter().rposition(|&s| s <= now);

    match current {
        // Past the opening moments of a line: restart it.
        Some(i) if now - starts[i] > RESTART_WINDOW_MS => starts[i],
        // Near its start: step back one.
        Some(i) if i > 0 => starts[i - 1],
        // On the first line, or before it: go to the top.
        _ => 0,
    }
}

/// Where the skip forward button should jump to. Stays on the last line, since
/// there is nowhere further to go.
pub fn next_line_start(sentences: &[Sentence], now: i64) -> i64 {
    sentences
        .iter()
        .map(Sentence::start)
        .find(|&s| s > now)
        .unwrap_or_else(|| sentences.last().map_or(now, Sentence::start))
}

/// Parse `H:MM:SS` or `MM:SS` into milliseconds.
pub fn parse_time(s: &str) -> i64 {
    let parts: Vec<i64> = s.split(':').filter_map(|p| p.parse().ok()).collect();
    match parts.len() {
        3 => (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000,
        2 => (parts[0] * 60 + parts[1]) * 1000,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(start: i64, end: i64) -> Word {
        Word { start_time: start, end_time: end, data: "x".into() }
    }

    fn line(start: i64, end: i64) -> Sentence {
        Sentence { words: vec![word(start, end)], is_gap: false }
    }

    #[test]
    fn parses_both_time_formats() {
        assert_eq!(parse_time("0:01:02"), 62_000);
        assert_eq!(parse_time("01:02"), 62_000);
        assert_eq!(parse_time("nonsense"), 0);
    }

    #[test]
    fn injects_a_leading_gap_only_when_the_intro_is_long() {
        let long = inject_gaps(vec![line(5_000, 6_000)]);
        assert!(long[0].is_gap);
        assert_eq!(long.len(), 2);

        let short = inject_gaps(vec![line(1_000, 2_000)]);
        assert!(!short[0].is_gap);
        assert_eq!(short.len(), 1);
    }

    #[test]
    fn injects_a_gap_between_distant_lines() {
        let out = inject_gaps(vec![line(0, 1_000), line(9_000, 10_000)]);
        assert_eq!(out.len(), 3);
        assert!(out[1].is_gap);
        assert_eq!(out[1].start(), 1_500);
        assert_eq!(out[1].end(), 8_500);
    }

    #[test]
    fn leaves_adjacent_lines_alone() {
        let out = inject_gaps(vec![line(0, 1_000), line(1_500, 2_000)]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn skipping_back_restarts_the_line_before_it_leaves_it() {
        let s = vec![line(0, 900), line(1_000, 1_900), line(5_000, 5_900)];

        // Just after a line starts, step back to the one before.
        assert_eq!(previous_line_start(&s, 1_100), 0);
        // Well into it, restart it instead.
        assert_eq!(previous_line_start(&s, 3_000), 1_000);
        // On the very first line there is nowhere before it.
        assert_eq!(previous_line_start(&s, 100), 0);
        // Before any line has started.
        assert_eq!(previous_line_start(&s, -50), 0);
    }

    #[test]
    fn skipping_forward_lands_on_the_next_line() {
        let s = vec![line(0, 900), line(1_000, 1_900), line(5_000, 5_900)];

        assert_eq!(next_line_start(&s, 0), 1_000);
        assert_eq!(next_line_start(&s, 1_500), 5_000);
        // Nowhere further to go.
        assert_eq!(next_line_start(&s, 9_000), 5_000);
    }

    #[test]
    fn skipping_an_empty_lyric_set_is_not_a_panic() {
        assert_eq!(previous_line_start(&[], 1_000), 0);
        assert_eq!(next_line_start(&[], 1_000), 1_000);
    }

    #[test]
    fn active_index_eases_into_the_next_line() {
        let s = vec![line(0, 1_000), line(5_000, 6_000)];
        assert_eq!(active_index(&s, 100), 0.0);
        // Outside the transition window it stays put.
        assert_eq!(active_index(&s, 4_000), 0.0);
        // Inside it, it climbs toward 1 without reaching it.
        let mid = active_index(&s, 4_750);
        assert!(mid > 0.0 && mid < 1.0, "got {mid}");
        assert_eq!(active_index(&s, 5_000), 1.0);
    }
}
