//! Registration marks: a crosshair inside a circle at each corner, outside the artwork, so every
//! film carries the same marks and the screens can be aligned on press.
//!
//! The mark size (about 0.25 to 0.5 inch) and four-corner layout follow common shop practice
//! described in public guides; they are parameters, not a standard.

use crate::error::{Result, ScError, param};
use crate::image::Bitmap1;

/// Where the artwork sits on the film, in film pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkSpec {
    /// Outer diameter of the circle (and length of the crosshair), in film pixels.
    pub size_px: usize,
    /// Line thickness in film pixels.
    pub line_px: usize,
    /// Gap between the artwork's corner and the nearest edge of the mark.
    pub gap_px: usize,
}

impl MarkSpec {
    /// A 0.375 inch mark with a 0.01 inch line, 0.25 inch from the artwork, at `dpi`.
    pub fn for_dpi(dpi: f64) -> Result<Self> {
        if !dpi.is_finite() || dpi <= 0.0 || dpi > 20_000.0 {
            return Err(param("dpi", "must be 0 < dpi <= 20000"));
        }
        Ok(Self { size_px: (0.375 * dpi).round() as usize, line_px: ((0.01 * dpi).round() as usize).max(1), gap_px: (0.25 * dpi).round() as usize })
    }
}

/// Centres of the four marks, clockwise from the top left, or an error if any would leave the film.
pub fn corner_centers(film_w: usize, film_h: usize, art: Rect, m: &MarkSpec) -> Result<[(usize, usize); 4]> {
    let off = m.gap_px.checked_add(m.size_px / 2).ok_or(ScError::MarksDontFit)?;
    let (x1, y1) = (art.x.saturating_add(art.width), art.y.saturating_add(art.height));
    let half = m.size_px / 2;
    let reach = off.checked_add(half).ok_or(ScError::MarksDontFit)?;
    let fits = |end: usize, film: usize| end.checked_add(reach).is_some_and(|v| v <= film);
    if art.x < reach || art.y < reach || !fits(x1, film_w) || !fits(y1, film_h) {
        return Err(ScError::MarksDontFit);
    }
    Ok([(art.x - off, art.y - off), (x1 + off, art.y - off), (x1 + off, y1 + off), (art.x - off, y1 + off)])
}

/// Draw a crosshair-in-circle centred at `(cx, cy)`.
pub fn draw_mark(film: &mut Bitmap1, cx: usize, cy: usize, m: &MarkSpec) {
    // i128 so no sum or square can overflow whatever the caller passes; the loops are clipped to the film
    // so a huge centre or size costs no more than the film itself (#87).
    let r = (m.size_px / 2) as i128;
    let t = m.line_px as i128;
    let (cx, cy) = (cx as i128, cy as i128);
    let (w, h) = (film.width() as i128, film.height() as i128);
    let inner = (r - t).max(0);
    for dy in (-r).max(-cy)..=r.min(h - 1 - cy) {
        for dx in (-r).max(-cx)..=r.min(w - 1 - cx) {
            let d2 = dx * dx + dy * dy;
            let ring = d2 <= r * r && d2 >= inner * inner;
            let cross = dx.abs() * 2 < t + 1 || dy.abs() * 2 < t + 1;
            if ring || cross {
                // In range: the loops keep (cx + dx, cy + dy) inside the film, so it fits an i64.
                film.set((cx + dx) as i64, (cy + dy) as i64, true);
            }
        }
    }
}

/// Stamp the four corner marks onto a film.
pub fn add_registration_marks(film: &mut Bitmap1, art: Rect, m: &MarkSpec) -> Result<()> {
    for (cx, cy) in corner_centers(film.width(), film.height(), art, m)? {
        draw_mark(film, cx, cy, m);
    }
    Ok(())
}

