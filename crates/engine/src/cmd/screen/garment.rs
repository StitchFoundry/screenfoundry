//! Start a print job from the garment (issue #56): what is being printed on, its colour and where
//! the print goes, instead of a page size.
//!
//! `screen.newJob` makes a document whose first artboard is the print area (so the films are the
//! size of the print), with a plain garment drawing in the garment colour on a locked template
//! layer behind it (it never prints). What was chosen is kept with the document, and the garment
//! colour sets the underbase default of the separation commands. `screen.printAreaCheck` tells
//! whether the art has left the print area; it warns, it never blocks.

use std::borrow::Cow;

use sc_core::color::{parse_hex, to_hex};
use sc_core::garment::{Garment, Mockup, Placement, SWATCHES, print_area, underbase_reason, wants_underbase};
use serde_json::{Value, json};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document, LayerColor, Node, NodeKind};
use vectorcraft_geom::{PathData, Point, Rect, SubPath};

use super::{CommandSpec, Result, Session, always, bad, cmd, str_param};
use crate::cmd::has_doc;

/// The key the job is kept under in the document's plug-in data (see `vectorcraft_doc::plugin_data`).
pub const JOB_KEY: &str = "screenfoundry.job";
/// The name of the layer holding the garment drawing.
pub const MOCKUP_LAYER: &str = "Garment mockup";
const PT_PER_IN: f64 = 72.0;
/// Print areas are 0.5 to 100 inches a side.
const AREA_RANGE: std::ops::RangeInclusive<f64> = 0.5..=100.0;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "screen.garments",
            "Garment Choices",
            [],
            None,
            "{} what a print job can start from → {garments: [{id, label, placements: [{id, label, widthIn, heightIn}], wearable, outline: {body, extras: [[ [x, y] in inches ]]}}], swatches: [{name, color, underbase, reason}]}; the sizes are rules of thumb you can change",
            always,
            garments
        ),
        cmd!(
            "screen.newJob",
            "New Print Job",
            ["File"],
            None,
            "{garment: tshirt|hoodie|tote|poster|other, color?: \"#rrggbb\" or a swatch name from screen.garments (white), placement?: leftChest|fullFront|fullBack|sleeve|oversized|custom (the item's first), widthIn?, heightIn?: the print area in inches (0.5–100; default: the rule of thumb for the placement), name?: document name} a new document for printing on a garment: the artboard is the print area, a locked template layer behind it draws the garment in its colour, and the garment colour sets the underbase default of the screen.* separations (dark: on, light: off; an explicit `underbase` still wins) → {index, garment, placement, color, areaIn: [w, h], underbase, reason}",
            always,
            new_job
        ),
        cmd!(
            query "screen.printAreaCheck",
            "Check Print Area",
            [],
            None,
            "{} whether the printing art of a garment job stays inside its print area → {job: bool, inside: bool, areaIn?: [w, h], artIn?: [w, h], message}; art outside is not printed on the films, so this warns",
            has_doc,
            print_area_check
        ),
    ]
}

fn garments(_s: &mut Session, _p: &Value) -> Result<Value> {
    let garments: Vec<Value> = Garment::ALL
        .iter()
        .map(|g| {
            let placements: Vec<Value> = g
                .placements()
                .iter()
                .map(|p| {
                    let (w, h) = print_area(*g, *p);
                    json!({ "id": p.id(), "label": p.label(), "widthIn": w, "heightIn": h })
                })
                .collect();
            // The drawing of the item at its first location, for the picker's pictures.
            let (w, h) = print_area(*g, g.default_placement());
            let m = Mockup::new(*g, g.default_placement(), (w, h));
            json!({ "id": g.id(), "label": g.label(), "placements": placements, "wearable": g.is_wearable(), "outline": { "body": m.body, "extras": m.extras } })
        })
        .collect();
    let swatches: Vec<Value> = SWATCHES
        .iter()
        .map(|(name, rgb)| json!({ "name": name, "color": to_hex(*rgb), "underbase": wants_underbase(*rgb), "reason": underbase_reason(*rgb) }))
        .collect();
    Ok(json!({ "garments": garments, "swatches": swatches }))
}

