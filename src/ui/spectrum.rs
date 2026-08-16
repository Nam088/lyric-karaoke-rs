//! Drawing the spectrum, in whichever style is selected.
//!
//! The measurement is the same for all of them; only the picture differs. Most
//! draw onto a braille canvas in three layers: a solid crest along the top of
//! each column, a dithered body beneath it, and peak markers that hold and
//! fall. A braille cell cannot carry two colours, so each cell takes the colour
//! of the highest layer it contains and neighbouring cells of the same colour
//! merge into one `Text` run.

use iocraft::prelude::*;

use crate::analysis::Analyzer;
use crate::braille::Canvas;
use crate::color::Theme;

/// How the spectrum is drawn, or whether it is drawn at all. Cycled with `S`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Style {
    /// A bright crest over a dithered body.
    #[default]
    Curve,
    /// Grown out from a centre line in both directions.
    Mirror,
    /// Just the crest, nothing under it.
    Line,
    /// Block characters, one bar per column. The familiar look, and the only
    /// style that does not need braille.
    Bars,
    /// Hidden. The rows it would have taken go back to the lyrics.
    Off,
}

impl Style {
    /// The styles that actually draw something.
    #[cfg(test)]
    pub const DRAWN: [Style; 4] = [Style::Curve, Style::Mirror, Style::Line, Style::Bars];

    /// Everything the `S` key steps through, hidden included.
    pub const ALL: [Style; 5] = [
        Style::Curve,
        Style::Mirror,
        Style::Line,
        Style::Bars,
        Style::Off,
    ];

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn is_visible(self) -> bool {
        self != Style::Off
    }

    pub fn name(self) -> &'static str {
        match self {
            Style::Curve => "curve",
            Style::Mirror => "mirror",
            Style::Line => "line",
            Style::Bars => "bars",
            Style::Off => "off",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Empty,
    Fill,
    Edge,
    Peak,
}

impl Layer {
    fn color(self, theme: &Theme) -> Color {
        match self {
            Layer::Peak => theme.spectrum_peak,
            Layer::Edge => theme.spectrum_edge,
            _ => theme.spectrum_fill,
        }
    }
}

/// Build the three braille layers for a style. Returned separately so each
/// cell can be coloured by the highest layer it contains.
fn draw(a: &Analyzer, cells_w: usize, cells_h: usize, style: Style) -> (Canvas, Canvas, Canvas) {
    let mut fill = Canvas::new(cells_w, cells_h);
    let mut edge = Canvas::new(cells_w, cells_h);
    let mut peak = Canvas::new(cells_w, cells_h);

    let h = fill.height();
    let w = fill.width().min(a.levels.len());

    for x in 0..w {
        let level = a.levels[x];

        match style {
            Style::Mirror => {
                // Half the height each way, so the total ink matches the other
                // styles rather than doubling.
                let mid = h / 2;
                let n = ((level * mid as f32).round() as usize).min(mid);
                if n > 0 {
                    edge.set(x, mid - n);
                    edge.set(x, mid + n - 1);
                }
                for y in 1..n {
                    if (x + y) % 2 == 0 {
                        fill.set(x, mid - n + y);
                        fill.set(x, mid + n - 1 - y);
                    }
                }
            }
            _ => {
                let n = ((level * h as f32).round() as usize).min(h);
                if n > 0 {
                    // Solid crest.
                    edge.set(x, h - n);

                    // Dithered body. A checkerboard halves the ink so the
                    // region reads as shaded without a second character set.
                    if style == Style::Curve {
                        for y in 1..n {
                            if (x + y) % 2 == 0 {
                                fill.set(x, h - n + y);
                            }
                        }
                    }
                }

                // Peak marker, only once it has separated from the crest.
                let p = ((a.peaks[x] * h as f32).round() as usize).min(h);
                if p > n + 1 {
                    peak.set(x, h - p);
                }
            }
        }
    }

    (fill, edge, peak)
}

/// Block characters, eight steps per row. One bar per terminal column, so the
/// two braille dot columns behind it are averaged.
fn bars(a: &Analyzer, cells_w: usize, cells_h: usize) -> Vec<String> {
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let steps = cells_h * 8;

    let mut rows = vec![String::with_capacity(cells_w); cells_h];
    for x in 0..cells_w {
        let level = match (a.levels.get(x * 2), a.levels.get(x * 2 + 1)) {
            (Some(&l), Some(&r)) => (l + r) / 2.0,
            (Some(&l), None) => l,
            _ => 0.0,
        };
        let filled = ((level * steps as f32).round() as usize).min(steps);

        for (r, row) in rows.iter_mut().enumerate() {
            // Rows are built top down, so the bottom row is the last one.
            let below = (cells_h - 1 - r) * 8;
            row.push(BLOCKS[filled.saturating_sub(below).min(8)]);
        }
    }
    rows
}

/// Group a row of cells into runs of the same colour.
fn runs(fill: &str, edge: &str, peak: &str) -> Vec<(String, Layer)> {
    let mut out: Vec<(String, Layer)> = Vec::new();

    for ((f, e), p) in fill.chars().zip(edge.chars()).zip(peak.chars()) {
        let bits = |c: char| c as u32 - 0x2800;
        let (fb, eb, pb) = (bits(f), bits(e), bits(p));

        let layer = if pb != 0 {
            Layer::Peak
        } else if eb != 0 {
            Layer::Edge
        } else if fb != 0 {
            Layer::Fill
        } else {
            Layer::Empty
        };

        let merged = char::from_u32(0x2800 + (fb | eb | pb)).unwrap_or(' ');

        match out.last_mut() {
            Some((s, l)) if *l == layer => s.push(merged),
            _ => out.push((merged.to_string(), layer)),
        }
    }

    out
}

pub fn render(
    a: &Analyzer,
    cells_w: usize,
    cells_h: usize,
    style: Style,
    theme: &Theme,
) -> AnyElement<'static> {
    if style == Style::Bars {
        let rows: Vec<AnyElement<'static>> = bars(a, cells_w, cells_h)
            .into_iter()
            .enumerate()
            .map(|(r, row)| {
                // Brighter at the top, so tall bars read as peaks.
                let color = if r == 0 {
                    theme.spectrum_edge
                } else {
                    theme.spectrum_fill
                };
                element! { Text(color: color, content: row) }.into()
            })
            .collect();

        // Rows are aligned to the start, not centred. A bar chart is mostly
        // blank, the renderer drops trailing whitespace, and a row that
        // shrinks inside a centred container slides sideways. Anchoring the
        // left edge holds it still.
        //
        // The width is a percentage on purpose. Pinning it to the exact column
        // count makes a full width sibling overflow, and an overflowing Text
        // wraps once per character, which empties the whole panel.
        return element! {
            View(
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                width: 100pct,
            ) {
                #(rows)
            }
        }
        .into();
    }

