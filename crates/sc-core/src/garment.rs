//! Starting a job from what is being printed on: the item, its colour and where the print goes.
//!
//! A screen printer begins with the garment, not with a page. This module holds the rules of
//! thumb that follow from that choice: the largest print area for each item and location, whether
//! a garment colour wants a white underbase, and the outline of a plain mockup to draw behind the
//! art. Everything here is an editable default (see `docs/research/garment-start.md`), not a
//! fact, and all the artwork is original: simple polygons made in code.
//!
//! Sizes are inches, measured from the print area's top left corner; the mockup is placed around
//! the print area so the area sits where the print would go.

use crate::color::srgb_to_lab;
use serde::{Deserialize, Serialize};

/// What is being printed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Garment {
    TShirt,
    Hoodie,
    Tote,
    /// Paper or board.
    Poster,
    /// Anything else, at a size the user gives.
    Other,
}

/// Where the print goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    LeftChest,
    FullFront,
    FullBack,
    Sleeve,
    Oversized,
    Custom,
}

impl Garment {
    pub const ALL: [Garment; 5] = [Garment::TShirt, Garment::Hoodie, Garment::Tote, Garment::Poster, Garment::Other];

    pub fn id(self) -> &'static str {
        match self {
            Garment::TShirt => "tshirt",
            Garment::Hoodie => "hoodie",
            Garment::Tote => "tote",
            Garment::Poster => "poster",
            Garment::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Garment::TShirt => "T-shirt",
            Garment::Hoodie => "Hoodie / sweatshirt",
            Garment::Tote => "Tote bag",
            Garment::Poster => "Poster / paper",
            Garment::Other => "Other (custom size)",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        Self::ALL.into_iter().find(|g| g.id().eq_ignore_ascii_case(s) || g.label().eq_ignore_ascii_case(s))
    }

    /// The locations offered for this item, in the order the picker shows them.
    pub fn placements(self) -> &'static [Placement] {
        match self {
            Garment::TShirt | Garment::Hoodie => {
                &[Placement::LeftChest, Placement::FullFront, Placement::FullBack, Placement::Sleeve, Placement::Oversized, Placement::Custom]
            }
            Garment::Tote => &[Placement::FullFront, Placement::Oversized, Placement::Custom],
            Garment::Poster | Garment::Other => &[Placement::Custom],
        }
    }

    /// The location chosen first for this item.
    pub fn default_placement(self) -> Placement {
        self.placements().first().copied().unwrap_or(Placement::Custom)
    }

    /// Does the item have a garment colour to pick (paper has the colour of the stock)?
    pub fn is_wearable(self) -> bool {
        matches!(self, Garment::TShirt | Garment::Hoodie)
    }
}

impl Placement {
    pub const ALL: [Placement; 6] =
        [Placement::LeftChest, Placement::FullFront, Placement::FullBack, Placement::Sleeve, Placement::Oversized, Placement::Custom];

    pub fn id(self) -> &'static str {
        match self {
            Placement::LeftChest => "leftChest",
            Placement::FullFront => "fullFront",
            Placement::FullBack => "fullBack",
            Placement::Sleeve => "sleeve",
            Placement::Oversized => "oversized",
            Placement::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Placement::LeftChest => "Left chest",
            Placement::FullFront => "Full front",
            Placement::FullBack => "Full back",
            Placement::Sleeve => "Sleeve",
            Placement::Oversized => "Oversized",
            Placement::Custom => "Custom size",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        Self::ALL.into_iter().find(|p| p.id().eq_ignore_ascii_case(s) || p.label().eq_ignore_ascii_case(s))
    }
}

