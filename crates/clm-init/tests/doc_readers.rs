//! Tests that read this repository's own doc-only files say so (`doc-readers-marked`).

use std::fs;
use std::path::Path;

/// The marker's text, built so this file isn't itself a reader.
const MARK: &str = concat!("clm", ":reads ");

/// The prefix a test reads the repository's tree through, built so this file doesn't hold it.
const UP: &str = concat!("..", "/", "..", "/");

/// The doc-only paths a test here could read (`ledger`'s `doc_only`).
const DOC_ONLY: [&str; 8] = [
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "CLAUDE.md",
    "README.md",
    "specs/",
    "changes/",
    "backlog/",
    "docs/decisions/",
];

/// Globs of the markers in `text`.
fn globs(text: &str) -> Vec<&str> {
    text.match_indices(MARK)
        .filter_map(|(i, m)| text[i + m.len()..].split_whitespace().next())
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

#[test]
fn a_test_reading_a_doc_only_path_holds_a_marker_for_it() {
    // req:init/doc-readers-marked@c6719af2
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        let marked = globs(&text);
        for doc in DOC_ONLY {
            let reads = text.contains(&format!("{UP}{doc}"));
            let name = doc.trim_end_matches('/');
            assert!(
                !reads || marked.iter().any(|g| covers(g, doc) || covers(g, name)),
                "{} reads {doc} without a marker for it",
                path.display()
            );
        }
    }
}

#[test]
fn covers_reads_names_and_directories() {
    // req:init/doc-readers-marked@c6719af2
    assert!(covers("ARCHITECTURE.md", "ARCHITECTURE.md"));
    assert!(covers("specs/**", "specs/a/SPEC.md"));
    assert!(!covers("specs/**", "backlog/x.md"));
    assert_eq!(globs(&format!("x {MARK}a/** y\n{MARK}b")), ["a/**", "b"]);
}
