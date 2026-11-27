//! MakerFoundry (SC-11): the drawing as the SVG a draft on MakerFoundry is made from. The engine
//! has no network code: this command exports and checks, and the host (File › Publish to
//! MakerFoundry…, `ui.publishToMakerFoundry`) shows the person what goes where, signs the request
//! with their Foundry Account and sends it (`sc-maker`).

use sc_maker::Site;
use sc_maker::publish::{MAX_FILE_BYTES, svg_file_name};
use sc_maker::share_prompt::{Choice, should_offer};
use serde_json::{Value, json};

use super::fileio::encode_all;
use super::{CommandSpec, Result, Session, bad, cmd, has_doc, str_param};

const C: &str = "screen.makerFoundryExport";

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "screen.makerFoundryExport",
            "Export for MakerFoundry",
            [],
            None,
            "{fileName?: the name to publish under (default: the document's title; always a plain .svg name)} the drawing as the SVG MakerFoundry takes as a design (not sent: ui.publishToMakerFoundry sends it) → {fileName, bytes, fileBase64, site, app, warnings}; an empty drawing or one over 8 MB is an error",
            has_doc,
            export
        ),
        cmd!(
            query "makerfoundry.shareOffer",
            "Offer to Share on MakerFoundry",
            [],
            None,
            "{exportSucceeded?: whether the export just done worked (default true)} whether to show \"Share it on MakerFoundry?\" now: the Preferences toggle is on, the export worked and this document wasn't asked yet → {offer}",
            has_doc,
            share_offer
        ),
        cmd!(
            "makerfoundry.shareAnswer",
            "Answer the MakerFoundry Share Prompt",
            [],
            None,
            "{choice: \"shareFree\" | \"sell\" | \"notNow\", dontAskAgain?: bool (false)} record the answer: this document counts as asked, and dontAskAgain turns Preferences > MakerFoundry > Offer to share after export off → {proceed: the host goes on to sign in and open the draft page, offerSetting}",
            has_doc,
            share_answer
        ),
    ]
}

fn share_offer(s: &mut Session, p: &Value) -> Result<Value> {
    let succeeded = match p.get("exportSucceeded") {
        None | Some(Value::Null) => true,
        Some(v) => v.as_bool().ok_or_else(|| bad("makerfoundry.shareOffer", "exportSucceeded must be true or false"))?,
    };
    let shown = s.doc()?.share_prompt;
    Ok(json!({"offer": should_offer(s.prefs.offer_share_after_export, shown, succeeded)}))
}

fn share_answer(s: &mut Session, p: &Value) -> Result<Value> {
    const A: &str = "makerfoundry.shareAnswer";
    let choice = match str_param(p, "choice") {
        Some("shareFree") => Choice::ShareFree,
        Some("sell") => Choice::Sell,
        Some("notNow") => Choice::NotNow,
        _ => return Err(bad(A, "choice must be shareFree, sell or notNow")),
    };
    let dont_ask = match p.get("dontAskAgain") {
        None | Some(Value::Null) => false,
        Some(v) => v.as_bool().ok_or_else(|| bad(A, "dontAskAgain must be true or false"))?,
    };
    let setting = s.prefs.offer_share_after_export;
    let st = s.doc_mut()?;
    let out = st.share_prompt.record(choice, dont_ask, setting);
    st.share_prompt = out.prompt;
    s.prefs.offer_share_after_export = out.offer_setting;
    Ok(json!({"proceed": out.proceed, "offerSetting": out.offer_setting}))
}

