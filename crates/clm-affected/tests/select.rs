//! `select`: the nodes and files a set of changed paths affects, from the graph alone.

use std::collections::BTreeMap;

use clm_affected::{Changed, Selection, select};
use clm_model::Graph;

/// A node's `SPEC.md`: `paths` and `depends_on` as YAML lists.
fn spec(paths: &str, depends_on: &str) -> Vec<u8> {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n---\n"
    )
    .into_bytes()
}

/// The root (holding `Cargo.lock`, depending on nothing), `a`, `b` on `a`, `c` on `b`, and
/// `d` and `e` on nothing, each holding `crates/<id>/**`.
fn graph() -> Graph {
    let files: clm_model::Files = [
        ("specs/SPEC.md", spec("[\"Cargo.lock\"]", "[]")),
        ("specs/a/SPEC.md", spec("[\"crates/a/**\"]", "[]")),
        ("specs/b/SPEC.md", spec("[\"crates/b/**\"]", "[a]")),
        ("specs/c/SPEC.md", spec("[\"crates/c/**\"]", "[b]")),
        ("specs/d/SPEC.md", spec("[\"crates/d/**\"]", "[]")),
        ("specs/e/SPEC.md", spec("[\"crates/e/**\"]", "[]")),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t))
    .collect();
    let (graph, violations) = clm_model::load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn changed(paths: &[&str]) -> Vec<Changed> {
    paths
        .iter()
        .map(|p| Changed {
            path: p.to_string(),
            exists: true,
        })
        .collect()
}

/// Scanned files and the requirement IDs each cites.
fn scanned(files: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    files
        .iter()
        .map(|(p, ids)| (p.to_string(), ids.iter().map(|i| i.to_string()).collect()))
        .collect()
}

fn nodes(selection: &Selection) -> Vec<&str> {
    match selection {
        Selection::Nodes { nodes, .. } => nodes.iter().map(String::as_str).collect(),
        Selection::All(path) => panic!("all {path}"),
    }
}

fn files(selection: &Selection) -> Vec<&str> {
    match selection {
        Selection::Nodes { files, .. } => files.iter().map(String::as_str).collect(),
        Selection::All(path) => panic!("all {path}"),
    }
}

#[test]
fn owners_and_their_dependants() {
    // req:check.affected/affected-components@b287ecd8
    let s = select(
        &graph(),
        &changed(&["crates/a/src/lib.rs"]),
        &scanned(&[]),
        &[],
    );
    assert_eq!(nodes(&s), ["a", "b", "c"]);
}

#[test]
fn a_dependant_alone_affects_no_dependency() {
    // req:check.affected/affected-components@b287ecd8
    let s = select(
        &graph(),
        &changed(&["crates/b/src/lib.rs"]),
        &scanned(&[]),
        &[],
    );
    assert_eq!(nodes(&s), ["b", "c"]);
}

#[test]
fn nothing_changed_selects_nothing() {
    // req:check.affected/affected-components@b287ecd8
    let s = select(&graph(), &[], &scanned(&[("crates/a/x.rs", &[])]), &[]);
    assert_eq!(
        s,
        Selection::Nodes {
            nodes: vec![],
            files: vec![]
        }
    );
}

#[test]
fn cited_owned_and_changed_files() {
    // req:check.affected/affected-files@0b773abd
    let files_scanned = scanned(&[
        ("crates/a/src/lib.rs", &[]),
        ("crates/a/src/util.rs", &[]),
        ("crates/d/src/lib.rs", &[]),
        ("crates/e/tests/x.rs", &["c/one", "d/two"]),
        ("crates/e/tests/y.rs", &["d/two"]),
        ("crates/e/tests/z.rs", &["d/two"]),
    ]);
    let s = select(
        &graph(),
        &changed(&["crates/e/tests/z.rs", "crates/a/src/lib.rs"]),
        &files_scanned,
        &[],
    );
    assert_eq!(nodes(&s), ["a", "b", "c", "e"]);
    assert_eq!(
        files(&s),
        [
            "crates/a/src/lib.rs",
            "crates/a/src/util.rs",
            "crates/e/tests/x.rs",
            "crates/e/tests/z.rs"
        ]
    );
}

#[test]
fn a_marker_naming_a_removed_requirement_selects_by_its_node() {
    // req:check.affected/affected-files@0b773abd
    let s = select(
        &graph(),
        &changed(&["crates/b/src/lib.rs"]),
        &scanned(&[("crates/e/tests/old.rs", &["b/gone"])]),
        &[],
    );
    assert_eq!(files(&s), ["crates/b/src/lib.rs", "crates/e/tests/old.rs"]);
}

#[test]
fn a_deleted_file_affects_its_owner_without_a_file_line() {
    // req:check.affected/affected-files@0b773abd
    let gone = Changed {
        path: "crates/a/src/old.rs".into(),
        exists: false,
    };
    let s = select(
        &graph(),
        &[gone],
        &scanned(&[("crates/a/src/x.rs", &[])]),
        &[],
    );
    assert_eq!(nodes(&s), ["a", "b", "c"]);
    assert_eq!(files(&s), ["crates/a/src/x.rs"]);
}

