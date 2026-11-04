//! `clm-init` builds for Windows: every Unix-only API sits under `#[cfg(unix)]`
//! (init/windows-build).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds what it looks for.
const UNIX: &str = concat!("std::os::", "unix");
const GATE: &str = concat!("#[cfg(", "unix)]");

/// The lines of `text` naming a Unix-only API outside an item or block marked
/// `#[cfg(unix)]`, by number. An attribute gates the item that follows it: up to its `;` when
/// it opens no brace first, else to the brace that closes the one it opens.
fn ungated(text: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    // Depths of the open gated blocks, and whether an attribute waits for its item.
    let mut gated: Vec<usize> = Vec::new();
    let mut pending = false;
    for (n, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        if code.trim_start().starts_with(GATE) {
            pending = true;
            continue;
        }
        if line.contains(UNIX) && !pending && gated.is_empty() {
            found.push(n + 1);
        }
        for c in code.chars() {
            match c {
                '{' => {
                    if pending {
                        gated.push(depth);
                        pending = false;
                    }
                    depth += 1;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if gated.last() == Some(&depth) {
                        gated.pop();
                    }
                }
                ';' if pending => pending = false,
                _ => {}
            }
        }
    }
    found
}

#[test]
fn unix_apis_gated() {
    // req:init/windows-build@03cde96d
    let mut found = Vec::new();
    let files = rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(!files.is_empty(), "no source files read");
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for n in ungated(&text) {
            found.push(format!("{}:{n}", file.display()));
        }
    }
    assert!(
        found.is_empty(),
        "Unix-only APIs not under cfg(unix): {found:?}"
    );
}

#[test]
fn the_scan_tells_gated_from_ungated() {
    // req:init/windows-build@03cde96d
    let api = format!("use {UNIX}::fs::PermissionsExt;");
    assert_eq!(ungated(&format!("{api}\nfn f() {{}}\n")), [1]);
    assert_eq!(
        ungated(&format!("{GATE}\n{api}\nfn f() {{}}\n")),
        [] as [usize; 0]
    );
    let gated_fn = format!("{GATE}\nfn f() {{\n    {api}\n    if x {{\n    }}\n    {api}\n}}\n");
    assert_eq!(ungated(&gated_fn), [] as [usize; 0]);
    // After the gated item closes, the next use is ungated again.
    assert_eq!(
        ungated(&format!("{gated_fn}fn g() {{\n    {api}\n}}\n")),
        [9]
    );
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
