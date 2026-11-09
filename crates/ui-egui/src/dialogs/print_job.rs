//! Start a print job (issue #56): what is being printed on, its colour and where the print goes,
//! in one window of three plain steps, instead of a page size. Create runs `screen.newJob`.
//!
//! Fields: `garment`, `color` (`#rrggbb`), `placement`, `widthIn` and `heightIn` (the print area,
//! which a new location resets to its rule of thumb), plus `__catalog` (`screen.garments`, read
//! once when the window opens). The pictures are outlines drawn in code from the engine's mockup.

use serde_json::{Value, json};

use super::DialogSpec;
use crate::state::Dialog;
use crate::theme::{self, Tokens};
use crate::{VectorcraftApp, widgets};

pub(super) const KIND: &str = "newPrintJob";
pub(super) const SPEC: DialogSpec = DialogSpec::window(show, confirm);

const CATALOG: &str = "__catalog";
const WIDTH: f32 = 640.0;
const CARD: egui::Vec2 = egui::vec2(116.0, 108.0);

/// Open the window on a white T-shirt with the first location.
pub fn open(app: &mut VectorcraftApp) {
    let catalog = app.session.execute("screen.garments", &json!({})).unwrap_or(Value::Null);
    let mut d = Dialog::new(KIND, json!({ CATALOG: catalog, "color": "#ffffff", "garment": "tshirt" }));
    choose_garment(&mut d, "tshirt");
    app.ui.dialog = Some(d);
}

fn garments(d: &Dialog) -> Vec<Value> {
    d.fields.get(CATALOG).and_then(|c| c["garments"].as_array()).cloned().unwrap_or_default()
}

fn placements(d: &Dialog) -> Vec<Value> {
    garments(d).into_iter().find(|g| g["id"] == d.str("garment")).and_then(|g| g["placements"].as_array().cloned()).unwrap_or_default()
}

fn choose_garment(d: &mut Dialog, id: &str) {
    d.fields.insert("garment".into(), json!(id));
    if let Some(first) = placements(d).first().and_then(|p| p["id"].as_str()).map(str::to_string) {
        choose_placement(d, &first);
    }
}

/// A location resets the print area to its starting size.
fn choose_placement(d: &mut Dialog, id: &str) {
    d.fields.insert("placement".into(), json!(id));
    if let Some(p) = placements(d).into_iter().find(|p| p["id"] == id) {
        d.fields.insert("widthIn".into(), p["widthIn"].clone());
        d.fields.insert("heightIn".into(), p["heightIn"].clone());
    }
}

/// The `screen.newJob` params of the fields.
fn params(d: &Dialog) -> Value {
    json!({
        "garment": d.str("garment"), "color": d.str("color"), "placement": d.str("placement"),
        "widthIn": d.f64("widthIn", 12.0), "heightIn": d.f64("heightIn", 12.0),
    })
}

fn confirm(app: &mut VectorcraftApp, d: &Dialog) -> Result<Value, String> {
    let r = app.run("screen.newJob", params(d))?;
    app.ui.dialog = None;
    Ok(r)
}

fn hex(s: &str) -> Option<egui::Color32> {
    let s = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(egui::Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8 & 0xff, v as u8))
}

/// Triangles of a simple polygon by ear clipping (the garment outlines are concave at the neck).
fn triangulate(pts: &[egui::Pos2]) -> Vec<[usize; 3]> {
    let n = pts.len();
    let area: f32 = (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum();
    let mut idx: Vec<usize> = if area < 0.0 { (0..n).rev().collect() } else { (0..n).collect() };
    let cross = |a: egui::Pos2, b: egui::Pos2, c: egui::Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut out = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (pa, pb, pc) = (pts[a], pts[b], pts[c]);
            cross(pa, pb, pc) > 0.0
                && !idx.iter().any(|&o| {
                    o != a && o != b && o != c && {
                        let p = pts[o];
                        cross(pa, pb, p) >= 0.0 && cross(pb, pc, p) >= 0.0 && cross(pc, pa, p) >= 0.0
                    }
                })
        });
        let Some(i) = ear else { break };
        out.push([idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]]);
        idx.remove(i);
    }
    if let [a, b, c] = idx[..] {
        out.push([a, b, c]);
    }
    out
}

