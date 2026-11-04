//! Writes `$OUT_DIR/guide_pages.rs`, `clm guide`'s table of pages, one `include_str!` per
//! `.md` file of `docs/guide/` and `docs/spec-format.md` (`guide-table-from-files`), so a
//! page added to the guide is in the next build with no edit under `crates/`.

use std::path::PathBuf;

#[path = "src/guide/table.rs"]
mod table;

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let docs = manifest.join("../../docs");
    let guide_dir = docs.join("guide");
    let spec_format = docs.join("spec-format.md");
    // A script that prints the directive at all is rerun for these paths alone; Cargo
    // scans a directory whole, so adding or removing a page reruns it.
    for path in [
        guide_dir.clone(),
        spec_format.clone(),
        manifest.join("build.rs"),
        manifest.join("src/guide/table.rs"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let pages = table::pages(&guide_dir, &spec_format).unwrap_or_else(|e| panic!("{e}"));
    let mut out = format!("const PAGES: [(&str, &str); {}] = [\n", pages.len());
    for (name, file) in &pages {
        // `{:?}` escapes a Windows path's backslashes.
        out.push_str(&format!("    ({name:?}, include_str!({file:?})),\n"));
    }
    out.push_str("];\n");
    let dest = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("guide_pages.rs");
    std::fs::write(dest, out).unwrap();
}
