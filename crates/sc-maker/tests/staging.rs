#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The staging switch changes every address at once, and back. One test: the switch is process-wide.

use sc_maker::{Site, account::Accounts, link, publish::Publish};

#[test]
fn production_is_the_default_and_staging_switches_both_addresses() {
    assert!(!sc_maker::staging());
    assert_eq!(Site::makerfoundry().origin(), "https://makerfoundry.app");
    assert_eq!(Site::accounts().origin(), "https://cloud.stitchfoundry.app");
    let staged_link = "https://makerfoundry-test-test.up.railway.app/api/v1/designs/1/file/a.svg";
    assert!(link::open_link(staged_link).is_err());

    sc_maker::use_staging(true);
    assert_eq!(Site::makerfoundry().origin(), "https://makerfoundry-test-test.up.railway.app");
    assert_eq!(Site::accounts().origin(), "https://cloud-staging-staging.up.railway.app");
    assert!(link::open_link(staged_link).is_ok());
    let signin = Accounts::new(Site::accounts()).sign_in("a@example.com", "pw").unwrap();
    assert_eq!(signin.url, "https://cloud-staging-staging.up.railway.app/v1/signin");
    assert!(signin.expected().is_ok());
    let publish = Publish::new(Site::makerfoundry(), "a.svg", "PHN2Zy8+".into(), "ScreenFoundry").unwrap();
    let request = publish.request("sfs_token").unwrap();
    assert!(request.url.starts_with("https://makerfoundry-test-test.up.railway.app/"), "{}", request.url);
    assert!(request.expected().is_ok());

    sc_maker::use_staging(false);
    assert_eq!(Site::makerfoundry().origin(), "https://makerfoundry.app");
    assert!(request.expected().is_err(), "a staging request is refused in production");
}
