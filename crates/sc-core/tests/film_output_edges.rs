#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Edge inputs to the film and file writers: `export`, `dtf` and `dcs`. Every bad input must come
//! back as an error (never a panic), and the extreme but valid ones must give a well-formed file.

use sc_core::dcs::{self, MAX_PLATES, Plate};
use sc_core::dtf::{self, DtfOptions, MAX_DESIGNS};
use sc_core::export;
use sc_core::{Bitmap1, Rgba8};

fn film(w: usize, h: usize) -> Bitmap1 {
    Bitmap1::from_bits(w, h, (0..w * h).map(|i| i % 3 == 0).collect()).unwrap()
}

fn solid(w: usize, h: usize) -> Rgba8 {
    Rgba8::new(w, h, [10u8, 20, 30, 255].iter().copied().cycle().take(w * h * 4).collect()).unwrap()
}

const BAD_DPI: [f64; 7] = [0.0, -300.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, export::MAX_DPI + 1.0, 1e300];

#[test]
fn every_film_writer_rejects_bad_dpi() {
    let f = film(8, 8);
    let img = solid(4, 4);
    for dpi in BAD_DPI {
        assert!(export::png_1bit(&f, dpi).is_err(), "png_1bit {dpi}");
        assert!(export::png_rgba(&img, dpi).is_err(), "png_rgba {dpi}");
        assert!(export::tiff_rgba(&img, dpi).is_err(), "tiff_rgba {dpi}");
        assert!(export::tiff_1bit(&f, dpi).is_err(), "tiff_1bit {dpi}");
        assert!(export::tiff_g4(&f, dpi).is_err(), "tiff_g4 {dpi}");
        assert!(export::pdf_g4(&f, dpi).is_err(), "pdf_g4 {dpi}");
    }
}

#[test]
fn dpi_at_the_limits_is_accepted() {
    let f = film(8, 8);
    assert!(export::png_1bit(&f, export::MAX_DPI).is_ok());
    assert!(export::tiff_g4(&f, 0.001).is_ok());
}

#[test]
fn one_pixel_film_writes_every_format() {
    for ink in [false, true] {
        let f = Bitmap1::from_bits(1, 1, vec![ink]).unwrap();
        let png = export::png_1bit(&f, 300.0).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        assert!(export::tiff_1bit(&f, 300.0).unwrap().starts_with(b"II"));
        assert!(export::tiff_g4(&f, 300.0).unwrap().starts_with(b"II"));
        assert!(export::pdf_g4(&f, 300.0).unwrap().starts_with(b"%PDF"));
    }
}

#[test]
fn very_wide_and_very_tall_films_write() {
    for (w, h) in [(5000, 1), (1, 5000), (65_535, 2)] {
        let f = Bitmap1::new(w, h).unwrap();
        assert!(export::png_1bit(&f, 300.0).is_ok(), "png {w}x{h}");
        assert!(export::tiff_1bit(&f, 300.0).is_ok(), "tiff {w}x{h}");
        assert!(export::tiff_g4(&f, 300.0).is_ok(), "g4 {w}x{h}");
    }
}

#[test]
fn group4_refuses_a_film_wider_than_65535() {
    let f = Bitmap1::new(65_536, 1).unwrap();
    assert!(export::tiff_g4(&f, 300.0).is_err());
    assert!(export::pdf_g4(&f, 300.0).is_err());
    // The other writers have no such limit.
    assert!(export::tiff_1bit(&f, 300.0).is_ok());
}

#[test]
fn pdf_refuses_a_page_over_200_inches() {
    let f = Bitmap1::new(300, 4000).unwrap(); // 4000 px at 1 dpi is far over 200 in
    assert!(export::pdf_g4(&f, 1.0).is_err());
    assert!(export::pdf_g4(&f, 300.0).is_ok());
}

#[test]
fn gang_sheet_rejects_no_designs_too_many_and_zero_width() {
    assert!(dtf::gang_sheet(&[], 100, 2, false).is_err());
    let many = vec![solid(1, 1); MAX_DESIGNS + 1];
    assert!(dtf::gang_sheet(&many, 1000, 0, false).is_err());
    assert!(dtf::gang_sheet(&[solid(2, 2)], 0, 0, false).is_err());
}

#[test]
fn gang_sheet_where_nothing_fits_is_an_error() {
    let d = [solid(50, 40), solid(60, 45)];
    assert!(dtf::gang_sheet(&d, 30, 2, false).is_err());
    // Rotating cannot help when both sides are wider than the sheet.
    assert!(dtf::gang_sheet(&d, 30, 2, true).is_err());
}

#[test]
fn gang_sheet_turns_a_too_wide_design_when_allowed() {
    let d = [solid(50, 10)];
    assert!(dtf::gang_sheet(&d, 20, 0, false).is_err());
    let (sheet, placed) = dtf::gang_sheet(&d, 20, 0, true).unwrap();
    assert_eq!((sheet.width(), sheet.height()), (20, 50));
    assert!(placed[0].rotated);
}

#[test]
fn gang_sheet_with_a_huge_gap_is_an_error_not_an_overflow() {
    let d = [solid(4, 4), solid(4, 4), solid(4, 4)];
    assert!(dtf::gang_sheet(&d, 4, usize::MAX, false).is_err());
    assert!(dtf::gang_sheet(&d, 4, usize::MAX / 2, true).is_err());
}

