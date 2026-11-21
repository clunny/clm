//! `select`: the nodes and files a set of changed paths affects, from the graph alone.

use std::collections::BTreeMap;

use clm_affected::{Changed, Selection};
use clm_model::Graph;

/// `clm_affected::select` with no file the scan couldn't read.
fn select(
    graph: &Graph,
    changed: &[Changed],
    files: &BTreeMap<String, Vec<String>>,
    all: &[String],
) -> Selection {
    clm_affected::select(graph, changed, files, &[], all)
}

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
fn a_dependant_alone_affects_no_dependency() {
    // req:check.affected/affected-components@ce7d5c08
    let s = select(
        &graph(),
        &changed(&["crates/b/src/lib.rs"]),
        &scanned(&[]),
        &[],
    );
    assert_eq!(nodes(&s), ["b", "c"]);
}

#[test]
fn cited_owned_and_changed_files() {
    // req:check.affected/affected-files@72f3baff
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
    // req:check.affected/affected-files@72f3baff
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
    // req:check.affected/affected-files@72f3baff
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
    // req:check.affected/affected-all@a55dd83e
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
    // req:check.affected/affected-all@a55dd83e
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
fn doc_only_paths_beside_code_select_what_the_code_does() {
    // req:check.affected/doc-only-selects-nothing@bb19731a
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
fn a_file_an_exemption_names_is_selected_whenever_a_component_is_affected() {
    // req:check.affected/affected-files@72f3baff
    let doc = |paths: &str, extra: &str| {
        format!("---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: []\n{extra}---\n")
            .into_bytes()
    };
    let exempt = "exemptions:\n  - rule: undeclared-imports\n    files: [\"crates/feat/tests/t.rs\", \"crates/feat/tests/m.rs\"]\n    until: backlog/x\n  - rule: skipped-tests\n    files: [\"crates/feat/tests/s.rs\"]\n    until: backlog/x\n";
    let specs: clm_model::Files = [
        ("specs/SPEC.md", doc("[\"Cargo.lock\"]", "")),
        (
            "specs/app/SPEC.md",
            // A glob over a file another component owns exempts nothing there.
            doc(
                "[\"crates/app/**\"]",
                "exemptions:\n  - rule: undeclared-imports\n    files: [\"crates/feat/tests/n.rs\"]\n    until: backlog/x\n",
            ),
        ),
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
        ("crates/feat/tests/m.rs", &["zzz/x"]),
        ("crates/feat/tests/n.rs", &["zzz/x"]),
    ]);
    let s = select(&graph, &changed(&["crates/app/src/lib.rs"]), &scan, &[]);
    assert_eq!(nodes(&s), ["app"]);
    assert_eq!(
        files(&s),
        [
            "crates/app/src/lib.rs",
            "crates/feat/tests/m.rs",
            "crates/feat/tests/t.rs"
        ]
    );
    let none = select(&graph, &changed(&[]), &scan, &[]);
    assert_eq!(files(&none), Vec::<&str>::new());
}

#[test]
fn a_file_that_isnt_utf8_is_selected_by_its_exemption_alone() {
    // req:check.affected/affected-files@72f3baff
    let doc = |paths: &str, depends_on: &str, extra: &str| {
        format!("---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n{extra}---\n")
            .into_bytes()
    };
    let exempt = "exemptions:\n  - rule: undeclared-imports\n    files: [\"crates/feat/tests/t.py\"]\n    until: backlog/x\n";
    let specs: clm_model::Files = [
        ("specs/SPEC.md", doc("[\"Cargo.lock\"]", "[]", "")),
        ("specs/app/SPEC.md", doc("[\"crates/app/**\"]", "[]", "")),
        (
            "specs/feat/SPEC.md",
            doc("[\"crates/feat/**\"]", "[app]", exempt),
        ),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t))
    .collect();
    let (graph, violations) = clm_model::load_files(&specs);
    assert!(violations.is_empty(), "{violations:?}");
    let unread = [
        "crates/feat/tests/t.py".to_string(),
        // Its owner is affected, but no exemption names it: owner and citation need the text.
        "crates/feat/tests/u.py".to_string(),
    ];
    let s = clm_affected::select(
        &graph,
        &changed(&["crates/app/src/lib.rs"]),
        &BTreeMap::new(),
        &unread,
        &[],
    );
    assert_eq!(nodes(&s), ["app", "feat"]);
    assert_eq!(
        files(&s),
        ["crates/app/src/lib.rs", "crates/feat/tests/t.py"]
    );
    let none = clm_affected::select(&graph, &changed(&[]), &BTreeMap::new(), &unread, &[]);
    assert_eq!(files(&none), Vec::<&str>::new());
}
