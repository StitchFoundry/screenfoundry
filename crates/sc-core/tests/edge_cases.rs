#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Malformed and extreme inputs to the image containers and the separation entry points.

use sc_core::image::MAX_PIXELS;
use sc_core::separate::{extract_palette, separate_spot, tone_plate};
use sc_core::{Gray8, Rgba8, ScError};

fn solid(w: usize, h: usize, px: [u8; 4]) -> Rgba8 {
    Rgba8::new(w, h, px.iter().copied().cycle().take(w * h * 4).collect()).unwrap()
}

#[test]
fn constructors_reject_empty_images() {
    assert!(matches!(Rgba8::new(0, 5, vec![]), Err(ScError::Empty)));
    assert!(matches!(Rgba8::new(5, 0, vec![]), Err(ScError::Empty)));
    assert!(matches!(Gray8::new(0, 0, vec![]), Err(ScError::Empty)));
}

#[test]
fn constructors_reject_wrong_buffer_length() {
    assert!(matches!(Rgba8::new(2, 2, vec![0; 15]), Err(ScError::BadBuffer { .. })));
    assert!(matches!(Rgba8::new(2, 2, vec![0; 17]), Err(ScError::BadBuffer { .. })));
    assert!(matches!(Gray8::new(3, 3, vec![0; 8]), Err(ScError::BadBuffer { .. })));
}

#[test]
fn constructors_reject_oversized_and_overflowing_dimensions() {
    // Rejected on size before the (tiny) buffer is even compared.
    assert!(matches!(Rgba8::new(MAX_PIXELS + 1, 1, vec![]), Err(ScError::TooLarge { .. })));
    assert!(matches!(Rgba8::new(usize::MAX, 2, vec![]), Err(ScError::TooLarge { .. })));
    assert!(matches!(Gray8::new(usize::MAX, usize::MAX, vec![]), Err(ScError::TooLarge { .. })));
}

#[test]
fn pixel_lookup_is_bounds_checked() {
    let img = solid(2, 2, [1, 2, 3, 255]);
    assert_eq!(img.pixel(1, 1), Some([1, 2, 3, 255]));
    assert_eq!(img.pixel(2, 0), None);
    assert_eq!(img.pixel(0, 2), None);
    assert_eq!(img.pixel(usize::MAX, usize::MAX), None);
}

#[test]
fn palette_size_bounds() {
    let img = solid(4, 4, [200, 10, 10, 255]);
    assert!(extract_palette(&img, 0).is_err());
    assert!(extract_palette(&img, 65).is_err());
    assert_eq!(extract_palette(&img, 1).unwrap().len(), 1);
    // More colours requested than exist: fewer come back, not an error.
    assert!(extract_palette(&img, 64).unwrap().len() <= 64);
}

#[test]
fn fully_transparent_art_has_no_palette_and_empty_plates() {
    let img = solid(3, 3, [255, 0, 0, 0]);
    assert!(extract_palette(&img, 4).unwrap().is_empty());
    let plates = separate_spot(&img, &[[255, 0, 0], [0, 0, 255]]).unwrap();
    assert_eq!(plates.len(), 2);
    assert!(plates.iter().all(|p| p.data().iter().all(|&v| v == 0)));
    assert!(tone_plate(&img, [0, 0, 0]).unwrap().data().iter().all(|&v| v == 0));
}

#[test]
fn spot_separation_ink_count_bounds() {
    let img = solid(2, 2, [10, 20, 30, 255]);
    assert!(separate_spot(&img, &[]).is_err());
    assert!(separate_spot(&img, &[[0, 0, 0]; 65]).is_err());
    assert_eq!(separate_spot(&img, &[[0, 0, 0]; 64]).unwrap().len(), 64);
}

#[test]
fn duplicate_inks_still_partition_pixels() {
    let img = solid(2, 2, [250, 0, 0, 255]);
    let plates = separate_spot(&img, &[[255, 0, 0], [255, 0, 0]]).unwrap();
    for i in 0..4 {
        let on = plates.iter().filter(|p| p.data()[i] == 255).count();
        assert_eq!(on, 1, "pixel {i} must land on exactly one plate");
    }
}

#[test]
fn alpha_cutoff_boundary() {
    let below = solid(1, 1, [0, 0, 0, 127]);
    let at = solid(1, 1, [0, 0, 0, 128]);
    let ink = [[0, 0, 0]];
    assert_eq!(separate_spot(&below, &ink).unwrap()[0].data(), &[0]);
    assert_eq!(separate_spot(&at, &ink).unwrap()[0].data(), &[255]);
}

#[test]
fn tone_plate_rejects_white_ink() {
    let img = solid(2, 2, [0, 0, 0, 255]);
    assert!(tone_plate(&img, [255, 255, 255]).is_err());
    // Near-white is also indistinguishable.
    assert!(tone_plate(&img, [255, 255, 254]).is_err());
}

#[test]
fn single_pixel_image_separates() {
    let img = solid(1, 1, [0, 128, 0, 255]);
    assert_eq!(extract_palette(&img, 3).unwrap().len(), 1);
    assert_eq!(tone_plate(&img, [0, 128, 0]).unwrap().data().len(), 1);
}