/// The largest print area, width by height in inches, for an item at a location: a rule of thumb
/// the user can change (and must, for [`Placement::Custom`], where it is only a starting size).
pub fn print_area(garment: Garment, placement: Placement) -> (f64, f64) {
    match (garment, placement) {
        (Garment::TShirt | Garment::Hoodie, Placement::LeftChest) => (4.0, 4.0),
        (Garment::TShirt, Placement::FullFront | Placement::FullBack) => (12.0, 14.0),
        (Garment::Hoodie, Placement::FullFront | Placement::FullBack) => (11.0, 13.0),
        (Garment::TShirt | Garment::Hoodie, Placement::Sleeve) => (3.5, 3.5),
        (Garment::TShirt | Garment::Hoodie, Placement::Oversized) => (16.0, 20.0),
        (Garment::Tote, Placement::Oversized) => (12.0, 14.0),
        (Garment::Tote, _) => (10.0, 12.0),
        (Garment::Poster, _) => (18.0, 24.0),
        _ => (12.0, 12.0),
    }
}

/// Garments lighter than this (CIE L*) print without a white underbase; darker ones get one.
/// A rule of thumb: red and mid green or blue shirts sit close to the line, and the user can
/// switch the underbase either way.
pub const DARK_BELOW_L: f64 = 60.0;

/// Does a garment of this colour want a white underbase by default?
pub fn wants_underbase(rgb: [u8; 3]) -> bool {
    srgb_to_lab(rgb).first().is_some_and(|l| *l < DARK_BELOW_L)
}

/// The one plain line that says why the underbase is on or off.
pub fn underbase_reason(rgb: [u8; 3]) -> &'static str {
    if wants_underbase(rgb) {
        "Dark shirt: a white underbase prints first so colours stay bright."
    } else {
        "Light shirt: no underbase needed, the colours print straight onto the fabric."
    }
}

/// Garment colours offered as swatches: our own names for common shirt colours.
pub const SWATCHES: [(&str, [u8; 3]); 12] = [
    ("White", [255, 255, 255]),
    ("Natural", [238, 230, 208]),
    ("Light grey", [200, 200, 205]),
    ("Yellow", [250, 220, 70]),
    ("Light blue", [150, 195, 230]),
    ("Pink", [240, 170, 190]),
    ("Red", [200, 30, 40]),
    ("Royal blue", [30, 70, 170]),
    ("Green", [30, 120, 70]),
    ("Charcoal", [70, 72, 78]),
    ("Navy", [25, 35, 70]),
    ("Black", [20, 20, 22]),
];

/// A closed outline in inches.
pub type Outline = Vec<[f64; 2]>;

/// The plain garment drawing behind the art, in inches measured from the print area's top left
/// corner (so parts of it have negative coordinates). `body` is drawn first, in the garment
/// colour; `extras` (a hood, tote straps) follow in the same colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Mockup {
    pub body: Outline,
    pub extras: Vec<Outline>,
}

/// A mirror of `pts` on the garment's centre line `cx`, the points reversed so the outline keeps
/// its direction.
fn mirrored(pts: &[[f64; 2]], cx: f64) -> Outline {
    pts.iter().rev().map(|p| [2.0 * cx - p[0], p[1]]).collect()
}

/// The T-shirt outline in garment inches (centre line at x = 15, collar at the top).
fn shirt(back: bool) -> Outline {
    let neck_dip = if back { 1.0 } else { 2.8 };
    let left = [[11.0, 0.0], [4.0, 3.0], [0.0, 12.0], [5.5, 14.0], [5.0, 30.0]];
    let mut o: Outline = left.to_vec();
    o.push([25.0, 30.0]);
    o.extend(mirrored(&left, 15.0).into_iter().skip(1));
    // Neckline, right to left.
    o.extend([[17.0, neck_dip * 0.8], [15.0, neck_dip], [13.0, neck_dip * 0.8]]);
    o
}

