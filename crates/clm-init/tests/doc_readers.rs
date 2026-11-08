//! Tests that read this repository's own doc-only files say so (`doc-readers-marked`).

use std::fs;
use std::path::Path;

/// The marker's text, built so this file isn't itself a reader.
const MARK: &str = concat!("clm", ":reads ");

/// The prefix a test reads the repository's tree through, built so this file doesn't hold it.
const UP: &str = concat!("..", "/", "..", "/");

/// The doc-only paths a test here could read (`ledger`'s `doc_only`).
const DOC_ONLY: [&str; 10] = [
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "CLAUDE.md",
    "README.md",
    "specs/",
    "changes/",
    "backlog/",
    "docs/decisions/",
    "docs/guide/",
    "docs/spec-format.md",
];

/// The text of a source that joins a path onto the repository's root, built so this file
/// doesn't hold it.
const ROOT: &str = concat!("\"..", "/", "..\"");

/// Globs of the markers in `text`.
fn globs(text: &str) -> Vec<&str> {
    text.match_indices(MARK)
        .map(|(i, m)| &text[i + m.len()..])
        .filter(|rest| !rest.starts_with(char::is_whitespace))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect()
}

/// Whether `glob` names `path`: exactly, as `**`, or as `dir/**` over a path beneath `dir`.
fn covers(glob: &str, path: &str) -> bool {
    glob == path
        || glob == "**"
        || glob
            .strip_suffix("/**")
            .is_some_and(|d| path.starts_with(&format!("{d}/")))
}

/// The `.rs` files of `dir`, beneath it too.
fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Whether `text` reads `doc` of the repository's tree: through the `../../` prefix, or by
/// joining a file name onto the root and naming the path.
fn reads(text: &str, doc: &str) -> bool {
    let reads_files = text.contains("read_to_string") || text.contains("read_dir");
    reads_files
        && (text.contains(&format!("{UP}{doc}"))
            || (doc.starts_with("docs/") && text.contains(ROOT) && text.contains(doc)))
}

#[test]
fn a_test_reading_a_doc_only_path_holds_a_marker_for_it() {
    // req:init/doc-readers-marked@7b4edd3f
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("tests"), &mut files);
    rust_files(&root.join("src"), &mut files);
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        let marked = globs(&text);
        for doc in DOC_ONLY {
            let name = doc.trim_end_matches('/');
            assert!(
                !reads(&text, doc) || marked.iter().any(|g| covers(g, doc) || covers(g, name)),
                "{} reads {doc} without a marker for it",
                path.display()
            );
        }
    }
}