    let (fill, edge, peak) = draw(a, cells_w, cells_h, style);
    let (fr, er, pr) = (fill.rows(), edge.rows(), peak.rows());

    let rows: Vec<AnyElement<'static>> = (0..cells_h)
        .map(|r| {
            let spans: Vec<AnyElement<'static>> = runs(&fr[r], &er[r], &pr[r])
                .into_iter()
                .map(|(text, layer)| {
                    element! { Text(color: layer.color(theme), content: text) }.into()
                })
                .collect();

            element! {
                View(flex_direction: FlexDirection::Row) { #(spans) }
            }
            .into()
        })
        .collect();

    element! {
        View(
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            width: 100pct,
        ) {
            #(rows)
        }
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyzer(levels: Vec<f32>) -> Analyzer {
        let mut a = Analyzer::new(levels.len());
        a.peaks = vec![0.0; levels.len()];
        a.levels = levels;
        a
    }

    #[test]
    fn a_silent_spectrum_draws_nothing() {
        let a = analyzer(vec![0.0; 8]);
        for style in Style::DRAWN {
            if style == Style::Bars {
                assert!(bars(&a, 4, 2).iter().all(|r| r.chars().all(|c| c == ' ')));
                continue;
            }
            let (fill, edge, peak) = draw(&a, 4, 2, style);
            for canvas in [fill, edge, peak] {
                assert!(
                    canvas.rows().iter().all(|r| r.chars().all(|c| c == '\u{2800}')),
                    "{style:?} drew something for silence"
                );
            }
        }
    }

    /// Read a bar column back out of the rendered rows as a step count.
    fn column_steps(rows: &[String], x: usize) -> usize {
        const BLOCKS: [char; 9] = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}',
                                   '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        rows.iter()
            .map(|r| {
                let ch = r.chars().nth(x).unwrap();
                BLOCKS.iter().position(|&b| b == ch).expect("not a block char")
            })
            .sum()
    }

    #[test]
    fn a_bar_column_is_solid_from_the_bottom_up() {
        let n = 64;
        let levels: Vec<f32> = (0..n).map(|i| i as f32 / n as f32).collect();
        let mut a = analyzer(levels);
        a.peaks = vec![0.0; n];

        let rows = bars(&a, n / 2, 4);

        for x in 0..n / 2 {
            let cells: Vec<char> = rows.iter().map(|r| r.chars().nth(x).unwrap()).collect();
            // Walking down: empties, then at most one partial, then only fulls.
            let mut seen_partial = false;
            let mut seen_full = false;
            for (r, &c) in cells.iter().enumerate() {
                match c {
                    ' ' => assert!(
                        !seen_partial && !seen_full,
                        "column {x} has a gap above a filled cell at row {r}: {cells:?}"
                    ),
                    '\u{2588}' => seen_full = true,
                    _ => {
                        assert!(!seen_partial, "column {x} has two partial cells: {cells:?}");
                        assert!(!seen_full, "column {x} is partial below a full cell: {cells:?}");
                        seen_partial = true;
                    }
                }
            }
        }
    }

    #[test]
    fn a_bar_height_matches_the_level_it_came_from() {
        let cells_h = 4;
        let steps = cells_h * 8;

        for level in [0.0f32, 0.1, 0.25, 0.5, 0.75, 0.99, 1.0] {
            let a = analyzer(vec![level; 8]);
            let rows = bars(&a, 4, cells_h);
            let want = (level * steps as f32).round() as usize;
            for x in 0..4 {
                assert_eq!(
                    column_steps(&rows, x),
                    want,
                    "level {level} drew {} steps, wanted {want}",
                    column_steps(&rows, x)
                );
            }
        }
    }

    #[test]
    fn bars_cover_the_whole_spectrum() {
        // Only the top half of the range is loud. If the bars read the wrong
        // slice of levels, the loud half lands in the wrong place.
        let n = 64;
        let levels: Vec<f32> = (0..n).map(|i| if i < n / 2 { 0.0 } else { 1.0 }).collect();
        let mut a = analyzer(levels);
        a.peaks = vec![0.0; n];

        let rows = bars(&a, n / 2, 4);
        for x in 0..n / 4 {
            assert_eq!(column_steps(&rows, x), 0, "left half should be silent at {x}");
        }
        for x in n / 4..n / 2 {
            assert_eq!(column_steps(&rows, x), 32, "right half should be full at {x}");
        }
    }

    /// Where a rendered row starts, in columns from the left.
    fn left_edge(a: &Analyzer, cells_w: usize, style: Style) -> Vec<usize> {
        let theme = Theme::default();
        let mut e = render(a, cells_w, 2, style, &theme);
        let mut buf = Vec::new();
        e.render(Some(60)).write(&mut buf).unwrap();
        String::from_utf8_lossy(&buf)
            .lines()
            .map(|l| l.len() - l.trim_start().len())
            .collect()
    }

    /// Trailing blanks are load bearing.
    ///
    /// The writer drops trailing whitespace, and a bar chart is mostly empty
    /// space, so a row's rendered width follows the music. Inside a centred
    /// container that would slide the whole chart sideways every frame. The
    /// braille styles are immune because their blank cell is U+2800, which is
    /// not whitespace.
    #[test]
    fn bars_do_not_slide_when_the_music_changes() {
        let n = 32;
        let loud_left = {
            let mut a = analyzer((0..n).map(|i| if i < 4 { 1.0 } else { 0.0 }).collect());
            a.peaks = vec![0.0; n];
            a
        };
        let loud_wide = {
            let mut a = analyzer(vec![1.0; n]);
            a.peaks = vec![0.0; n];
            a
        };

        assert_eq!(
            left_edge(&loud_left, n / 2, Style::Bars),
            left_edge(&loud_wide, n / 2, Style::Bars),
            "the bars moved sideways when the spectrum changed"
        );
    }

    #[test]
    fn cycling_visits_every_style_including_off_and_comes_back() {
        let mut seen = vec![Style::default()];
        let mut s = Style::default();
        for _ in 0..Style::ALL.len() {
            s = s.next();
            if s != Style::default() {
                seen.push(s);
            }
        }
        assert_eq!(s, Style::default(), "cycling should return to the start");
        for style in Style::ALL {
            assert!(seen.contains(&style), "{style:?} is unreachable from S");
        }
        assert!(seen.contains(&Style::Off), "S cannot switch the spectrum off");
    }

    #[test]
    fn only_off_is_invisible() {
        assert!(!Style::Off.is_visible());
        for style in Style::DRAWN {
            assert!(style.is_visible(), "{style:?} should draw something");
        }
    }

    #[test]
    fn every_style_fills_the_width_it_is_given() {
        let a = analyzer(vec![0.6; 16]);
        for style in Style::DRAWN {
            let rows = if style == Style::Bars {
                bars(&a, 8, 2)
            } else {
                let (fill, edge, peak) = draw(&a, 8, 2, style);
                let (f, e, p) = (fill.rows(), edge.rows(), peak.rows());
                (0..2).map(|r| runs(&f[r], &e[r], &p[r])
                    .into_iter().map(|(t, _)| t).collect()).collect()
            };
            assert_eq!(rows.len(), 2, "{style:?} row count");
            for row in rows {
                assert_eq!(row.chars().count(), 8, "{style:?} row width");
            }
        }
    }

    #[test]
    fn mirror_grows_from_the_middle_and_the_others_from_the_bottom() {
        let a = analyzer(vec![0.5; 2]);

        let (_, edge, _) = draw(&a, 1, 2, Style::Mirror);
        let rows = edge.rows();
        let lit: Vec<usize> = (0..8).filter(|&y| dot_lit(&rows, 0, y)).collect();
        assert!(!lit.is_empty(), "mirror drew nothing");
        // Symmetric about the centre line at y = 4.
        for y in &lit {
            assert!(lit.contains(&(7 - y)), "asymmetric at y={y}: {lit:?}");
        }

        let (_, edge, _) = draw(&a, 1, 2, Style::Line);
        let lit: Vec<usize> = (0..8).filter(|&y| dot_lit(&edge.rows(), 0, y)).collect();
        assert_eq!(lit, vec![4], "line should sit at half height, got {lit:?}");
    }

    /// Whether dot (x, y) is set, reading the braille back out.
    fn dot_lit(rows: &[String], x: usize, y: usize) -> bool {
        const DOT: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
        let ch = rows[y / 4].chars().nth(x / 2).unwrap();
        (ch as u32 - 0x2800) as u8 & DOT[y % 4][x % 2] != 0
    }

    #[test]
    fn the_crest_sits_at_the_top_of_each_column() {
        // Full height column: the crest belongs on the very first dot row.
        let a = analyzer(vec![1.0, 0.0]);
        let (_, edge, _) = draw(&a, 1, 1, Style::Curve);
        let row = &edge.rows()[0];
        // Dot (0,0) is bit 0x01, so U+2801.
        assert_eq!(row.chars().next().unwrap(), '\u{2801}');
    }

    #[test]
    fn peaks_only_show_once_they_clear_the_crest() {
        let mut a = analyzer(vec![0.5, 0.5]);
        a.peaks = vec![0.5, 0.5];
        let (_, _, touching) = draw(&a, 1, 2, Style::Curve);
        assert!(touching.rows().iter().all(|r| r.chars().all(|c| c == '\u{2800}')));

        a.peaks = vec![1.0, 1.0];
        let (_, _, separated) = draw(&a, 1, 2, Style::Curve);
        assert!(separated.rows().iter().any(|r| r.chars().any(|c| c != '\u{2800}')));
    }

    #[test]
    fn adjacent_cells_of_one_colour_become_a_single_run() {
        // Two lit edge cells, then two blank.
        let out = runs("\u{2800}\u{2800}\u{2800}\u{2800}", "\u{2801}\u{2801}\u{2800}\u{2800}", "\u{2800}\u{2800}\u{2800}\u{2800}");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0.chars().count(), 2);
        assert!(matches!(out[0].1, Layer::Edge));
        assert!(matches!(out[1].1, Layer::Empty));
    }

