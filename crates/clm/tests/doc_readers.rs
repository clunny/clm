//! The tests that read this repository's own doc-only files name them in a reads marker, so a
//! land that changes one runs them (clm/doc-readers-marked).

// clm:reads specs/**

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
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["specs/SPEC.md"]), &markers());
    assert!(
        selected.contains(&"crates/clm/tests/retired_terms.rs".to_string()),
        "{selected:?}"
    );
}

#[test]
fn the_root_documents_have_readers() {
    // req:clm/doc-readers-marked@3125f2de
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

#[test]
fn the_readers_of_clm_toml() {
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["clm.toml"]), &markers());
    for file in ["crates/clm/tests/cli.rs", "crates/clm/tests/ci_check.rs"] {
        assert!(selected.contains(&file.to_string()), "{selected:?}");
    }
}

#[test]
fn the_guides_readers() {
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["docs/guide/glossary.md"]), &markers());
    for file in [
        "crates/clm/tests/glossary.rs",
        "crates/clm/tests/guide.rs",
        "crates/clm/tests/retired_terms.rs",
    ] {
        assert!(selected.contains(&file.to_string()), "{selected:?}");
    }
}

#[test]
fn the_spec_formats_readers() {
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["docs/spec-format.md"]), &markers());
    for file in [
        "crates/clm/tests/guide.rs",
        "crates/clm/tests/retired_terms.rs",
        "crates/clm-init/src/guide.rs",
    ] {
        assert!(selected.contains(&file.to_string()), "{selected:?}");
    }
}

#[test]
fn other_crates_read_architecture_md() {
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["ARCHITECTURE.md"]), &markers());
    for file in [
        "crates/clm-init/tests/says_component.rs",
        "crates/clm-model/tests/carried.rs",
        "crates/clm-context/tests/carried.rs",
    ] {
        assert!(selected.contains(&file.to_string()), "{selected:?}");
    }
}

#[test]
fn the_guides_readers_in_other_crates() {
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["docs/guide/quickstart.md"]), &markers());
    for file in [
        "crates/clm-init/src/guide.rs",
        "crates/clm-init/src/guide/entry_pages.rs",
    ] {
        assert!(selected.contains(&file.to_string()), "{selected:?}");
    }
}

#[test]
fn the_settings_pages_reader() {
    // req:clm/doc-readers-marked@3125f2de
    let selected = readers(&differing(&["docs/guide/settings.md"]), &markers());
    assert!(
        selected.contains(&"crates/clm-config/tests/settings_guide.rs".to_string()),
        "{selected:?}"
    );
}