fn outline_points(v: &Value) -> Vec<[f32; 2]> {
    v.as_array().map(|a| a.iter().filter_map(|p| Some([p[0].as_f64()? as f32, p[1].as_f64()? as f32])).collect()).unwrap_or_default()
}

/// Draw an item's outline (its extras first) fitted into `rect`, filled with `fill`.
fn draw_item(painter: &egui::Painter, rect: egui::Rect, item: &Value, fill: egui::Color32, line: egui::Color32) {
    let mut shapes: Vec<Vec<[f32; 2]>> = item["outline"]["extras"].as_array().map(|a| a.iter().map(outline_points).collect()).unwrap_or_default();
    shapes.push(outline_points(&item["outline"]["body"]));
    let all = shapes.iter().flatten();
    let (min, max) = all.fold(([f32::MAX; 2], [f32::MIN; 2]), |(lo, hi), p| ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])]));
    let (w, h) = (max[0] - min[0], max[1] - min[1]);
    if !(w.is_finite() && h.is_finite()) || w <= 0.0 || h <= 0.0 {
        return;
    }
    let k = (rect.width() / w).min(rect.height() / h);
    let origin = rect.center() - egui::vec2(w * k, h * k) / 2.0;
    for s in &shapes {
        let pts: Vec<egui::Pos2> = s.iter().map(|p| origin + egui::vec2((p[0] - min[0]) * k, (p[1] - min[1]) * k)).collect();
        if pts.len() < 3 {
            continue;
        }
        let mut mesh = egui::Mesh::default();
        mesh.vertices = pts.iter().map(|&pos| egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color: fill }).collect();
        mesh.indices = triangulate(&pts).into_iter().flat_map(|t| t.map(|i| i as u32)).collect();
        painter.add(egui::Shape::mesh(mesh));
        painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(1.25, line)));
    }
}

fn section(ui: &mut egui::Ui, n: u32, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(12.0);
    ui.label(egui::RichText::new(format!("{n}.  {text}")).font(theme::semibold(13.0)).color(t.text));
    ui.add_space(6.0);
}

fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let Some(mut d) = app.ui.dialog.clone() else { return };
    let t = Tokens::get(ctx);
    let (mut create, mut close, mut blank) = (false, false, false);
    super::modal::show(ctx, KIND, egui::Id::unique(("dialog", KIND)), -20.0, 20, |ui| {
        ui.set_width(WIDTH);
        super::modal::heading(ui, tl!("Start a print job"));
        ui.label(
            egui::RichText::new(tl!("Start with what you are printing on. The page size comes later, when you make the films.")).color(t.text_dim),
        );

        section(ui, 1, tl!("What are you printing on?"));
        let color = hex(&d.str("color")).unwrap_or(egui::Color32::WHITE);
        let wearable_fill = |g: &Value| if g["wearable"] == true { color } else { egui::Color32::WHITE };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for g in garments(&d) {
                let id = g["id"].as_str().unwrap_or_default().to_string();
                let selected = d.str("garment") == id;
                let (r, resp) = ui.allocate_exact_size(CARD, egui::Sense::click());
                ui.painter().rect_filled(r, 6.0, if resp.hovered() || selected { t.hover } else { t.panel_darker });
                if selected {
                    ui.painter().rect_stroke(r, 6.0, egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
                }
                draw_item(
                    ui.painter(),
                    egui::Rect::from_min_max(r.min + egui::vec2(10.0, 8.0), r.max - egui::vec2(10.0, 30.0)),
                    &g,
                    wearable_fill(&g),
                    t.text_dim,
                );
                let label = g["label"].as_str().unwrap_or_default();
                ui.painter().text(
                    egui::pos2(r.center().x, r.bottom() - 14.0),
                    egui::Align2::CENTER_CENTER,
                    tl!(label),
                    theme::semibold(11.0),
                    t.text_strong,
                );
                if resp.on_hover_text(tl!(label)).clicked() && !selected {
                    choose_garment(&mut d, &id);
                }
            }
        });

        let wearable = garments(&d).iter().any(|g| g["id"] == d.str("garment") && g["wearable"] == true);
        section(ui, 2, if wearable { tl!("Garment colour") } else { tl!("Colour of the stock") });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for s in d.fields.get(CATALOG).and_then(|c| c["swatches"].as_array()).cloned().unwrap_or_default() {
                let (Some(name), Some(c)) = (s["name"].as_str(), s["color"].as_str().and_then(hex)) else { continue };
                let (r, resp) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
                ui.painter().rect_filled(r, 4.0, c);
                let on = d.str("color").eq_ignore_ascii_case(s["color"].as_str().unwrap_or_default());
                let ring = if on { egui::Stroke::new(2.5, t.accent) } else { egui::Stroke::new(1.0, t.text_dim) };
                ui.painter().rect_stroke(r, 4.0, ring, egui::StrokeKind::Outside);
                if resp.on_hover_text(tl!(name)).clicked() {
                    d.fields.insert("color".into(), s["color"].clone());
                }
            }
            let custom = ui.label(egui::RichText::new(tl!("Custom")).color(t.text_dim));
            let mut text = d.str("color");
            // The input is named by the label beside it.
            let changed = ui.label_inputs_by(custom.id, |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).desired_width(72.0).font(egui::TextStyle::Monospace)).changed()
            });
            if changed {
                d.fields.insert("color".into(), json!(text));
            }
        });
        // The plain reason the underbase is on or off (the same rule the separations use).
        if wearable && let Some(rgb) = hex(&d.str("color")) {
            let dark = srgb_l(rgb) < 60.0;
            ui.add_space(4.0);
            let line = if dark {
                tl!("Dark shirt: a white underbase prints first so colours stay bright.")
            } else {
                tl!("Light shirt: no underbase needed, the colours print straight onto the fabric.")
            };
            ui.label(egui::RichText::new(line).color(t.text));
        }

        section(ui, 3, tl!("Where does the print go?"));
        ui.horizontal_wrapped(|ui| {
            for p in placements(&d) {
                let id = p["id"].as_str().unwrap_or_default().to_string();
                let label = p["label"].as_str().unwrap_or_default();
                if ui.selectable_label(d.str("placement") == id, tl!(label)).clicked() {
                    choose_placement(&mut d, &id);
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tl!("Print area")).color(t.text_dim));
            for key in ["widthIn", "heightIn"] {
                let mut v = d.f64(key, 12.0);
                if ui.add(egui::DragValue::new(&mut v).range(0.5..=100.0).speed(0.1).suffix(" in")).changed() {
                    d.fields.insert(key.into(), json!(v));
                }
                if key == "widthIn" {
                    ui.label("×");
                }
            }
            ui.label(egui::RichText::new(tl!("Starting sizes are rules of thumb: change them to suit the job.")).color(t.text_dim));
        });

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            blank = widgets::flat_button(ui, tl!("Blank canvas (page size)…"), 190.0).clicked();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                create = widgets::primary_button(ui, tl!("Create")).clicked();
                ui.add_space(8.0);
                close = widgets::secondary_button(ui, tl!("Close")).clicked();
            });
        });
    });
    app.ui.dialog = Some(d);
    if close {
        app.ui.dialog = None;
    } else if blank {
        super::open_new_document(app);
    } else if create && let Err(e) = super::confirm(app) {
        app.status(e);
    }
}

