//! `replica/files-it-fetches`: which files the replica reads, and what it still needs.

mod common;

use clm_replica::{Change, Entry, File, Replica, keeps_text, wanted};
use common::*;

#[test]
fn which_files_the_replica_reads() {
    // req:replica/files-it-fetches@9a81a118
    let paths = [
        "specs/a/SPEC.md",
        "changes/archive/x/deltas/a.md",
        "docs/decisions/d.md",
        "backlog/i.md",
        "CAPABILITIES.md",
        "clm.toml",
        ".gitattributes",
        "web/.gitattributes",
        "src/lib.rs",
        "README.md",
        "docs/guide.md",
    ];
    let kept: Vec<bool> = paths.iter().map(|p| keeps_text(p)).collect();
    let read: Vec<bool> = paths.iter().map(|p| wanted(p)).collect();
    assert_eq!(
        kept,
        [
            true, true, true, true, true, true, true, true, false, false, false
        ]
    );
    assert_eq!(
        read,
        [
            true, true, true, true, true, true, true, true, true, false, false
        ]
    );
    assert!(keeps_text("ARCHITECTURE.md"));
}

#[test]
fn what_is_still_to_fetch() {
    // req:replica/files-it-fetches@9a81a118
    let listed = |path: &str, mode: &str| Entry {
        mode: mode.into(),
        ..entry(path, path.as_bytes())
    };
    let mut r = Replica::default();
    r.apply(Change::Tree {
        sha: "111".into(),
        entries: vec![
            listed("specs/a/SPEC.md", "100644"),
            listed("src/b.rs", "100755"),
            listed("README.md", "100644"),
            listed("a/link", "120000"),
            listed("vendor/x", "160000"),
        ],
    });
    assert_eq!(r.pending_fetch(), ["specs/a/SPEC.md", "src/b.rs"]);
    // A file carried with no text and not marked binary was read of nothing: still to fetch.
    r.apply(Change::Tip {
        sha: "111".into(),
        files: vec![File {
            entry: listed("src/b.rs", "100755"),
            binary: false,
            text: None,
        }],
        removed: Vec::new(),
    });
    assert_eq!(r.pending_fetch(), ["specs/a/SPEC.md", "src/b.rs"]);

    // The files whose text it keeps come first, whatever their paths' order.
    let mut later = Replica::default();
    later.apply(Change::Tree {
        sha: "111".into(),
        entries: vec![
            listed("a/x.rs", "100644"),
            listed("specs/a/SPEC.md", "100644"),
            listed("clm.toml", "100644"),
        ],
    });
    assert_eq!(
        later.pending_fetch(),
        ["clm.toml", "specs/a/SPEC.md", "a/x.rs"]
    );
}
