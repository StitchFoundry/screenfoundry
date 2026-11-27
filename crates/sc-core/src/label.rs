//! The ink name printed in a film's margin, so a printer can match each film to its screen.
//!
//! The text sits between the artwork and the film edge, within the artwork's width, so it never
//! touches the artwork or the corner registration marks (which lie beyond the artwork's corners).
//! It is drawn with a small built-in 3x5 pixel font (letters, digits and a few signs; anything else
//! is a blank), scaled to the film resolution.

use crate::error::{Result, ScError};
use crate::image::Bitmap1;
use crate::marks::Rect;

/// Which margin carries the label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelPosition {
    Top,
    Bottom,
}

/// Rows of a 3x5 glyph, top first, the left column being bit 2.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 1, 1],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [7, 4, 4, 4, 7],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [7, 4, 5, 5, 7],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 7],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [7, 5, 7, 4, 4],
        'Q' => [2, 5, 5, 7, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [7, 4, 7, 1, 7],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '-' => [0, 0, 7, 0, 0],
        '.' => [0, 0, 0, 0, 2],
        '#' => [5, 7, 5, 7, 5],
        _ => [0; 5],
    }
}

/// Print `text` in the margin of a film whose artwork is `art`, within `margin_px` pixels of the
/// artwork, `scale` film pixels to a font pixel. Text too long for the artwork's width is cut, and
/// a margin too small for one line of text is an error (`MarksDontFit`).
pub fn add_label(film: &mut Bitmap1, art: Rect, margin_px: usize, text: &str, scale: usize, pos: LabelPosition) -> Result<()> {
    let k = scale.max(1);
    let height = k.checked_mul(5).ok_or(ScError::MarksDontFit)?;
    let pad = k; // clear space between the artwork and the text
    let need = height.checked_add(pad.saturating_mul(2)).ok_or(ScError::MarksDontFit)?;
    if margin_px < need {
        return Err(ScError::MarksDontFit);
    }
    let (x1, y1) = (art.x.saturating_add(art.width), art.y.saturating_add(art.height));
    let top = match pos {
        LabelPosition::Bottom => y1.checked_add(pad),
        LabelPosition::Top => art.y.checked_sub(pad.saturating_add(height)),
    }
    .ok_or(ScError::MarksDontFit)?;
    let bottom_edge = top.checked_add(height).ok_or(ScError::MarksDontFit)?;
    if bottom_edge > film.height() || x1 > film.width() {
        return Err(ScError::MarksDontFit);
    }
    let advance = 4 * k;
    let mut at = art.x;
    for c in text.chars() {
        if at.saturating_add(3 * k) > x1 {
            break;
        }
        for (row, bits) in glyph(c).iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) != 0 {
                    for dy in 0..k {
                        for dx in 0..k {
                            film.set((at + col * k + dx) as i64, (top + row * k + dy) as i64, true);
                        }
                    }
                }
            }
        }
        at += advance;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marks::{MarkSpec, frame_with_marks};

    fn framed(w: usize) -> (Bitmap1, Rect, MarkSpec) {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        let f = Bitmap1::new(w, 40).unwrap();
        let out = frame_with_marks(&f, &m).unwrap();
        let margin = m.gap_px + m.size_px + 1;
        (out, Rect { x: margin, y: margin, width: w, height: 40 }, m)
    }

    #[test]
    fn distinct_names_give_distinct_labels_outside_the_art_and_marks() {
        let mut seen = Vec::new();
        for name in ["Red 185", "Black", "White UB"] {
            let (mut film, art, m) = framed(120);
            let before = film.clone();
            add_label(&mut film, art, m.gap_px + m.size_px + 1, name, 1, LabelPosition::Bottom).unwrap();
            let mut added = 0;
            for y in 0..film.height() {
                for x in 0..film.width() {
                    if film.get(x, y) != before.get(x, y) {
                        added += 1;
                        let in_art = x >= art.x && x < art.x + art.width && y >= art.y && y < art.y + art.height;
                        assert!(!in_art, "label inside the art at {x},{y}");
                        assert!(y >= art.y + art.height && x >= art.x && x < art.x + art.width, "label strays at {x},{y}");
                    }
                }
            }
            assert!(added > 0);
            seen.push(film.bits().to_vec());
        }
        assert!(seen[0] != seen[1] && seen[1] != seen[2] && seen[0] != seen[2]);
    }

    #[test]
    fn long_text_is_cut_to_the_art_width_and_top_works() {
        let (mut film, art, m) = framed(30);
        add_label(&mut film, art, 31, "A VERY LONG INK NAME INDEED", 1, LabelPosition::Top).unwrap();
        for y in 0..art.y {
            for x in 0..film.width() {
                if film.get(x, y) && y > art.y - 8 {
                    assert!(x >= art.x && x < art.x + art.width);
                }
            }
        }
        let _ = m;
    }

    #[test]
    fn a_margin_too_small_is_an_error() {
        let (mut film, art, _) = framed(30);
        assert_eq!(add_label(&mut film, art, 5, "A", 1, LabelPosition::Bottom), Err(ScError::MarksDontFit));
        let huge = Rect { x: usize::MAX, y: usize::MAX, width: usize::MAX, height: usize::MAX };
        assert!(add_label(&mut film, huge, 100, "A", usize::MAX, LabelPosition::Bottom).is_err());
        assert!(add_label(&mut film, huge, 100, "A", 1, LabelPosition::Top).is_err());
    }
}