#[test]
fn gang_sheet_with_a_one_pixel_sheet_stacks_one_pixel_designs() {
    let d = [solid(1, 1), solid(1, 1)];
    let (sheet, placed) = dtf::gang_sheet(&d, 1, 1, false).unwrap();
    assert_eq!(sheet.width(), 1);
    assert_eq!(sheet.height(), 3);
    assert_eq!(placed.len(), 2);
}

#[test]
fn dtf_prepare_rejects_bad_chokes_and_handles_one_pixel_art() {
    let img = solid(1, 1);
    for choke in [f64::NAN, -1.0, f64::INFINITY] {
        let o = DtfOptions { mirror: true, white: true, white_choke_px: choke, graded: false };
        assert!(dtf::prepare(&img, &o).is_err(), "choke {choke}");
    }
    // The choke is not looked at when no white is asked for.
    let o = DtfOptions { mirror: true, white: false, white_choke_px: f64::NAN, graded: false };
    assert!(dtf::prepare(&img, &o).unwrap().white.is_none());
    let o = DtfOptions { mirror: true, white: true, white_choke_px: 0.0, graded: true };
    let files = dtf::prepare(&img, &o).unwrap();
    assert_eq!(files.white.unwrap().pixel(0, 0), Some([255, 255, 255, 255]));
}

fn plate<'a>(name: &'a str, film: &'a Bitmap1) -> Plate<'a> {
    Plate { name, rgb: [0, 0, 0], cmyk: None, film }
}

#[test]
fn dcs_rejects_no_plates_and_too_many() {
    assert!(dcs::single_file(&[], 300.0, "t", 72.0).is_err());
    let f = film(8, 8);
    let many: Vec<Plate> = (0..=MAX_PLATES).map(|_| plate("Ink", &f)).collect();
    assert!(dcs::single_file(&many, 300.0, "t", 72.0).is_err());
    let names: Vec<String> = (0..many.len()).map(|i| format!("p{i}.eps")).collect();
    assert!(dcs::multiple_files(&many, 300.0, "t", &names, 72.0).is_err());
    assert!(dcs::multiple_files(&[], 300.0, "t", &[], 72.0).is_err());
}

#[test]
fn dcs_accepts_exactly_the_most_plates() {
    let f = film(8, 8);
    let ok: Vec<Plate> = (0..MAX_PLATES).map(|_| plate("Ink", &f)).collect();
    assert!(dcs::single_file(&ok, 300.0, "t", 72.0).is_ok());
}

#[test]
fn dcs_rejects_bad_dpi_and_composite_dpi_and_mismatched_plates() {
    let (a, b) = (film(8, 8), film(8, 9));
    let one = [plate("Ink", &a)];
    for dpi in BAD_DPI {
        assert!(dcs::single_file(&one, dpi, "t", 72.0).is_err(), "dpi {dpi}");
    }
    for c in [0.0, -72.0, f64::NAN, f64::INFINITY] {
        assert!(dcs::single_file(&one, 300.0, "t", c).is_err(), "composite {c}");
    }
    assert!(dcs::single_file(&[plate("A", &a), plate("B", &b)], 300.0, "t", 72.0).is_err());
}

#[test]
fn dcs_multiple_files_checks_the_file_names() {
    let f = film(8, 8);
    let two = [plate("A", &f), plate("B", &f)];
    let n = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(dcs::multiple_files(&two, 300.0, "t", &n(&["a.eps"]), 72.0).is_err());
    assert!(dcs::multiple_files(&two, 300.0, "t", &n(&["a.eps", ""]), 72.0).is_err());
    assert!(dcs::multiple_files(&two, 300.0, "t", &n(&["a.eps", "../b.eps"]), 72.0).is_err());
    assert!(dcs::multiple_files(&two, 300.0, "t", &n(&["a.eps", "b c.eps"]), 72.0).is_err());
    assert!(dcs::multiple_files(&two, 300.0, "t", &n(&["a.eps", "b.eps"]), 72.0).is_ok());
}

#[test]
fn dcs_survives_one_pixel_plates_and_odd_names() {
    let f = Bitmap1::from_bits(1, 1, vec![true]).unwrap();
    let hostile = "bad (name)\\\n\u{1F600}";
    let out = dcs::single_file(&[plate(hostile, &f), plate("", &f)], 300.0, "t\n(x)", 72.0).unwrap();
    assert!(out.starts_with(b"%!PS-Adobe"));
    // No stray line break or unbalanced parenthesis got into a comment from the name.
    let text = String::from_utf8_lossy(&out);
    assert!(!text.contains("bad (name)"));
}

#[test]
fn dcs_composite_too_large_is_refused() {
    let f = Bitmap1::new(9_000, 9_000).unwrap(); // 81 Mpx: a 243 MB composite at full size
    assert!(dcs::single_file(&[plate("Ink", &f)], 20_000.0, "t", 20_000.0).is_err());
    // A coarser composite is fine.
    assert!(dcs::single_file(&[plate("Ink", &f)], 20_000.0, "t", 72.0).is_ok());
}
