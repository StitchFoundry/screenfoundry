//! Start a print job from the garment (issue #56): `screen.newJob`, its underbase default and the
//! print-area check.

use serde_json::{Value, json};

use crate::*;

/// A rectangle `w` × `h` pt at (`x`, `y`), filled `fill`, on the first layer that takes art.
fn rect(s: &mut Session, x: f64, y: f64, w: f64, h: f64, fill: &str) {
    let r = s.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h})).unwrap();
    s.execute("select.set", &json!({"ids": [r["id"].as_u64().unwrap()]})).unwrap();
    s.execute("paint.setFill", &json!({"color": fill})).unwrap();
    s.execute("paint.setStroke", &json!({"ids": [r["id"].as_u64().unwrap()], "none": true})).unwrap();
}

fn job(garment: &str, color: &str, placement: &str) -> Session {
    let mut s = Session::new();
    s.execute("screen.newJob", &json!({"garment": garment, "color": color, "placement": placement})).unwrap();
    s
}

fn roles(r: &Value) -> Vec<String> {
    r["films"].as_array().unwrap().iter().map(|f| f["role"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_new_job_starts_from_the_garment_colour_and_location_not_a_page() {
    let mut s = Session::new();
    let r = s.execute("screen.newJob", &json!({"garment": "tshirt", "color": "Navy", "placement": "leftChest"})).unwrap();
    assert_eq!((r["areaIn"].clone(), r["color"].clone(), r["underbase"].clone()), (json!([4.0, 4.0]), json!("#192346"), json!(true)));
    assert!(r["reason"].as_str().unwrap().starts_with("Dark shirt"), "{r}");
    let st = s.doc().unwrap();
    // The artboard is the print area (4 in = 288 pt), in inches.
    let a = st.doc.artboards[0].rect;
    assert_eq!((a.width(), a.height()), (288.0, 288.0));
    assert_eq!(st.doc.units, vectorcraft_doc::Unit::Inches);
    // The garment is drawn on a locked template layer that never prints, under the art layer.
    let mock = &st.doc.layers[0];
    assert_eq!(mock.name.as_deref(), Some("Garment mockup"));
    assert!(mock.locked);
    assert!(matches!(mock.kind, NodeKind::Layer { template: true, printable: false, .. }));
    assert!(mock.children().is_some_and(|c| !c.is_empty()));
    assert_eq!(st.doc.layers[1].name.as_deref(), Some("Art"));
}

#[test]
fn locations_set_the_print_area_and_it_can_be_changed() {
    let s = job("tshirt", "#ffffff", "fullFront");
    let a = s.doc().unwrap().doc.artboards[0].rect;
    assert_eq!((a.width(), a.height()), (12.0 * 72.0, 14.0 * 72.0));
    let mut s = Session::new();
    s.execute("screen.newJob", &json!({"garment": "tshirt", "placement": "fullFront", "widthIn": 10, "heightIn": 11.5})).unwrap();
    let a = s.doc().unwrap().doc.artboards[0].rect;
    assert_eq!((a.width(), a.height()), (720.0, 828.0));
    // Bad choices are errors, not panics.
    for bad in [
        json!({}),
        json!({"garment": "jumper"}),
        json!({"garment": "tote", "placement": "sleeve"}),
        json!({"garment": "tshirt", "color": "teal-ish"}),
        json!({"garment": "tshirt", "widthIn": 0}),
        json!({"garment": "tshirt", "heightIn": 1e9}),
    ] {
        assert!(Session::new().execute("screen.newJob", &bad).is_err(), "{bad}");
    }
}

#[test]
fn the_garment_colour_sets_the_underbase_default() {
    let separate = |color: &str, extra: Value| {
        let mut s = job("tshirt", color, "fullFront");
        rect(&mut s, 36.0, 36.0, 200.0, 200.0, "#fadc14");
        let mut p = json!({"dpi": 50, "inks": [{"name": "Yellow", "color": "#fadc14"}]});
        p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let spot = s.execute("screen.separate", &p).unwrap();
        let vec = s.execute("screen.separateVector", &p).unwrap();
        (roles(&spot), roles(&vec))
    };
    let (dark, dark_vec) = separate("Black", json!({}));
    assert_eq!(dark, ["underbase", "color"]);
    assert_eq!(dark_vec, ["underbase", "color"]);
    let (light, light_vec) = separate("White", json!({}));
    assert_eq!((light, light_vec), (vec!["color".to_string()], vec!["color".to_string()]));
    // What the user asks for always wins.
    assert_eq!(separate("Black", json!({"underbase": false})).0, ["color"]);
    assert_eq!(separate("White", json!({"underbase": true})).0, ["underbase", "color"]);
}

#[test]
fn the_mockup_is_not_separated_and_the_films_are_the_print_area() {
    let mut s = job("tshirt", "Black", "leftChest");
    rect(&mut s, 18.0, 18.0, 100.0, 100.0, "#ffffff");
    let r = s.execute("screen.separateVector", &json!({"dpi": 100, "underbase": false, "marks": false})).unwrap();
    // One ink (the art's), not the dark garment drawn behind it; the film is 4 × 4 in.
    assert_eq!(r["inks"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!((r["width"].clone(), r["height"].clone()), (json!(400), json!(400)));
    // Registration marks still go on the films.
    let m = s.execute("screen.separateVector", &json!({"dpi": 100, "marks": true})).unwrap();
    assert!(m["films"].as_array().unwrap().iter().all(|f| f["width"].as_u64().unwrap() >= 400));
}

#[test]
fn art_outside_the_print_area_is_warned_about_not_blocked() {
    let mut s = job("tshirt", "#ffffff", "leftChest");
    assert_eq!(s.execute("screen.printAreaCheck", &json!({})).unwrap()["inside"], json!(true));
    rect(&mut s, 10.0, 10.0, 100.0, 100.0, "#cc0000");
    let ok = s.execute("screen.printAreaCheck", &json!({})).unwrap();
    assert_eq!((ok["inside"].clone(), ok["artIn"].clone()), (json!(true), json!([1.39, 1.39])));
    rect(&mut s, 100.0, 100.0, 400.0, 100.0, "#cc0000");
    let warn = s.execute("screen.printAreaCheck", &json!({})).unwrap();
    assert_eq!(warn["inside"], json!(false));
    assert!(warn["message"].as_str().unwrap().contains("outside the print area"), "{warn}");
    // The separation still runs.
    assert!(s.execute("screen.separateVector", &json!({"dpi": 50})).is_ok());
}

#[test]
fn a_blank_canvas_and_the_new_document_presets_work_as_before() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"preset": "Letter"})).unwrap();
    let a = s.doc().unwrap().doc.artboards[0].rect;
    assert_eq!((a.width(), a.height()), (612.0, 792.0));
    assert_eq!(s.doc().unwrap().doc.layers.len(), 1, "no mockup layer");
    assert_eq!(s.execute("screen.printAreaCheck", &json!({})).unwrap()["job"], json!(false));
    rect(&mut s, 10.0, 10.0, 100.0, 100.0, "#336699");
    // No garment, no underbase unless asked.
    let r = s.execute("screen.separateVector", &json!({"dpi": 50})).unwrap();
    assert_eq!(roles(&r), ["color"]);
    let r = s.execute("screen.separateVector", &json!({"dpi": 50, "underbase": true})).unwrap();
    assert_eq!(roles(&r), ["underbase", "color"]);
}

#[test]
fn the_choices_are_listed_for_the_picker() {
    let r = Session::new().execute("screen.garments", &json!({})).unwrap();
    assert_eq!(r["garments"].as_array().unwrap().len(), 5);
    let shirt = &r["garments"][0];
    assert_eq!(shirt["id"], json!("tshirt"));
    assert_eq!(shirt["placements"][0]["id"], json!("leftChest"));
    assert!(shirt["outline"]["body"].as_array().unwrap().len() > 8);
    assert!(r["swatches"].as_array().unwrap().iter().any(|s| s["name"] == "Black" && s["underbase"] == true));
}
