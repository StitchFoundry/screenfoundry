#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Settings > MakerFoundry > "Offer to share after export" (add-on 37): on by default, settable.

use serde_json::json;
use vectorcraft_engine::Session;

#[test]
fn offer_share_after_export_is_on_by_default_and_settable() {
    let mut s = Session::new();
    assert!(s.prefs.offer_share_after_export);
    s.execute("prefs.set", &json!({"key": "offerShareAfterExport", "value": false})).unwrap();
    assert!(!s.prefs.offer_share_after_export);
    s.execute("prefs.reset", &json!({})).unwrap();
    assert!(s.prefs.offer_share_after_export);
}