/// A copy of `film` on a larger film with the four registration marks around it. The margin is
/// the gap plus the mark's size on every side, so every screen of a job grows identically and the
/// marks line up from one film to the next.
pub fn frame_with_marks(film: &Bitmap1, m: &MarkSpec) -> Result<Bitmap1> {
    let margin = m.gap_px.checked_add(m.size_px).and_then(|v| v.checked_add(1)).ok_or(ScError::MarksDontFit)?;
    let width = film.width().checked_add(margin.saturating_mul(2)).ok_or(ScError::MarksDontFit)?;
    let height = film.height().checked_add(margin.saturating_mul(2)).ok_or(ScError::MarksDontFit)?;
    let mut out = Bitmap1::new(width, height)?;
    for y in 0..film.height() {
        for x in 0..film.width() {
            if film.get(x, y) {
                out.set((x + margin) as i64, (y + margin) as i64, true);
            }
        }
    }
    let art = Rect { x: margin, y: margin, width: film.width(), height: film.height() };
    add_registration_marks(&mut out, art, m)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_centres_and_sizes_never_overflow() {
        let mut film = Bitmap1::new(8, 8).unwrap();
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        draw_mark(&mut film, usize::MAX / 2, usize::MAX / 2, &m);
        draw_mark(&mut film, usize::MAX, 3, &m);
        let huge = MarkSpec { size_px: usize::MAX, line_px: usize::MAX, gap_px: usize::MAX };
        draw_mark(&mut film, 4, 4, &huge);
        let art = Rect { x: usize::MAX - 5, y: 0, width: 10, height: 10 };
        assert!(corner_centers(8, 8, art, &huge).is_err());
        assert!(corner_centers(8, 8, art, &m).is_err());
    }

    #[test]
    fn marks_land_at_each_corner_and_are_identical() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        let art = Rect { x: 60, y: 60, width: 100, height: 80 };
        let mut film = Bitmap1::new(220, 200).unwrap();
        add_registration_marks(&mut film, art, &m).unwrap();
        let cs = corner_centers(220, 200, art, &m).unwrap();
        assert_eq!(cs[0], (40, 40));
        assert_eq!(cs[2], (180, 160));
        for (cx, cy) in cs {
            assert!(film.get(cx, cy), "centre of mark at {cx},{cy}");
            assert!(film.get(cx + 10, cy) || film.get(cx + 9, cy), "ring");
        }
        // Nothing inked inside the artwork area.
        for y in 60..140 {
            for x in 60..160 {
                assert!(!film.get(x, y));
            }
        }
    }

    #[test]
    fn marks_that_do_not_fit_are_an_error_not_a_panic() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        let art = Rect { x: 5, y: 5, width: 50, height: 50 };
        let mut film = Bitmap1::new(60, 60).unwrap();
        assert_eq!(add_registration_marks(&mut film, art, &m), Err(ScError::MarksDontFit));
        let huge = Rect { x: usize::MAX, y: 0, width: usize::MAX, height: 1 };
        assert!(corner_centers(10, 10, huge, &m).is_err());
    }

    #[test]
    fn framing_keeps_the_art_and_adds_identical_margins() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        let mut a = Bitmap1::new(30, 20).unwrap();
        a.set(3, 4, true);
        let b = Bitmap1::new(30, 20).unwrap();
        let (fa, fb) = (frame_with_marks(&a, &m).unwrap(), frame_with_marks(&b, &m).unwrap());
        assert_eq!((fa.width(), fa.height()), (fb.width(), fb.height()));
        assert_eq!(fa.width(), 30 + 2 * 31);
        assert!(fa.get(3 + 31, 4 + 31), "art is carried over, offset by the margin");
        // The marks are the only difference between an empty film and a film with one dot.
        let diff = fa.bits().iter().zip(fb.bits()).filter(|(x, y)| x != y).count();
        assert_eq!(diff, 1);
        assert!(frame_with_marks(&a, &MarkSpec { size_px: usize::MAX, line_px: 1, gap_px: 1 }).is_err());
    }

    #[test]
    fn spec_from_dpi() {
        let m = MarkSpec::for_dpi(1200.0).unwrap();
        assert_eq!((m.size_px, m.line_px, m.gap_px), (450, 12, 300));
        assert!(MarkSpec::for_dpi(0.0).is_err());
        assert!(MarkSpec::for_dpi(f64::NAN).is_err());
    }

    #[test]
    fn marks_that_just_fit_are_accepted_and_one_pixel_less_is_not() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        // Each mark needs gap + size/2 + size/2 = 30 px between the art and the film edge.
        let art = Rect { x: 30, y: 30, width: 10, height: 10 };
        assert!(corner_centers(70, 70, art, &m).is_ok());
        assert_eq!(corner_centers(69, 70, art, &m), Err(ScError::MarksDontFit));
        assert_eq!(corner_centers(70, 69, art, &m), Err(ScError::MarksDontFit));
        let tight = Rect { x: 29, ..art };
        assert_eq!(corner_centers(70, 70, tight, &m), Err(ScError::MarksDontFit));
    }

    #[test]
    fn zero_sized_art_and_empty_film_are_errors_not_panics() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        // A film with no pixels cannot exist, so there is nothing to stamp marks on.
        assert!(Bitmap1::new(0, 0).is_err());
        let art = Rect { x: 0, y: 0, width: 0, height: 0 };
        assert_eq!(corner_centers(0, 0, art, &m), Err(ScError::MarksDontFit));
        // A 0x0 artwork in the middle of a big enough film still gets four marks.
        let mut film = Bitmap1::new(100, 100).unwrap();
        let mid = Rect { x: 50, y: 50, width: 0, height: 0 };
        assert!(add_registration_marks(&mut film, mid, &m).is_ok());
    }

    #[test]
    fn drawing_a_mark_at_the_film_edge_clips_instead_of_panicking() {
        let m = MarkSpec { size_px: 20, line_px: 2, gap_px: 10 };
        let mut film = Bitmap1::new(8, 8).unwrap();
        draw_mark(&mut film, 0, 0, &m);
        draw_mark(&mut film, 7, 7, &m);
        draw_mark(&mut film, 1 << 40, 1 << 40, &m);
        assert!(film.get(0, 0));
    }

    #[test]
    fn spec_from_dpi_rejects_out_of_range_and_never_gives_a_zero_line() {
        assert!(MarkSpec::for_dpi(-300.0).is_err());
        assert!(MarkSpec::for_dpi(f64::INFINITY).is_err());
        assert!(MarkSpec::for_dpi(20_000.1).is_err());
        let low = MarkSpec::for_dpi(10.0).unwrap();
        assert_eq!(low.line_px, 1);
        assert!(MarkSpec::for_dpi(20_000.0).is_ok());
    }
}
