//! The `Replica` Durable Object class and the platform it runs on (`object-class`).

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under the crate's `src`.
fn sources() -> Vec<(PathBuf, String)> {
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    let mut out = Vec::new();
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path).unwrap();
                out.push((path, text));
            }
        }
    }
    out
}

/// The identifiers in `text`: runs of ASCII letters, digits and `_`.
fn identifiers(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

#[test]
fn the_class_is_defined() {
    // req:web.worker.replica/object-class@2ce37c07
    // The wasm32 build itself is `ci-check.sh`'s; here the class is read from the sources.
    let defined = sources()
        .iter()
        .any(|(_, text)| text.contains("#[durable_object]\npub struct Replica {"));
    assert!(defined, "no `#[durable_object]` on a type named `Replica`");
}

#[test]
fn no_source_names_a_cache() {
    // req:web.worker.replica/object-class@2ce37c07
    let sources = sources();
    assert!(!sources.is_empty(), "no sources read");
    let mut found = Vec::new();
    for (path, text) in &sources {
        for word in identifiers(text).filter(|w| ["Kv", "kv", "Cache", "KvStore"].contains(w)) {
            found.push(format!("{}: {word}", path.display()));
        }
    }
    assert!(found.is_empty(), "the object names the caches: {found:?}");
}