/// CIE L* of an sRGB colour (the engine uses the same rule to set the underbase; this only words
/// the line under the swatches).
fn srgb_l(c: egui::Color32) -> f32 {
    let lin = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    let y = 0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b());
    if y > 0.008856 { 116.0 * y.cbrt() - 16.0 } else { 903.3 * y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_params_follow_the_choices() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        open(&mut app);
        let mut d = app.ui.dialog.clone().unwrap();
        assert_eq!((d.str("garment"), d.str("placement")), ("tshirt".into(), "leftChest".into()));
        choose_placement(&mut d, "fullFront");
        assert_eq!(params(&d)["widthIn"], json!(12.0));
        assert_eq!(params(&d)["heightIn"], json!(14.0));
        choose_garment(&mut d, "poster");
        assert_eq!(d.str("placement"), "custom");
        assert_eq!(params(&d)["heightIn"], json!(24.0));
    }

    #[test]
    fn creating_makes_the_job_and_closes_the_window() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        open(&mut app);
        let mut d = app.ui.dialog.clone().unwrap();
        choose_placement(&mut d, "fullFront");
        d.fields.insert("color".into(), json!("#141416"));
        let r = confirm(&mut app, &d).unwrap();
        assert_eq!(r["underbase"], json!(true));
        assert!(app.ui.dialog.is_none());
        assert_eq!(app.session.documents().len(), 1);
    }

    #[test]
    fn ear_clipping_covers_a_concave_outline() {
        // An L shape: 6 vertices, 4 triangles, area 3.
        let pts: Vec<egui::Pos2> =
            [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (1.0, 1.0), (1.0, 2.0), (0.0, 2.0)].iter().map(|&(x, y)| egui::pos2(x, y)).collect();
        let tris = triangulate(&pts);
        assert_eq!(tris.len(), 4);
        let area: f32 = tris
            .iter()
            .map(|t| ((pts[t[1]] - pts[t[0]]).x * (pts[t[2]] - pts[t[0]]).y - (pts[t[1]] - pts[t[0]]).y * (pts[t[2]] - pts[t[0]]).x).abs() / 2.0)
            .sum();
        assert!((area - 3.0).abs() < 1e-4, "{area}");
    }

    /// Headless screenshots of the start screen: `SF_SCREENSHOT_DIR=docs/images cargo test -p
    /// vectorcraft-ui-egui print_job_screenshots` (needs a software or real GPU adapter).
    #[test]
    fn print_job_screenshots() {
        let Some(dir) = std::env::var_os("SF_SCREENSHOT_DIR").map(std::path::PathBuf::from) else { return };
        std::fs::create_dir_all(&dir).unwrap();
        let shot = |name: &str, size: egui::Vec2, prepare: &dyn Fn(&mut VectorcraftApp), draw: fn(&mut VectorcraftApp, &mut egui::Ui)| {
            let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
            prepare(&mut app);
            let mut ready = false;
            let mut h = egui_kittest::Harness::builder().with_size(size).with_pixels_per_point(1.5).wgpu().build_ui_state(
                move |ui, app: &mut VectorcraftApp| {
                    if !std::mem::replace(&mut ready, true) {
                        crate::theme::install_fonts(ui.ctx());
                        crate::theme::apply(ui.ctx(), crate::theme::Brightness::Dark);
                        return;
                    }
                    draw(app, ui);
                },
                app,
            );
            h.input_mut().max_texture_side = Some(8192);
            h.run_steps(5);
            h.render().unwrap().save(dir.join(name)).unwrap();
        };
        let dialog: fn(&mut VectorcraftApp, &mut egui::Ui) = |app, ui| show(app, &ui.ctx().clone());
        shot("start-1-home.png", egui::vec2(980.0, 640.0), &|_| {}, |app, ui| crate::canvas::show(app, ui));
        shot(
            "start-2-dark-shirt-left-chest.png",
            egui::vec2(720.0, 560.0),
            &|app| {
                open(app);
                let mut d = app.ui.dialog.clone().unwrap();
                d.fields.insert("color".into(), json!("#191a1c"));
                app.ui.dialog = Some(d);
            },
            dialog,
        );
        shot(
            "start-3-light-hoodie-full-front.png",
            egui::vec2(720.0, 560.0),
            &|app| {
                open(app);
                let mut d = app.ui.dialog.clone().unwrap();
                choose_garment(&mut d, "hoodie");
                choose_placement(&mut d, "fullFront");
                d.fields.insert("color".into(), json!("#f0aabe"));
                app.ui.dialog = Some(d);
            },
            dialog,
        );
        shot(
            "start-4-new-job-on-the-canvas.png",
            egui::vec2(980.0, 700.0),
            &|app| {
                app.run("screen.newJob", json!({"garment": "tshirt", "color": "Navy", "placement": "fullFront"})).unwrap();
                let r = app.session.execute("shape.ellipse", &json!({"x": 90, "y": 120, "width": 680, "height": 560})).unwrap();
                app.session.execute("select.set", &json!({"ids": [r["id"].as_u64().unwrap()]})).unwrap();
                app.session.execute("paint.setFill", &json!({"color": "#fadc14"})).unwrap();
                app.session.execute("view.zoomFit", &json!({})).ok();
            },
            |app, ui| crate::canvas::show(app, ui),
        );
    }
}