#[test]
fn a_file_no_node_holds_selects_everything() {
    // req:check.affected/affected-all@c4a42acf
    let s = select(
        &graph(),
        &changed(&["tools/build.sh", "crates/a/src/lib.rs", "zz/other"]),
        &scanned(&[]),
        &[],
    );
    assert_eq!(s, Selection::All("tools/build.sh".into()));
}

#[test]
fn a_file_the_setting_names_selects_everything() {
    // req:check.affected/affected-all@c4a42acf
    let s = select(
        &graph(),
        &changed(&["Cargo.lock"]),
        &scanned(&[]),
        &["Cargo.lock".to_string()],
    );
    assert_eq!(s, Selection::All("Cargo.lock".into()));
    let s = select(
        &graph(),
        &changed(&["crates/d/src/x.rs", "crates/c/src/x.rs"]),
        &scanned(&[]),
        &["crates/d/**".to_string()],
    );
    assert_eq!(s, Selection::All("crates/d/src/x.rs".into()));
}

#[test]
fn a_deleted_file_no_node_holds_selects_everything() {
    // req:check.affected/affected-all@c4a42acf
    let gone = Changed {
        path: "tools/build.sh".into(),
        exists: false,
    };
    let s = select(&graph(), &[gone], &scanned(&[]), &[]);
    assert_eq!(s, Selection::All("tools/build.sh".into()));
}

#[test]
fn doc_only_paths_select_nothing() {
    // req:check.affected/doc-only-selects-nothing@d8831f2e
    let s = select(
        &graph(),
        &changed(&["docs/decisions/x.md", "backlog/y.md", "specs/a/SPEC.md"]),
        &scanned(&[("crates/a/x.rs", &[])]),
        &[],
    );
    assert_eq!(
        s,
        Selection::Nodes {
            nodes: vec![],
            files: vec![]
        }
    );
}

#[test]
fn doc_only_paths_beside_code_select_what_the_code_does() {
    // req:check.affected/doc-only-selects-nothing@d8831f2e
    let s = select(
        &graph(),
        &changed(&["backlog/y.md", "crates/d/src/lib.rs"]),
        &scanned(&[("crates/d/src/lib.rs", &[]), ("crates/d/src/x.rs", &[])]),
        &[],
    );
    assert_eq!(nodes(&s), ["d"]);
    assert_eq!(files(&s), ["crates/d/src/lib.rs", "crates/d/src/x.rs"]);
}

#[test]
fn a_component_whose_tests_name_a_changed_one_is_not_affected() {
    // req:check.affected/affected-components@b287ecd8
    let doc = |paths: &str, extra: &str| {
        format!("---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: []\n{extra}---\n")
            .into_bytes()
    };
    let files: clm_model::Files = [
        ("specs/SPEC.md", doc("[\"Cargo.lock\"]", "")),
        ("specs/app/SPEC.md", doc("[\"crates/app/**\"]", "")),
        (
            "specs/feat/SPEC.md",
            doc("[\"crates/feat/**\"]", "test_depends_on: [app]\n"),
        ),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t))
    .collect();
    let (graph, violations) = clm_model::load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    let s = select(
        &graph,
        &changed(&["crates/app/src/lib.rs"]),
        &scanned(&[]),
        &[],
    );
    assert_eq!(nodes(&s), ["app"]);
}

#[test]
fn a_file_an_exemption_names_is_selected_whenever_a_component_is_affected() {
    // req:check.affected/affected-files@0b773abd
    let doc = |paths: &str, extra: &str| {
        format!("---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: []\n{extra}---\n")
            .into_bytes()
    };
    let exempt = "exemptions:\n  - rule: undeclared-imports\n    files: [\"crates/feat/tests/t.rs\"]\n    until: backlog/x\n  - rule: skipped-tests\n    files: [\"crates/feat/tests/s.rs\"]\n    until: backlog/x\n";
    let specs: clm_model::Files = [
        ("specs/SPEC.md", doc("[\"Cargo.lock\"]", "")),
        ("specs/app/SPEC.md", doc("[\"crates/app/**\"]", "")),
        ("specs/feat/SPEC.md", doc("[\"crates/feat/**\"]", exempt)),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t))
    .collect();
    let (graph, violations) = clm_model::load_files(&specs);
    assert!(violations.is_empty(), "{violations:?}");
    let scan = scanned(&[
        ("crates/feat/tests/t.rs", &[]),
        ("crates/feat/tests/s.rs", &["zzz/x"]),
        ("crates/feat/tests/u.rs", &["zzz/x"]),
    ]);
    let s = select(&graph, &changed(&["crates/app/src/lib.rs"]), &scan, &[]);
    assert_eq!(nodes(&s), ["app"]);
    assert_eq!(
        files(&s),
        ["crates/app/src/lib.rs", "crates/feat/tests/t.rs"]
    );
    let none = select(&graph, &changed(&[]), &scan, &[]);
    assert_eq!(files(&none), Vec::<&str>::new());
}
