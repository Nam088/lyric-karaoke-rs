//! One line of lyrics, with the karaoke fill.
//!
//! The fill advances by grapheme cluster, not by code unit. macOS stores
//! filenames and often text in NFD, where "ế" is a base letter plus two
//! combining marks. The TypeScript version used `word.data.length`, so on
//! decomposed Vietnamese it counted three units for one visible character and
//! the highlight ran ahead of the voice.

use iocraft::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

use crate::color;
use crate::config;
use crate::lyrics::Sentence;

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Past,
    Active,
    Future,
}

/// How far through a word the voice is, as a grapheme count.
fn filled_graphemes(text: &str, now: i64, start: i64, end: i64) -> usize {
    let total = text.graphemes(true).count();
    let duration = end - start;
    if duration <= 0 {
        return total;
    }
    let progress = (now - start) as f32 / duration as f32;
    ((progress * total as f32).ceil() as usize).min(total)
}

fn take_graphemes(text: &str, range: std::ops::Range<usize>) -> String {
    text.graphemes(true)
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .collect()
}

fn span(text: String, color: Color, bold: bool) -> AnyElement<'static> {
    element! {
        Text(
            color: color,
            weight: if bold { Weight::Bold } else { Weight::Normal },
            content: text,
        )
    }
    .into()
}

/// A run of text on the active line, with the colour it is drawn in.
type Span = (String, Color);

/// The word currently being sung, split into what has landed, the character
/// on the beat, and what is still ahead.
fn singing_word(text: &str, now: i64, start: i64, end: i64) -> Vec<Span> {
    let filled = filled_graphemes(text, now, start, end);
    let total = text.graphemes(true).count();

    [
        (take_graphemes(text, 0..filled.saturating_sub(1)), config::LYRIC_PAST),
        (take_graphemes(text, filled.saturating_sub(1)..filled), config::LYRIC_HIT),
        (take_graphemes(text, filled..total), config::LYRIC_SINGING),
    ]
    .into_iter()
    .filter(|(t, _)| !t.is_empty())
    .collect()
}

/// The active line, word by word, with a separator *between* words rather
/// than after each one.
///
/// The trailing space mattered. It made the active line one column wider than
/// the same words on any other line, so the text sat off centre and the right
/// hand marker was pushed further out than the left one.
fn active_spans(sentence: &Sentence, now: i64) -> Vec<Span> {
    let mut out = Vec::with_capacity(sentence.words.len() * 3);

    for (i, w) in sentence.words.iter().enumerate() {
        if i > 0 {
            out.push((" ".to_string(), config::LYRIC_SINGING));
        }

        if now >= w.end_time {
            out.push((w.data.clone(), config::LYRIC_PAST));
        } else if now >= w.start_time {
            out.extend(singing_word(&w.data, now, w.start_time, w.end_time));
        } else {
            out.push((w.data.clone(), config::LYRIC_SINGING));
        }
    }

    out
}

