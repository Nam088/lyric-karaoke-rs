//! Fitting the interface into whatever the terminal gives us.
//!
//! The panel has one elastic part, the lyric window, and a lot of chrome
//! around it. At full size it wants 23 rows, which is already more than a
//! standard 80x24 terminal has once anything else is on screen, so the chrome
//! is dropped in order of how little it matters until the whole thing fits.
//! Lyrics are never given up: there is always at least one line.

use crate::config;

/// Rows that exist no matter what: two border rows and the header. The
/// timeline is added on top, since its height depends on which style is on.
const ESSENTIAL_ROWS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub box_width: usize,
    /// Usable width inside the panel, after border and padding.
    pub inner_width: usize,
    /// Lyric lines shown. Always odd, so one sits in the middle.
    pub window: usize,
    /// Blank rows between lyric lines.
    pub line_spacing: usize,
    /// Terminal rows for the spectrum. Zero hides it.
    pub spectrum_rows: usize,
    pub show_padding: bool,
    pub show_rules: bool,
    pub show_ticker: bool,
    pub show_transport: bool,
}

/// What gets sacrificed, and in what order. Each entry is the rows it costs.
///
/// Blank space goes first, then decoration, then the parts that carry
/// information. The scrolling title is the last to go because it is the only
/// thing naming the song.
const SACRIFICES: [(Feature, usize); 5] = [
    (Feature::LineSpacing, 0), // variable, handled separately
    (Feature::Padding, 2),
    (Feature::Transport, 2),
    (Feature::Rules, 2),
    (Feature::Ticker, 2), // the ticker row plus the footer margin above it
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Feature {
    LineSpacing,
    Padding,
    Transport,
    Rules,
    Ticker,
}

impl Layout {
    /// `show_spectrum` comes from the live style rather than the config, so
    /// pressing `S` reflows the panel immediately.
    pub fn measure_with(term_w: usize, term_h: usize, show_spectrum: bool) -> Self {
        let box_width = term_w
            .saturating_sub(2 + config::SAFE_MARGIN * 2)
            .min(config::MAX_BOX_WIDTH);

        // Two border columns, five of padding either side, and one column
        // left spare.
        //
        // The spare column is the important part. Full width children that
        // exactly fill the content area end up competing for the last one, and
        // whichever loses gets squeezed to a single column and wraps once per
        // character. That is how the panel has twice ended up hundreds of rows
        // tall with nothing visible on screen.
        let inner_width = box_width.saturating_sub(13).max(20);

        let mut l = Self {
            box_width,
            inner_width,
            window: config::WINDOW_SIZE,
            line_spacing: config::LINE_SPACING,
            spectrum_rows: if show_spectrum { config::SPECTRUM_ROWS } else { 0 },
            show_padding: true,
            show_rules: true,
            show_ticker: true,
            show_transport: true,
        };

        for (feature, _) in SACRIFICES {
            if l.rows_needed() <= term_h {
                break;
            }
            match feature {
                Feature::LineSpacing => l.line_spacing = 0,
                Feature::Padding => l.show_padding = false,
                Feature::Transport => l.show_transport = false,
                Feature::Rules => l.show_rules = false,
                Feature::Ticker => l.show_ticker = false,
            }
        }

        // Then trade spectrum height for lyric lines, since the words are the
        // point of the exercise.
        while l.rows_needed() > term_h && l.spectrum_rows > 0 {
            l.spectrum_rows -= 1;
        }

        // Only now start dropping lines, and never the last one.
        while l.rows_needed() > term_h && l.window > 1 {
            l.window -= 2;
        }

        l
    }

    /// Total terminal rows this layout occupies.
    pub fn rows_needed(&self) -> usize {
        let lyrics = self.window + self.window.saturating_sub(1) * self.line_spacing;
        // Two rows of margin sit around the lyric block whenever there is
        // padding to separate it from.
        let lyric_margins = if self.show_padding { 2 } else { 0 };

        ESSENTIAL_ROWS
            + crate::ui::footer::timeline_rows()
            + lyrics
            + lyric_margins
            + self.spectrum_rows
            + if self.show_padding { 2 } else { 0 }
            + if self.show_rules { 2 } else { 0 }
            + if self.show_ticker { 2 } else { 0 }
            + if self.show_transport { 2 } else { 0 }
    }

    /// Lines either side of the centre one.
    pub fn half_window(&self) -> i64 {
        (self.window / 2) as i64
    }

    pub fn padding_y(&self) -> u32 {
        u32::from(self.show_padding)
    }

    pub fn lyric_margin(&self) -> u32 {
        u32::from(self.show_padding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fewest rows anything can be drawn in: the frame, the header, the
    /// timeline, and one line to sing.
    fn minimum_rows() -> usize {
        ESSENTIAL_ROWS + crate::ui::footer::timeline_rows() + 1
    }

    /// Switching the spectrum off has to actually free its rows, not just stop
    /// drawing into them.
    ///
    /// Where they go depends on how tight the terminal is. The layout gives
    /// back whatever it sacrificed first, so on a short screen the padding
    /// returns before the blank rows between lyric lines do, and on some
    /// heights nothing is cheap enough to buy so the rows stay as slack.
    ///
    /// The one thing that must always hold is that nothing gets worse.
    #[test]
    fn hiding_the_spectrum_gives_the_rows_back() {
        for h in 10..40usize {
            let on = Layout::measure_with(80, h, true);
            let off = Layout::measure_with(80, h, false);

            assert_eq!(off.spectrum_rows, 0, "at {h}: the spectrum still has rows");

            // Nothing may get worse.
            assert!(off.window >= on.window, "at {h}: lost lyric lines");
            assert!(off.line_spacing >= on.line_spacing, "at {h}: lost spacing");
            assert!(on.show_padding <= off.show_padding, "at {h}: lost padding");
            assert!(on.show_rules <= off.show_rules, "at {h}: lost the rules");
            assert!(on.show_ticker <= off.show_ticker, "at {h}: lost the ticker");
            assert!(on.show_transport <= off.show_transport, "at {h}: lost transport");

            // Both still fit. Note the panel may end up *taller* without the
            // spectrum, because the freed rows plus whatever slack was already
            // there can add up to something the layout could not previously
            // afford, such as the blank lines between lyrics at six rows.
            assert!(on.rows_needed() <= h.max(minimum_rows()), "at {h}: on does not fit");
            assert!(off.rows_needed() <= h.max(minimum_rows()), "at {h}: off does not fit");
        }
    }

    #[test]
    fn the_layout_still_fits_at_every_height_with_the_spectrum_off() {
        for h in 4..80usize {
            let l = Layout::measure_with(100, h, false);
            assert_eq!(l.spectrum_rows, 0);
            assert!(
                l.rows_needed() <= h.max(minimum_rows()),
                "at height {h}: wants {} rows, layout {l:?}",
                l.rows_needed()
            );
        }
    }

    #[test]
    fn a_tall_terminal_gets_the_full_design() {
        let l = Layout::measure_with(120, 60, true);
        assert_eq!(l.window, config::WINDOW_SIZE);
        assert_eq!(l.line_spacing, config::LINE_SPACING);
        assert_eq!(l.spectrum_rows, config::SPECTRUM_ROWS);
        assert!(l.show_padding && l.show_rules && l.show_ticker && l.show_transport);
    }

    #[test]
    fn a_standard_terminal_keeps_every_lyric_line() {
        let l = Layout::measure_with(80, 24, true);
        assert_eq!(l.window, config::WINDOW_SIZE);
        assert_eq!(l.line_spacing, 0, "the blank rows are the first thing to go");
        assert!(l.show_rules && l.show_ticker, "chrome should survive at 24 rows");
        assert!(l.rows_needed() <= 24);
    }

    #[test]
    fn it_always_fits_and_always_shows_a_lyric() {
        for h in 4..80usize {
            let l = Layout::measure_with(100, h, true);
            assert!(
                l.rows_needed() <= h.max(minimum_rows()),
                "at height {h}: wants {} rows, layout {l:?}",
                l.rows_needed()
            );
            assert!(l.window >= 1, "at height {h} there is nothing to sing");
            assert_eq!(l.window % 2, 1, "at height {h} no line is centred");
        }
    }

    #[test]
    fn chrome_is_dropped_in_order_of_importance() {
        // Walking down from a comfortable height, each feature should switch
        // off no later than the ones after it in the sacrifice order.
        let mut lost_transport = None;
        let mut lost_ticker = None;
        for h in (6..40usize).rev() {
            let l = Layout::measure_with(100, h, true);
            if !l.show_transport && lost_transport.is_none() {
                lost_transport = Some(h);
            }
            if !l.show_ticker && lost_ticker.is_none() {
                lost_ticker = Some(h);
            }
        }
        let (t, k) = (lost_transport.unwrap(), lost_ticker.unwrap());
        assert!(t >= k, "transport ({t}) should go before the ticker ({k})");
    }

    #[test]
    fn the_smallest_useful_terminal_still_renders_something() {
        let l = Layout::measure_with(40, 6, true);
        assert_eq!(l.window, 1);
        assert_eq!(l.spectrum_rows, 0);
        assert!(!l.show_padding);
    }

    #[test]
    fn the_panel_is_capped_so_lyrics_do_not_stretch_across_a_wide_screen() {
        assert_eq!(Layout::measure_with(400, 60, true).box_width, config::MAX_BOX_WIDTH);
    }

    #[test]
    fn there_is_always_a_spare_column_inside_the_panel() {
        for w in [40usize, 60, 80, 100, 132, 200] {
            let l = Layout::measure_with(w, 40, true);
            // Border and padding account for twelve columns; anything left
            // over beyond inner_width is the slack.
            assert!(
                l.inner_width + 12 < l.box_width || l.box_width < 33,
                "at {w} columns: inner {} fills box {} exactly",
                l.inner_width,
                l.box_width
            );
        }
    }

    #[test]
    fn the_panel_never_reaches_the_edge_of_the_terminal() {
        // A glyph the terminal draws wider than the layout measured it would
        // otherwise wrap the line and corrupt every row below.
        for w in [40usize, 80, 100, 132, 200] {
            let l = Layout::measure_with(w, 40, true);
            assert!(
                l.box_width + 2 * config::SAFE_MARGIN <= w,
                "at {w} columns the panel is {} wide with no room to spare",
                l.box_width
            );
        }
    }
}