/// A colour given as `#rrggbb` or as the name of a swatch.
fn garment_color(p: &Value, cmd: &str) -> Result<[u8; 3]> {
    match str_param(p, "color") {
        None => Ok([255, 255, 255]),
        Some(c) => parse_hex(c)
            .or_else(|| SWATCHES.iter().find(|(n, _)| n.eq_ignore_ascii_case(c.trim())).map(|(_, rgb)| *rgb))
            .ok_or_else(|| bad(cmd, "`color` is a colour like \"#112233\" or a swatch name (see screen.garments)")),
    }
}

fn side(p: &Value, cmd: &str, key: &str, default: f64) -> Result<f64> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v
            .as_f64()
            .filter(|v| AREA_RANGE.contains(v))
            .ok_or_else(|| bad(cmd, format!("`{key}` is a number of inches from {} to {}", AREA_RANGE.start(), AREA_RANGE.end()))),
    }
}

fn polygon(points: &[[f64; 2]]) -> PathData {
    let pts: Vec<Point> = points.iter().map(|p| Point::new(p[0] * PT_PER_IN, p[1] * PT_PER_IN)).collect();
    PathData::new(vec![SubPath::polyline(&pts, true)])
}

/// The template layer with the garment drawn in `rgb`.
fn mockup_layer(d: &mut Document, mock: &Mockup, rgb: [u8; 3]) -> Result<()> {
    let id = d.alloc_id();
    let mut layer = Node::layer(id, MOCKUP_LAYER, LayerColor::Preset(0));
    layer.locked = true;
    if let NodeKind::Layer { template, printable, .. } = &mut layer.kind {
        *template = true;
        *printable = false;
    }
    // Index 0 puts it under any art.
    d.layers.insert(0, std::sync::Arc::new(layer));
    let fill = Paint::solid(Color::rgb(f32::from(rgb[0]) / 255.0, f32::from(rgb[1]) / 255.0, f32::from(rgb[2]) / 255.0));
    let outline = Paint::solid(Color::rgb(0.5, 0.5, 0.5));
    // Extras (hood, straps) go under the body so its outline stays whole.
    for (i, o) in mock.extras.iter().chain(std::iter::once(&mock.body)).enumerate() {
        let nid = d.alloc_id();
        let mut n = Node::path(nid, polygon(o), Appearance::basic(fill.clone(), outline.clone(), 0.75));
        n.name = Some(if i < mock.extras.len() { "Garment detail".to_string() } else { "Garment".to_string() });
        d.insert(Some(id), i, n).map_err(|e| bad("screen.newJob", e.to_string()))?;
    }
    Ok(())
}

fn new_job(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "screen.newJob";
    let garment = str_param(p, "garment")
        .and_then(Garment::parse)
        .ok_or_else(|| bad(C, "give `garment`: tshirt, hoodie, tote, poster or other (see screen.garments)"))?;
    let placement = match str_param(p, "placement") {
        None => garment.default_placement(),
        Some(n) => Placement::parse(n).filter(|pl| garment.placements().contains(pl)).ok_or_else(|| {
            bad(
                C,
                format!("`placement` for {} is one of: {}", garment.id(), garment.placements().iter().map(|p| p.id()).collect::<Vec<_>>().join(", ")),
            )
        })?,
    };
    let rgb = garment_color(p, C)?;
    let (dw, dh) = print_area(garment, placement);
    let (w, h) = (side(p, C, "widthIn", dw)?, side(p, C, "heightIn", dh)?);
    let underbase = garment.is_wearable() && wants_underbase(rgb);

    let settings = s
        .default_settings()
        .with_params(&json!({ "width": w * PT_PER_IN, "height": h * PT_PER_IN, "units": "Inches", "backgroundContents": "transparent" }), C)?;
    let mut d = settings.document();
    d.grid = s.prefs.grid();
    d.title = match str_param(p, "name").map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => t.to_string(),
        None => s.next_untitled(),
    };
    let mock = Mockup::new(garment, placement, (w, h));
    mockup_layer(&mut d, &mock, rgb)?;
    // The art goes on the layer above the drawing (the document's first, named for what it holds).
    match d.layers.get_mut(1) {
        Some(art) => std::sync::Arc::make_mut(art).name = Some("Art".into()),
        None => {
            d.add_layer(Some("Art"));
        }
    }
    let job = json!({
        "garment": garment.id(), "placement": placement.id(), "color": to_hex(rgb),
        "areaIn": [w, h], "underbase": underbase,
    });
    d.set_plugin_data(JOB_KEY, Some(job));
    let index = s.add_document(d, None);
    Ok(json!({
        "index": index, "garment": garment.id(), "placement": placement.id(), "color": to_hex(rgb),
        "areaIn": [w, h], "underbase": underbase,
        "reason": if garment.is_wearable() { underbase_reason(rgb) } else { "Paper takes the colour of the stock: choose the underbase yourself if you need one." },
    }))
}