pub fn render(
    sentence: &Sentence,
    now: i64,
    status: Status,
    distance: f32,
    gap_is_active: bool,
    margin_bottom: u32,
) -> AnyElement<'static> {
    let is_active = status == Status::Active;

    // While an instrumental break is the active line, everything else steps
    // out of the way so the notes stand alone.
    if gap_is_active && !is_active {
        return element! { View(margin_bottom: margin_bottom) { Text(content: " ") } }.into();
    }

    let fade = color::distance_fade(distance);

    // The marker sits on the line being sung and nowhere else.
    //
    // The TypeScript build put it on every line within one step of the centre
    // and dimmed the neighbours to near black to hide them. That only works on
    // a terminal whose background really is black; on any other theme three
    // lines wore the marker at once. An instrumental break does not get one
    // either, because it is already a row of musical notes and two sets of
    // symbols on one line read as noise.
    let show_indicator = is_active && !sentence.is_gap;
    let symbol = config::MUSIC_NOTES[((now as f64 / config::DANCING_INDICATOR_MS) as usize)
        % config::MUSIC_NOTES.len()];

    let words: Vec<AnyElement<'static>> = if sentence.is_gap {
        let alt = ((now as f64 / config::GAP_PATTERN_MS) as i64) % 2 == 0;
        let pattern = if alt { config::GAP_TEXT } else { config::GAP_ALT_TEXT };
        let c = if is_active {
            config::KEYBINDS_HIGHLIGHT
        } else {
            color::fade(config::LYRIC_PAST, fade, config::DARK_BASE)
        };
        vec![span(pattern.to_string(), c, true)]
    } else if !is_active {
        let base = if status == Status::Past {
            config::LYRIC_PAST
        } else {
            config::LYRIC_FUTURE
        };
        let c = color::fade(base, fade, config::DARK_BASE);
        vec![span(sentence.text(), c, false)]
    } else {
        active_spans(sentence, now)
            .into_iter()
            .map(|(t, c)| span(t, c, true))
            .collect()
    };

    // Both markers are the same glyph, so whatever width the terminal gives it
    // they stay balanced around the words. The reserved column stays put even
    // when nothing is drawn in it, so lines do not shift as the marker moves.
    let indicator = || -> Vec<AnyElement<'static>> {
        if show_indicator {
            vec![element! {
                Text(color: config::LYRIC_HIT, weight: Weight::Bold, content: symbol)
            }
            .into()]
        } else {
            Vec::new()
        }
    };

    element! {
        View(
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            margin_bottom: margin_bottom,
        ) {
            View(width: 5, justify_content: JustifyContent::Center) {
                #(indicator())
            }
            View(flex_direction: FlexDirection::Row) { #(words) }
            View(width: 5, justify_content: JustifyContent::Center) {
                #(indicator())
            }
        }
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::Word;

    fn line(words: &[(&str, i64, i64)]) -> Sentence {
        Sentence {
            words: words
                .iter()
                .map(|&(data, start_time, end_time)| Word {
                    data: data.into(),
                    start_time,
                    end_time,
                })
                .collect(),
            is_gap: false,
        }
    }

    fn joined(spans: &[Span]) -> String {
        spans.iter().map(|(t, _)| t.as_str()).collect()
    }

    /// Plain text of a rendered line, escape codes stripped.
    fn drawn(sentence: &Sentence, status: Status, gap_is_active: bool) -> String {
        let mut e = render(sentence, 1_500, status, 0.0, gap_is_active, 0);
        let mut buf = Vec::new();
        e.render(Some(60)).write(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).replace('\n', "")
    }

    #[test]
    fn the_fill_counts_visible_characters_not_code_units() {
        // "ế" decomposed: e + circumflex + acute. Three code points, one
        // grapheme. Counting code points would make this word look three
        // times as long as it is.
        let decomposed = "be\u{0302}\u{0301}";
        assert_eq!(decomposed.chars().count(), 4);
        assert_eq!(decomposed.graphemes(true).count(), 2);

        // Halfway through the word should light exactly one of the two.
        assert_eq!(filled_graphemes(decomposed, 500, 0, 1000), 1);
        assert_eq!(filled_graphemes(decomposed, 1000, 0, 1000), 2);
    }

    #[test]
    fn the_fill_spans_the_whole_word_by_its_end() {
        assert_eq!(filled_graphemes("hello", 0, 0, 1000), 0);
        assert_eq!(filled_graphemes("hello", 1000, 0, 1000), 5);
        assert_eq!(filled_graphemes("hello", 5000, 0, 1000), 5);
    }

    #[test]
    fn a_zero_length_word_is_treated_as_complete() {
        assert_eq!(filled_graphemes("xin", 0, 500, 500), 3);
    }

    #[test]
    fn the_active_line_is_exactly_as_wide_as_its_words() {
        // A trailing space after the last word made the active line one column
        // wider than the same text on any neighbouring line, which threw the
        // centring off and pushed the right hand marker outward.
        let s = line(&[("Đừng", 0, 500), ("về", 500, 900), ("trễ", 900, 1400)]);

        for now in [0, 700, 1_200, 5_000] {
            let text = joined(&active_spans(&s, now));
            assert_eq!(text, s.text(), "at {now}ms");
            assert!(!text.ends_with(' '), "trailing space at {now}ms");
        }
    }

    #[test]
    fn the_marker_is_only_on_the_line_being_sung() {
        let s = line(&[("một", 0, 500), ("hai", 500, 1000)]);

        let active = drawn(&s, Status::Active, false);
        assert!(
            config::MUSIC_NOTES.iter().any(|n| active.contains(n.trim())),
            "the active line should carry a marker: {active:?}"
        );

        for status in [Status::Past, Status::Future] {
            let other = drawn(&s, status, false);
            assert!(
                !config::MUSIC_NOTES.iter().any(|n| other.contains(n.trim())),
                "a neighbouring line should not: {other:?}"
            );
        }
    }

    #[test]
    fn an_instrumental_break_gets_no_marker() {
        // It is already a row of musical notes. Two sets of symbols on one
        // line reads as noise.
        let mut gap = line(&[(config::GAP_TEXT, 0, 4_000)]);
        gap.is_gap = true;

        let drawn = drawn(&gap, Status::Active, true);
        assert!(drawn.contains('♫') || drawn.contains('♪'), "pattern missing");
        assert!(!drawn.contains('♯'), "marker leaked onto a gap: {drawn:?}");
        assert!(!drawn.contains('♬'), "marker leaked onto a gap: {drawn:?}");
    }

    #[test]
    fn slicing_by_grapheme_never_splits_a_character() {
        let s = "be\u{0302}\u{0301}o";
        assert_eq!(take_graphemes(s, 0..1), "b");
        assert_eq!(take_graphemes(s, 1..2), "e\u{0302}\u{0301}");
        assert_eq!(take_graphemes(s, 2..3), "o");
        assert_eq!(take_graphemes(s, 0..99), s);
        // A backwards range yields nothing rather than panicking.
        #[allow(clippy::reversed_empty_ranges)]
        let backwards = take_graphemes(s, 5..2);
        assert_eq!(backwards, "");
    }
}
