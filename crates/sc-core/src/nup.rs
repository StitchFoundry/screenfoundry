//! N-up: repeat one film in a grid of rows and columns on a larger sheet, so small designs
//! waste less film. Each copy is a whole film (pass the output of [`crate::marks::frame_with_marks`]
//! and every copy keeps its registration marks). The grid is centred on the sheet.

use crate::error::{Result, ScError, param};
use crate::image::Bitmap1;
use crate::marks::Rect;

/// Rows, columns and the gap between neighbouring copies, in film pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NupSpec {
    pub rows: usize,
    pub cols: usize,
    pub gap_px: usize,
}

/// Where each copy of a `cell_w` x `cell_h` film goes on a `sheet_w` x `sheet_h` sheet, row by row.
/// The grid is centred; an error if there are no rows or columns, or the grid does not fit.
pub fn nup_layout(sheet_w: usize, sheet_h: usize, cell_w: usize, cell_h: usize, spec: &NupSpec) -> Result<Vec<Rect>> {
    if spec.rows == 0 || spec.cols == 0 {
        return Err(param("rows/cols", "must be at least 1"));
    }
    if cell_w == 0 || cell_h == 0 {
        return Err(ScError::Empty);
    }
    let span = |n: usize, cell: usize| {
        let gaps = spec.gap_px.checked_mul(n - 1)?;
        cell.checked_mul(n)?.checked_add(gaps)
    };
    let (total_w, total_h) = match (span(spec.cols, cell_w), span(spec.rows, cell_h)) {
        (Some(w), Some(h)) if w <= sheet_w && h <= sheet_h => (w, h),
        _ => return Err(ScError::NupDoesntFit),
    };
    let (x0, y0) = ((sheet_w - total_w) / 2, (sheet_h - total_h) / 2);
    let mut out = Vec::with_capacity(spec.rows.saturating_mul(spec.cols).min(4096));
    for r in 0..spec.rows {
        for c in 0..spec.cols {
            out.push(Rect { x: x0 + c * (cell_w + spec.gap_px), y: y0 + r * (cell_h + spec.gap_px), width: cell_w, height: cell_h });
        }
    }
    Ok(out)
}

/// A sheet with `film` repeated per `spec`, centred. Errors if the copies do not fit.
pub fn nup_film(film: &Bitmap1, sheet_w: usize, sheet_h: usize, spec: &NupSpec) -> Result<Bitmap1> {
    let cells = nup_layout(sheet_w, sheet_h, film.width(), film.height(), spec)?;
    let mut sheet = Bitmap1::new(sheet_w, sheet_h)?;
    for cell in cells {
        for y in 0..film.height() {
            for x in 0..film.width() {
                if film.get(x, y) {
                    sheet.set((cell.x + x) as i64, (cell.y + y) as i64, true);
                }
            }
        }
    }
    Ok(sheet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marks::{MarkSpec, frame_with_marks};

    #[test]
    fn two_by_three_gives_six_disjoint_centred_copies() {
        let spec = NupSpec { rows: 2, cols: 3, gap_px: 10 };
        let cells = nup_layout(200, 100, 50, 30, &spec).unwrap();
        assert_eq!(cells.len(), 6);
        // Grid is 170 x 70, so it starts at (15, 15).
        assert_eq!((cells[0].x, cells[0].y), (15, 15));
        assert_eq!((cells[5].x + 50, cells[5].y + 30), (185, 85));
        for (i, a) in cells.iter().enumerate() {
            for b in &cells[i + 1..] {
                let apart = a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn refuses_when_the_copies_do_not_fit() {
        let spec = NupSpec { rows: 2, cols: 3, gap_px: 10 };
        assert_eq!(nup_layout(169, 100, 50, 30, &spec), Err(ScError::NupDoesntFit));
        assert_eq!(nup_layout(200, 69, 50, 30, &spec), Err(ScError::NupDoesntFit));
        assert!(nup_layout(170, 70, 50, 30, &spec).is_ok());
        let huge = NupSpec { rows: usize::MAX, cols: usize::MAX, gap_px: usize::MAX };
        assert_eq!(nup_layout(200, 100, 50, 30, &huge), Err(ScError::NupDoesntFit));
        assert!(nup_layout(200, 100, 50, 30, &NupSpec { rows: 0, cols: 1, gap_px: 0 }).is_err());
        assert!(nup_layout(200, 100, 0, 30, &spec).is_err());
    }

    #[test]
    fn every_copy_keeps_its_marks() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        let framed = frame_with_marks(&Bitmap1::new(30, 20).unwrap(), &m).unwrap();
        let spec = NupSpec { rows: 2, cols: 3, gap_px: 4 };
        let (sw, sh) = (framed.width() * 3 + 20, framed.height() * 2 + 20);
        let sheet = nup_film(&framed, sw, sh, &spec).unwrap();
        let ink = |b: &Bitmap1| b.bits().iter().filter(|v| **v).count();
        assert_eq!(ink(&sheet), 6 * ink(&framed));
        for c in nup_layout(sw, sh, framed.width(), framed.height(), &spec).unwrap() {
            let sub = (0..framed.height()).all(|y| (0..framed.width()).all(|x| sheet.get(c.x + x, c.y + y) == framed.get(x, y)));
            assert!(sub);
        }
    }
}
