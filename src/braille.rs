//! A drawing surface made of braille characters.
//!
//! Each terminal cell holds a 2x4 dot matrix (U+2800 plus a bit per dot), so
//! a four row strip is really a 16 pixel tall canvas at double horizontal
//! resolution. That is enough to draw a curve instead of a staircase, which
//! the block characters `▂▃▄▅▆▇█` cannot do: they give eight levels per row
//! and no horizontal detail at all.

/// Dot bit for (row, column) within one cell.
const DOT: [[u8; 2]; 4] = [
    [0x01, 0x08],
    [0x02, 0x10],
    [0x04, 0x20],
    [0x40, 0x80],
];

pub struct Canvas {
    cells_w: usize,
    cells_h: usize,
    bits: Vec<u8>,
}

impl Canvas {
    pub fn new(cells_w: usize, cells_h: usize) -> Self {
        Self { cells_w, cells_h, bits: vec![0; cells_w * cells_h] }
    }

    /// Width in dots, which is twice the width in terminal columns.
    pub fn width(&self) -> usize {
        self.cells_w * 2
    }

    /// Height in dots, which is four times the height in terminal rows.
    pub fn height(&self) -> usize {
        self.cells_h * 4
    }

    /// Light one dot. Coordinates outside the canvas are ignored so callers
    /// do not have to clamp.
    pub fn set(&mut self, x: usize, y: usize) {
        if x >= self.width() || y >= self.height() {
            return;
        }
        self.bits[(y / 4) * self.cells_w + (x / 2)] |= DOT[y % 4][x % 2];
    }

    /// One string per terminal row. Rows are separate so each can be given
    /// its own colour, which is how the vertical gradient is done.
    pub fn rows(&self) -> Vec<String> {
        (0..self.cells_h)
            .map(|r| {
                self.bits[r * self.cells_w..(r + 1) * self.cells_w]
                    .iter()
                    .map(|&b| char::from_u32(0x2800 + b as u32).unwrap_or(' '))
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_canvas_is_blank_braille_not_spaces() {
        // U+2800 renders as blank but keeps the column width stable, which
        // matters when it sits next to lit cells.
        let rows = Canvas::new(3, 1).rows();
        assert_eq!(rows, vec!["\u{2800}\u{2800}\u{2800}"]);
    }

    #[test]
    fn dots_map_to_the_expected_code_points() {
        let mut c = Canvas::new(1, 1);
        c.set(0, 0);
        assert_eq!(c.rows()[0], "\u{2801}");

        let mut c = Canvas::new(1, 1);
        c.set(1, 3);
        assert_eq!(c.rows()[0], "\u{2880}");

        let mut c = Canvas::new(1, 1);
        for y in 0..4 {
            for x in 0..2 {
                c.set(x, y);
            }
        }
        assert_eq!(c.rows()[0], "\u{28FF}");
    }

    #[test]
    fn out_of_bounds_writes_are_dropped() {
        let mut c = Canvas::new(2, 1);
        c.set(99, 0);
        c.set(0, 99);
        assert_eq!(c.rows()[0], "\u{2800}\u{2800}");
    }

    #[test]
    fn reports_dot_dimensions_not_cell_dimensions() {
        let c = Canvas::new(45, 4);
        assert_eq!(c.width(), 90);
        assert_eq!(c.height(), 16);
    }
}
