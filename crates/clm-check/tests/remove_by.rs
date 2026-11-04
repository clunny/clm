use std::fs;
use std::path::Path;

use clm_check::run;
use tempfile::TempDir;

// Removal markers are assembled so this file doesn't itself hold one.
const DROP_OLD: &str = concat!("clm:remove", "-by 20261023-drop-old");
const NOWHERE: &str = concat!("clm:remove", "-by 20261023-nowhere");
const PLACEHOLDER: &str = concat!("clm:remove", "-by <change-id>");

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A repo whose `src/old.rs` holds `marker` on line 4.
fn repo(marker: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(
        dir.path(),
        "src/old.rs",
        &format!("fn a() {{}}\n\n// The old path.\n// {marker}\nfn old() {{}}\n"),
    );
    dir
}

fn change(root: &Path, dir: &str, status: &str) {
    write(
        root,
        &format!("{dir}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
}

/// `(path, message)` of each `check/remove-by-markers` violation.
fn found(root: &Path) -> Vec<(String, String)> {
    run(root)
        .into_iter()
        .filter(|v| v.rule == "check/remove-by-markers")
        .map(|v| (v.path.display().to_string(), v.message))
        .collect()
}

#[test]
fn marker_for_done_change_fails() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = repo(DROP_OLD);
    change(dir.path(), "changes/20261023-drop-old", "done");
    assert_eq!(
        found(dir.path()),
        [(
            "src/old.rs:4".to_string(),
            "marker names 20261023-drop-old, which is done".to_string()
        )]
    );
}

#[test]
fn marker_for_abandoned_change_fails() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = repo(DROP_OLD);
    change(dir.path(), "changes/20261023-drop-old", "abandoned");
    assert_eq!(
        found(dir.path()),
        [(
            "src/old.rs:4".to_string(),
            "marker names 20261023-drop-old, which is abandoned".to_string()
        )]
    );
}

#[test]
fn marker_for_archived_change_fails() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = repo(DROP_OLD);
    change(dir.path(), "changes/archive/20261023-drop-old", "done");
    assert_eq!(
        found(dir.path()),
        [(
            "src/old.rs:4".to_string(),
            "marker names 20261023-drop-old, which is archived".to_string()
        )]
    );
}

#[test]
fn marker_for_missing_change_fails() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = repo(NOWHERE);
    assert_eq!(
        found(dir.path()),
        [(
            "src/old.rs:4".to_string(),
            "marker names 20261023-nowhere, which doesn't exist".to_string()
        )]
    );
}

#[test]
fn marker_for_directory_without_change_md_is_missing() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = repo(NOWHERE);
    write(dir.path(), "changes/20261023-nowhere/notes.txt", "draft\n");
    let found = found(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].1,
        "marker names 20261023-nowhere, which doesn't exist"
    );
}

#[test]
fn marker_for_live_change_passes() {
    // req:check/remove-by-markers@4ca98d5a
    for status in ["proposed", "approved", "applied", "implementing"] {
        let dir = repo(DROP_OLD);
        change(dir.path(), "changes/20261023-drop-old", status);
        assert_eq!(found(dir.path()), [], "{status}");
    }
}

#[test]
fn markdown_and_placeholders_are_not_markers() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "docs/x.md", &format!("See {NOWHERE}.\n"));
    write(
        dir.path(),
        "src/help.rs",
        &format!("const HELP: &str = \"mark with {PLACEHOLDER}\";\n"),
    );
    assert_eq!(found(dir.path()), []);
}

#[test]
fn malformed_ids_are_not_markers() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    let text = [
        concat!("x", "clm:remove", "-by 20261023-nowhere"), // follows a word character
        concat!("clm:remove", "-by 2026102-nowhere"),       // seven digits
        concat!("clm:remove", "-by 202610233-nowhere"),     // nine digits
        concat!("clm:remove", "-by 20261023-"),             // empty slug
        concat!("clm:remove", "-by 20261023nowhere"),       // no dash
    ]
    .join("\n");
    write(dir.path(), "src/a.rs", &text);
    assert_eq!(found(dir.path()), []);
}

#[test]
fn marker_id_stops_at_punctuation() {
    // req:check/remove-by-markers@4ca98d5a
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "src/a.rs", &format!("// ({NOWHERE}).\n"));
    assert_eq!(
        found(dir.path()),
        [(
            "src/a.rs:1".to_string(),
            "marker names 20261023-nowhere, which doesn't exist".to_string()
        )]
    );
}
