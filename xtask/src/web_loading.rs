//! `cargo xtask web-loading [index.html]`: tells the loading screen how big the `.wasm` is.
//!
//! The server sends the wasm gzip-compressed, so its `Content-Length` is the compressed size while
//! the browser counts decompressed bytes as they arrive: a progress bar built on the header would
//! run past 100%. `apps/vectorcraft-web/index.html` therefore carries `data-wasm-bytes="__WASM_BYTES__"`
//! on the loading element, and this Trunk `post_build` hook writes the built file's real size there
//! (`apps/vectorcraft-web/loader.js` reads it).

use std::path::{Path, PathBuf};

/// Where the size goes in the source page.
const PLACEHOLDER: &str = "__WASM_BYTES__";

/// The command: the page is the first argument, else Trunk's staging folder's, else `dist/web`.
pub fn run(root: &Path, args: &[&str]) -> Result<(), String> {
    let given = args.iter().find(|a| !a.starts_with("--"));
    let staging = std::env::var_os("TRUNK_STAGING_DIR").filter(|v| !v.is_empty()).map(PathBuf::from);
    let page = match (given, &staging) {
        (Some(p), _) => root.join(p),
        (None, Some(s)) => s.join("index.html"),
        (None, None) => root.join("dist/web/index.html"),
    };
    let html = std::fs::read_to_string(&page).map_err(|e| format!("{}: {e}", page.display()))?;
    if !html.contains(PLACEHOLDER) {
        println!("web-loading: {} has no {PLACEHOLDER}; nothing to do", page.display());
        return Ok(());
    }
    let dir = page.parent().ok_or("the page has no folder")?;
    let bytes = wasm_size(dir)?;
    std::fs::write(&page, fill(&html, bytes)).map_err(|e| format!("{}: {e}", page.display()))?;
    println!("web-loading: the loading screen expects {bytes} bytes of wasm ({})", page.display());
    Ok(())
}

/// The size of the one `*_bg.wasm` in `dir`.
fn wasm_size(dir: &Path) -> Result<u64, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut sizes = vec![];
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().ends_with("_bg.wasm") {
            sizes.push(entry.metadata().map_err(|e| format!("{}: {e}", entry.path().display()))?.len());
        }
    }
    match sizes.as_slice() {
        [one] => Ok(*one),
        other => Err(format!("expected one *_bg.wasm in {}, found {}", dir.display(), other.len())),
    }
}

/// `html` with the placeholder replaced by `bytes`.
fn fill(html: &str, bytes: u64) -> String {
    html.replace(PLACEHOLDER, &bytes.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = include_str!("../../apps/vectorcraft-web/index.html");

    #[test]
    fn the_source_page_has_the_placeholder_on_the_loading_element() {
        assert!(SOURCE.contains(&format!("id=\"vectorcraft_loading\" data-wasm-bytes=\"{PLACEHOLDER}\"")));
    }

    #[test]
    fn the_size_replaces_the_placeholder() {
        let page = fill(SOURCE, 12_345);
        assert!(page.contains("data-wasm-bytes=\"12345\"") && !page.contains(PLACEHOLDER));
    }

    #[test]
    fn the_loader_reads_the_attribute_the_page_writes() {
        let loader = include_str!("../../apps/vectorcraft-web/loader.js");
        assert!(loader.contains("dataset.wasmBytes"));
    }

    #[test]
    fn the_wasm_is_found_by_its_name() {
        let dir = std::env::temp_dir().join(format!("web-loading-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app-1_bg.wasm"), [0u8; 7]).unwrap();
        std::fs::write(dir.join("app-1.js"), [0u8; 3]).unwrap();
        assert_eq!(wasm_size(&dir).unwrap(), 7);
        std::fs::write(dir.join("app-2_bg.wasm"), [0u8; 1]).unwrap();
        assert!(wasm_size(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