/// The garment job kept with the open document, if it has one.
fn job_of(s: &Session) -> Option<Value> {
    s.active().and_then(|st| st.doc.plugin_data(JOB_KEY).cloned())
}

/// `p` with the garment's underbase default added when it gives none: a dark garment turns the
/// underbase on. An explicit `underbase` (true or false) always wins; documents without a
/// garment job are untouched. Applies to the separations of the open document.
pub(super) fn with_job_defaults<'a>(s: &Session, p: &'a Value) -> Cow<'a, Value> {
    let from_document = matches!(p.get("source"), None | Some(Value::Null)) || p.get("source").and_then(Value::as_str) == Some("document");
    if !from_document || p.get("underbase").is_some_and(|u| !u.is_null()) {
        return Cow::Borrowed(p);
    }
    match job_of(s).and_then(|j| j.get("underbase").and_then(Value::as_bool)) {
        Some(true) => {
            let mut q = p.clone();
            if let Some(o) = q.as_object_mut() {
                o.insert("underbase".into(), json!(true));
            } else {
                q = json!({ "underbase": true });
            }
            Cow::Owned(q)
        }
        _ => Cow::Borrowed(p),
    }
}

fn print_area_check(s: &mut Session, _p: &Value) -> Result<Value> {
    let st = s.doc()?;
    let Some(job) = st.doc.plugin_data(JOB_KEY) else {
        return Ok(json!({ "job": false, "inside": true, "message": "This document did not start from a garment." }));
    };
    let Some(board) = st.doc.artboards.first().map(|a| a.rect) else {
        return Ok(json!({ "job": true, "inside": true, "message": "No print area." }));
    };
    // The printing art: every visible layer except templates and layers that do not print.
    let ids: Vec<_> = st
        .doc
        .layers
        .iter()
        .filter(|l| l.visible && !matches!(&l.kind, NodeKind::Layer { template, printable, .. } if *template || !*printable))
        .map(|l| l.id)
        .collect();
    let art: Option<Rect> = st.doc.bounds_of(&ids, true);
    let area_in = job.get("areaIn").cloned().unwrap_or(Value::Null);
    let Some(a) = art else {
        return Ok(json!({ "job": true, "inside": true, "areaIn": area_in, "message": "Place your art inside the print area." }));
    };
    const SLACK: f64 = 0.01;
    let inside = a.x0 >= board.x0 - SLACK && a.y0 >= board.y0 - SLACK && a.x1 <= board.x1 + SLACK && a.y1 <= board.y1 + SLACK;
    let art_in = [(a.width() / PT_PER_IN * 100.0).round() / 100.0, (a.height() / PT_PER_IN * 100.0).round() / 100.0];
    let message = if inside {
        "Your art fits the print area.".to_string()
    } else {
        format!(
            "Your art is {} x {} in and reaches outside the print area: anything outside is left off the films. Make it smaller or move it inside.",
            art_in[0], art_in[1]
        )
    };
    Ok(json!({ "job": true, "inside": inside, "areaIn": area_in, "artIn": art_in, "message": message }))
}
