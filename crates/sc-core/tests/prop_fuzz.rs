#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property tests: random sizes, pixel data and extreme numbers thrown at the entry points.
//! Each call must return Ok or Err (never panic), and every plate or film it returns must keep
//! the size of the input.

use proptest::prelude::*;
use sc_core::fm::{FmOptions, Hybrid, Kernel, fm_screen, hybrid_screen};
use sc_core::halftone::{DotShape, HalftoneOptions, halftone};
use sc_core::index::index_films;
use sc_core::marks::{MarkSpec, Rect, add_registration_marks, corner_centers, draw_mark, frame_with_marks};
use sc_core::separate::{extract_palette, separate_spot, tone_plate};
use sc_core::simulated::simulated_plates;
use sc_core::{Bitmap1, Gray8, Rgba8};

/// Ordinary and hostile floats: NaN, infinities, zero, negatives, huge and tiny values.
fn any_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => -2.0f64..2.0,
        2 => 0.0f64..1.0,
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => Just(f64::MAX),
        1 => Just(f64::MIN_POSITIVE),
        1 => -1.0e9f64..1.0e9,
    ]
}

fn rgba() -> impl Strategy<Value = Rgba8> {
    (1usize..=20, 1usize..=20).prop_flat_map(|(w, h)| prop::collection::vec(any::<u8>(), w * h * 4).prop_map(move |d| Rgba8::new(w, h, d).unwrap()))
}

fn gray() -> impl Strategy<Value = Gray8> {
    (1usize..=24, 1usize..=24).prop_flat_map(|(w, h)| prop::collection::vec(any::<u8>(), w * h).prop_map(move |d| Gray8::new(w, h, d).unwrap()))
}

fn inks() -> impl Strategy<Value = Vec<[u8; 3]>> {
    prop::collection::vec(any::<[u8; 3]>(), 0..8)
}

fn kernel() -> impl Strategy<Value = Kernel> {
    prop_oneof![Just(Kernel::FloydSteinberg), Just(Kernel::Jarvis), Just(Kernel::Stucki)]
}

fn shape() -> impl Strategy<Value = DotShape> {
    prop_oneof![Just(DotShape::Round), Just(DotShape::Ellipse), Just(DotShape::Line)]
}

fn ht_opts() -> impl Strategy<Value = HalftoneOptions> {
    (any_f64(), any_f64(), any_f64(), shape(), any_f64(), any_f64(), any_f64()).prop_map(
        |(dpi, lpi, angle_deg, shape, min_dot, max_dot, dot_gain)| HalftoneOptions { dpi, lpi, angle_deg, shape, min_dot, max_dot, dot_gain },
    )
}

fn fm_opts() -> impl Strategy<Value = FmOptions> {
    (0usize..100, any_f64(), any_f64(), any_f64(), kernel()).prop_map(|(cell_px, min_dot, max_dot, dot_gain, kernel)| FmOptions {
        cell_px,
        min_dot,
        max_dot,
        dot_gain,
        kernel,
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn separation_never_panics_and_keeps_size(img in rgba(), k in 0usize..12, ink_list in inks(), ink in any::<[u8; 3]>()) {
        let _ = extract_palette(&img, k);
        if let Ok(plates) = separate_spot(&img, &ink_list) {
            for p in plates {
                prop_assert_eq!((p.width(), p.height()), (img.width(), img.height()));
            }
        }
        if let Ok(p) = tone_plate(&img, ink) {
            prop_assert_eq!((p.width(), p.height()), (img.width(), img.height()));
        }
    }

    #[test]
    fn simulated_never_panics_and_keeps_size(img in rgba(), ink_list in inks(), base in any::<[u8; 3]>(), limit in any_f64()) {
        if let Ok(plates) = simulated_plates(&img, &ink_list, base, limit) {
            for p in plates {
                prop_assert_eq!((p.width(), p.height()), (img.width(), img.height()));
            }
        }
    }

    #[test]
    fn halftone_never_panics_and_keeps_size(plate in gray(), o in ht_opts()) {
        if let Ok(f) = halftone(&plate, &o) {
            prop_assert_eq!((f.width(), f.height()), (plate.width(), plate.height()));
        }
    }

    #[test]
    fn fm_and_hybrid_never_panic_and_keep_size(plate in gray(), o in fm_opts(), am in ht_opts(), below in any_f64(), above in any_f64()) {
        if let Ok(f) = fm_screen(&plate, &o) {
            prop_assert_eq!((f.width(), f.height()), (plate.width(), plate.height()));
        }
        if let Ok(f) = hybrid_screen(&plate, &am, &o, &Hybrid { below, above }) {
            prop_assert_eq!((f.width(), f.height()), (plate.width(), plate.height()));
        }
    }

    #[test]
    fn index_never_panics_and_keeps_size(img in rgba(), ink_list in inks(), base in any::<[u8; 3]>(), cell in 0usize..100, dither in any_f64(), k in kernel(), ordered in any::<bool>()) {
        if let Ok(films) = index_films(&img, &ink_list, base, cell, dither, k, ordered) {
            for f in films {
                prop_assert_eq!((f.width(), f.height()), (img.width(), img.height()));
            }
        }
    }

    #[test]
    fn marks_never_panic(
        fw in 1usize..64, fh in 1usize..64,
        x in any::<usize>(), y in any::<usize>(), w in any::<usize>(), h in any::<usize>(),
        size_px in any::<usize>(), line_px in any::<usize>(), gap_px in any::<usize>(),
        cx in any::<usize>(), cy in any::<usize>(), dpi in any_f64(),
    ) {
        let art = Rect { x, y, width: w, height: h };
        let m = MarkSpec { size_px: size_px % 200, line_px, gap_px: gap_px % 200 };
        // Hostile values on the fields that must be tolerated whatever their size.
        let wild = MarkSpec { size_px, line_px, gap_px };
        let mut film = Bitmap1::new(fw, fh).unwrap();
        draw_mark(&mut film, cx, cy, &MarkSpec { size_px: size_px % 4096, line_px: line_px % 4096, gap_px });
        let _ = corner_centers(fw, fh, art, &wild);
        let _ = add_registration_marks(&mut film, art, &m);
        prop_assert_eq!((film.width(), film.height()), (fw, fh));
        if let Ok(spec) = MarkSpec::for_dpi(dpi) {
            let small = Bitmap1::new(fw, fh).unwrap();
            let _ = spec.size_px;
            if spec.size_px < 500 {
                let _ = frame_with_marks(&small, &spec);
            }
        }
    }
}
