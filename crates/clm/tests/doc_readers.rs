//! The tests that read this repository's own doc-only files name them in a reads marker, so a
//! land that changes one runs them (clm/doc-readers-marked).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clm_affected::{Changed, readers, reads_in};
use clm_git::{Options, run};

/// Each file `check`'s coverage scan reads in this repository's tree (tracked or untracked and
/// not ignored, UTF-8, not Markdown) that holds a reads marker, with its globs.
fn markers() -> BTreeMap<String, Vec<String>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let listed = run(
        &root,
        "git",
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        Options::default(),
    )
    .unwrap();
    let mut out = BTreeMap::new();
    for name in listed.split(|&b| b == 0).filter(|n| !n.is_empty()) {
        let path = String::from_utf8_lossy(name).into_owned();
        if path.ends_with(".md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        let globs = reads_in(&text);
        if !globs.is_empty() {
            out.insert(path, globs);
        }
    }
    out
}

fn differing(paths: &[&str]) -> Vec<Changed> {
    paths
        .iter()
        .map(|p| Changed {
            path: (*p).to_string(),
            exists: true,
        })
        .collect()
}

#[test]
fn the_retired_terms_test_reads_the_specs() {
    // req:clm/doc-readers-marked@54c98381
    let selected = readers(&differing(&["specs/SPEC.md"]), &markers());
    assert!(
        selected.contains(&"crates/clm/tests/retired_terms.rs".to_string()),
        "{selected:?}"
    );
}

#[test]
fn the_root_documents_have_readers() {
    // req:clm/doc-readers-marked@54c98381
    let all = markers();
    for doc in [
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "README.md",
    ] {
        let selected = readers(&differing(&[doc]), &all);
        assert!(
            selected.contains(&"crates/clm/tests/retired_terms.rs".to_string()),
            "{doc}: {selected:?}"
        );
    }
    let selected = readers(&differing(&["README.md"]), &all);
    assert!(
        selected.contains(&"crates/clm/tests/readme.rs".to_string()),
        "{selected:?}"
    );
}
