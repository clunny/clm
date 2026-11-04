//! `readers`: the files whose `clm:reads` globs match a path that differs.

use std::collections::BTreeMap;

use clm_affected::{Changed, readers, reads_in};

/// The marker's text, built so this file isn't itself a reader.
const MARK: &str = concat!("clm", ":reads ");

fn changed(paths: &[(&str, bool)]) -> Vec<Changed> {
    paths
        .iter()
        .map(|(p, exists)| Changed {
            path: p.to_string(),
            exists: *exists,
        })
        .collect()
}

fn reads(files: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    files
        .iter()
        .map(|(p, globs)| (p.to_string(), globs.iter().map(|g| g.to_string()).collect()))
        .collect()
}

#[test]
fn a_glob_that_matches_a_doc_only_path_selects_its_file() {
    // req:check.affected/affected-readers@94edd10f
    let r = readers(
        &changed(&[("ARCHITECTURE.md", true)]),
        &reads(&[
            ("crates/e/tests/r.rs", &["ARCHITECTURE.md"]),
            ("crates/e/tests/s.rs", &["specs/**"]),
        ]),
    );
    assert_eq!(r, ["crates/e/tests/r.rs"]);
}

#[test]
fn globs_read_as_component_paths_do() {
    // req:check.affected/affected-readers@94edd10f
    let r = readers(
        &changed(&[("specs/a/SPEC.md", true), ("backlog/y.md", true)]),
        &reads(&[
            ("t/b.rs", &["specs/**"]),
            ("t/a.rs", &["backlog/*.md", "nothing"]),
            ("t/c.rs", &["docs/**"]),
        ]),
    );
    assert_eq!(r, ["t/a.rs", "t/b.rs"]);
}

#[test]
fn a_deleted_path_still_selects_its_readers() {
    // req:check.affected/affected-readers@94edd10f
    let r = readers(
        &changed(&[("specs/gone/SPEC.md", false)]),
        &reads(&[("t/a.rs", &["specs/**"])]),
    );
    assert_eq!(r, ["t/a.rs"]);
}

#[test]
fn no_matching_path_selects_nothing() {
    // req:check.affected/affected-readers@94edd10f
    let r = readers(
        &changed(&[("backlog/y.md", true), ("tools/build.sh", true)]),
        &reads(&[("t/a.rs", &["ARCHITECTURE.md"])]),
    );
    assert!(r.is_empty(), "{r:?}");
}

#[test]
fn a_marker_is_the_text_and_the_glob_up_to_whitespace() {
    // req:check.affected/affected-readers@94edd10f
    let text = format!(
        "// {MARK}specs/** and more\n// {MARK}ARCHITECTURE.md\nfn f() {{}} // {MARK}\n// {MARK}  \n// {MARK}\ttabbed\n"
    );
    assert_eq!(reads_in(&text), ["specs/**", "ARCHITECTURE.md"]);
}

#[test]
fn text_without_a_marker_reads_nothing() {
    // req:check.affected/affected-readers@94edd10f
    let text = concat!("// clm:remove", "-by 20261104-x\n// req", ":a/b@12345678\n");
    assert!(reads_in(text).is_empty());
}
