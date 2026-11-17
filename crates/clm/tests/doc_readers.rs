//! The tests that read this repository's own doc-only files name them in a reads marker, so a
//! land that changes one runs them (clm/doc-readers-marked).

// clm:reads specs/**

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use clm_affected::{Changed, readers, reads_in};
use clm_git::{Options, run};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Each file `check`'s coverage scan reads in this repository's tree (tracked or untracked and
/// not ignored, UTF-8, not Markdown) that holds a reads marker, with its globs.
fn markers() -> BTreeMap<String, Vec<String>> {
    let root = root();
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

/// The files `readers` selects from this repository's markers when `paths` differ.
fn selected(paths: &[&str]) -> Vec<String> {
    let differing: Vec<Changed> = paths
        .iter()
        .map(|p| Changed {
            path: (*p).to_string(),
            exists: true,
        })
        .collect();
    readers(&differing, &markers())
}

/// Asserts `selected` holds each of `wanted` and none of `unwanted`, each a reader of other
/// documents only, so markers that matched every path fail as markers that matched none do.
fn assert_selects(selected: &[String], wanted: &[&str], unwanted: &[&str]) {
    for file in wanted {
        assert!(selected.iter().any(|s| s == file), "{file}: {selected:?}");
    }
    for file in unwanted {
        assert!(!selected.iter().any(|s| s == file), "{file}: {selected:?}");
    }
}

/// The file holding the tests of `clm guide`, which read the guide's pages and the spec format.
/// They sit in the root's `guide.rs` until `init`'s build moves them into its own crate
/// (`commands-live-in-their-crates`), so it is whichever of the two the tree holds.
fn guide_tests() -> &'static str {
    let moved = "crates/clm-init/tests/guide.rs";
    if root().join(moved).is_file() {
        moved
    } else {
        "crates/clm/tests/guide.rs"
    }
}

const README: &str = "crates/clm/tests/readme.rs";
const GLOSSARY: &str = "crates/clm/tests/glossary.rs";
const RETIRED: &str = "crates/clm/tests/retired_terms.rs";
const SETTINGS: &str = "crates/clm-config/tests/settings_guide.rs";

#[test]
fn the_retired_terms_test_reads_the_specs() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(&selected(&["specs/SPEC.md"]), &[RETIRED], &[README]);
}

#[test]
fn the_root_documents_have_readers() {
    // req:clm/doc-readers-marked@3125f2de
    let docs = [
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "README.md",
    ];
    assert_selects(&selected(&docs), &[RETIRED, README], &[GLOSSARY]);
    for doc in &docs[..3] {
        assert_selects(&selected(&[doc]), &[RETIRED], &[README]);
    }
}

#[test]
fn the_readers_of_clm_toml() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(
        &selected(&["clm.toml"]),
        &["crates/clm/tests/cli.rs", "crates/clm/tests/ci_check.rs"],
        &[README, GLOSSARY, RETIRED],
    );
}

#[test]
fn the_guides_readers() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(
        &selected(&["docs/guide/glossary.md"]),
        &[GLOSSARY, RETIRED, guide_tests()],
        &[README, SETTINGS],
    );
}

#[test]
fn the_spec_formats_readers() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(
        &selected(&["docs/spec-format.md"]),
        &[RETIRED, "crates/clm-init/src/guide.rs", guide_tests()],
        &[GLOSSARY, SETTINGS],
    );
}

#[test]
fn other_crates_read_architecture_md() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(
        &selected(&["ARCHITECTURE.md"]),
        &[
            "crates/clm-init/tests/says_component.rs",
            "crates/clm-model/tests/carried.rs",
            "crates/clm-context/tests/carried.rs",
        ],
        &[GLOSSARY, SETTINGS],
    );
}

#[test]
fn the_guides_readers_in_other_crates() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(
        &selected(&["docs/guide/quickstart.md"]),
        &[
            "crates/clm-init/src/guide.rs",
            "crates/clm-init/src/guide/entry_pages.rs",
        ],
        &[GLOSSARY, SETTINGS],
    );
}

#[test]
fn the_settings_pages_reader() {
    // req:clm/doc-readers-marked@3125f2de
    assert_selects(
        &selected(&["docs/guide/settings.md"]),
        &[SETTINGS],
        &[GLOSSARY, README],
    );
}