fn export(s: &mut Session, p: &Value) -> Result<Value> {
    let st = s.doc()?;
    let doc = &st.doc;
    if !doc.layers.iter().any(|l| l.children().is_some_and(|c| !c.is_empty())) {
        return Err(bad(C, "the drawing is empty: draw something before publishing it"));
    }
    let name = svg_file_name(str_param(p, "fileName").unwrap_or(&doc.title));
    let mut encoded = encode_all(doc, "svg", &json!({}))?;
    let mut warnings = std::mem::take(&mut encoded.warnings);
    let mut files = encoded.files.into_iter();
    let (_, bytes) = files.next().ok_or_else(|| bad(C, "the drawing is empty: draw something before publishing it"))?;
    if files.next().is_some() {
        warnings.push("the drawing has several artboards: only the first is published".into());
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(bad(C, "the SVG is bigger than the 8 MB MakerFoundry takes"));
    }
    Ok(json!({
        "fileName": name,
        "bytes": bytes.len(),
        "fileBase64": vectorcraft_format::base64_encode(&bytes),
        "site": Site::MAKERFOUNDRY,
        "app": format!("ScreenFoundry {}", env!("CARGO_PKG_VERSION")),
        "warnings": warnings,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_session() -> Session {
        let mut s = Session::new();
        s.execute("file.new", &json!({"width": 200, "height": 100})).unwrap();
        s
    }

    fn session_with_art() -> Session {
        let mut s = empty_session();
        s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 100, "height": 50})).unwrap();
        s
    }

    #[test]
    fn the_drawing_is_exported_as_a_plain_svg_named_for_the_document() {
        let mut s = session_with_art();
        let r = s.execute(C, &json!({})).unwrap();
        assert_eq!((r["site"].as_str(), r["fileName"].as_str()), (Some("https://makerfoundry.app"), Some("Untitled-1.svg")));
        let svg = vectorcraft_format::base64_decode(r["fileBase64"].as_str().unwrap()).unwrap();
        assert_eq!(svg.len() as u64, r["bytes"].as_u64().unwrap());
        assert!(String::from_utf8(svg).unwrap().contains("<svg"));
        assert!(r["app"].as_str().unwrap().starts_with("ScreenFoundry "));
    }

    #[test]
    fn the_file_name_is_always_a_plain_svg_name() {
        let mut s = session_with_art();
        for (given, want) in [("shirt", "shirt.svg"), ("../../etc/passwd", "passwd.svg"), ("a\r\nb.svg", "ab.svg"), ("", "design.svg")] {
            assert_eq!(s.execute(C, &json!({"fileName": given})).unwrap()["fileName"], want, "{given:?}");
        }
    }

    #[test]
    fn an_empty_drawing_is_refused() {
        let mut s = empty_session();
        let e = s.execute(C, &json!({})).unwrap_err().to_string();
        assert!(e.contains("empty"), "{e}");
        assert!(session_with_art().execute(C, &json!({})).is_ok());
        // a document with only an empty layer is empty too
        s.execute("layer.new", &json!({})).unwrap();
        assert!(s.execute(C, &json!({})).is_err());
        // and drawing then deleting everything leaves it empty again
        let mut art = session_with_art();
        art.execute("select.all", &json!({})).unwrap();
        art.execute("edit.clear", &json!({})).unwrap();
        assert!(art.execute(C, &json!({})).is_err());
    }

    #[test]
    fn offered_once_per_document_and_never_when_off_or_failed() {
        let mut s = empty_session();
        let offer = |s: &mut Session, p: Value| s.execute("makerfoundry.shareOffer", &p).unwrap()["offer"] == true;
        assert!(!offer(&mut s, json!({"exportSucceeded": false})));
        assert!(offer(&mut s, json!({})));
        let r = s.execute("makerfoundry.shareAnswer", &json!({"choice": "notNow"})).unwrap();
        assert_eq!((r["proceed"].clone(), r["offerSetting"].clone()), (json!(false), json!(true)));
        assert!(!offer(&mut s, json!({})));
        // another document is still asked
        s.execute("file.new", &json!({"width": 10, "height": 10})).unwrap();
        assert!(offer(&mut s, json!({})));
        s.prefs.offer_share_after_export = false;
        assert!(!offer(&mut s, json!({})));
    }

    #[test]
    fn dont_ask_again_turns_the_setting_off_and_share_proceeds() {
        let mut s = empty_session();
        let r = s.execute("makerfoundry.shareAnswer", &json!({"choice": "sell", "dontAskAgain": true})).unwrap();
        assert_eq!((r["proceed"].clone(), r["offerSetting"].clone()), (json!(true), json!(false)));
        assert!(!s.prefs.offer_share_after_export);
        s.execute("file.new", &json!({"width": 10, "height": 10})).unwrap();
        assert_eq!(s.execute("makerfoundry.shareOffer", &json!({})).unwrap()["offer"], false);
    }

    #[test]
    fn bad_answers_are_errors() {
        let mut s = empty_session();
        assert!(s.execute("makerfoundry.shareAnswer", &json!({"choice": "maybe"})).is_err());
        assert!(s.execute("makerfoundry.shareAnswer", &json!({"choice": "sell", "dontAskAgain": "yes"})).is_err());
        assert!(s.execute("makerfoundry.shareOffer", &json!({"exportSucceeded": 3})).is_err());
        assert!(Session::new().execute("makerfoundry.shareOffer", &json!({})).is_err());
    }

    #[test]
    fn nothing_open_is_an_error_not_a_crash() {
        let e = Session::new().execute(C, &json!({})).unwrap_err().to_string();
        assert!(e.contains("no document"), "{e}");
    }
}