    #[test]
    fn a_cell_takes_the_colour_of_its_highest_layer() {
        let out = runs("\u{2802}", "\u{2801}", "\u{2804}");
        assert!(matches!(out[0].1, Layer::Peak));
        // and still shows every layer's dots
        assert_eq!(out[0].0, "\u{2807}");
    }
}

#[cfg(test)]
mod preview {
    use super::*;

    #[test]
    #[ignore = "visual check: cargo test -- --ignored --nocapture preview"]
    fn show_every_style() {
        // A plausible spectrum: strong low end falling away, a couple of peaks.
        let n = 120;
        let levels: Vec<f32> = (0..n)
            .map(|i| {
                let x = i as f32 / n as f32;
                let base = (1.0 - x).powf(1.6);
                let bumps = 0.22 * ((x * 34.0).sin() + (x * 11.0).cos()) * (1.0 - x * 0.6);
                (base + bumps).clamp(0.02, 1.0)
            })
            .collect();

        let mut a = Analyzer::new(n);
        a.peaks = levels.iter().map(|v| (v + 0.16).min(1.0)).collect();
        a.levels = levels;

        for style in Style::DRAWN {
            println!("\n── {} ──", style.name().to_uppercase());
            let rows = if style == Style::Bars {
                bars(&a, n / 2, 4)
            } else {
                let (fill, edge, peak) = draw(&a, n / 2, 4, style);
                let (f, e, p) = (fill.rows(), edge.rows(), peak.rows());
                (0..4)
                    .map(|r| runs(&f[r], &e[r], &p[r]).into_iter().map(|(t, _)| t).collect())
                    .collect()
            };
            for row in rows {
                println!("  {row}");
            }
        }
    }
}