impl Mockup {
    /// The mockup for an item and location whose print area is `area` (width, height) inches.
    pub fn new(garment: Garment, placement: Placement, area: (f64, f64)) -> Mockup {
        let (aw, ah) = area;
        match garment {
            Garment::TShirt | Garment::Hoodie => {
                let back = placement == Placement::FullBack;
                // Where the area's top left corner sits on the drawing.
                let (cx, top) = match placement {
                    Placement::LeftChest => (19.5, 5.0),
                    Placement::Sleeve => (3.5, 6.75),
                    _ => (15.0, 3.5),
                };
                let (ox, oy) = (cx - aw / 2.0, top);
                let shift = |o: Outline| -> Outline { o.into_iter().map(|p| [p[0] - ox, p[1] - oy]).collect() };
                let extras = if garment == Garment::Hoodie {
                    vec![shift(vec![[10.5, 1.0], [11.5, -4.0], [15.0, -6.5], [18.5, -4.0], [19.5, 1.0], [15.0, 2.0]])]
                } else {
                    vec![]
                };
                Mockup { body: shift(shirt(back)), extras }
            }
            Garment::Tote => {
                // Body a few inches wider and taller than the print, with two straps.
                let (bw, bh) = (aw + 6.0, ah + 8.0);
                let (x0, y0) = (-3.0, -4.0);
                let body = vec![[x0, y0], [x0 + bw, y0], [x0 + bw, y0 + bh], [x0, y0 + bh]];
                let strap = |x1: f64, x2: f64| {
                    vec![
                        [x1, y0],
                        [x1, y0 - 6.0],
                        [x2, y0 - 6.0],
                        [x2, y0],
                        [x2 - 1.0, y0],
                        [x2 - 1.0, y0 - 5.0],
                        [x1 + 1.0, y0 - 5.0],
                        [x1 + 1.0, y0],
                    ]
                };
                Mockup { body, extras: vec![strap(x0 + bw * 0.2, x0 + bw * 0.35), strap(x0 + bw * 0.65, x0 + bw * 0.8)] }
            }
            Garment::Poster | Garment::Other => {
                let m = 1.5;
                Mockup { body: vec![[-m, -m], [aw + m, -m], [aw + m, ah + m], [-m, ah + m]], extras: vec![] }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_garments_want_an_underbase_and_light_ones_do_not() {
        for (name, rgb) in SWATCHES {
            let dark = matches!(name, "Red" | "Royal blue" | "Green" | "Charcoal" | "Navy" | "Black");
            assert_eq!(wants_underbase(rgb), dark, "{name}");
        }
        assert!(underbase_reason([0, 0, 0]).starts_with("Dark shirt"));
        assert!(underbase_reason([255, 255, 255]).starts_with("Light shirt"));
    }

    #[test]
    fn every_offered_location_has_a_usable_area() {
        for g in Garment::ALL {
            assert!(!g.placements().is_empty());
            assert!(g.placements().contains(&g.default_placement()));
            for p in g.placements() {
                let (w, h) = print_area(g, *p);
                assert!(w > 0.0 && h > 0.0, "{g:?} {p:?}");
            }
        }
        assert_eq!(print_area(Garment::TShirt, Placement::LeftChest), (4.0, 4.0));
        assert_eq!(print_area(Garment::TShirt, Placement::FullFront), (12.0, 14.0));
    }

    #[test]
    fn names_parse_both_ways() {
        for g in Garment::ALL {
            assert_eq!(Garment::parse(g.id()), Some(g));
            assert_eq!(Garment::parse(g.label()), Some(g));
        }
        for p in Placement::ALL {
            assert_eq!(Placement::parse(p.id()), Some(p));
            assert_eq!(Placement::parse(p.label()), Some(p));
        }
        assert_eq!(Garment::parse("jumper"), None);
    }

    /// The print area has to sit on the drawing: inside the body for the chest and back prints.
    #[test]
    fn the_print_area_sits_on_the_garment_drawing() {
        for g in [Garment::TShirt, Garment::Hoodie] {
            for p in [Placement::LeftChest, Placement::FullFront, Placement::FullBack, Placement::Oversized] {
                let (w, h) = print_area(g, p);
                let m = Mockup::new(g, p, (w, h));
                let (min_x, max_x) = m.body.iter().fold((f64::MAX, f64::MIN), |a, q| (a.0.min(q[0]), a.1.max(q[0])));
                let max_y = m.body.iter().fold(f64::MIN, |a, q| a.max(q[1]));
                assert!(min_x < 0.0 && max_x > w && max_y > h, "{g:?} {p:?}: {min_x} {max_x} {max_y}");
            }
        }
    }
}
