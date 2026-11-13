//! `why_chain`: the chain by which a change selects a path, from the graph and scan alone.

use std::collections::BTreeMap;

use clm_affected::{Changed, why_chain};
use clm_model::Graph;

fn spec(paths: &str, depends_on: &str) -> Vec<u8> {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n---\n"
    )
    .into_bytes()
}

/// Components `a` on `b`, `b` on nothing, `c` on nothing, `x` on `y` and `z`, `y` on `w`, `z`
/// on `w`, `w` on nothing, each holding `<id>/**`; the root holds `Cargo.lock`.
fn graph() -> Graph {
    let files: clm_model::Files = [
        ("specs/SPEC.md", spec("[\"Cargo.lock\"]", "[]")),
        ("specs/a/SPEC.md", spec("[\"a/**\"]", "[b]")),
        ("specs/b/SPEC.md", spec("[\"b/**\"]", "[]")),
        ("specs/c/SPEC.md", spec("[\"c/**\"]", "[]")),
        ("specs/x/SPEC.md", spec("[\"x/**\"]", "[z, y]")),
        ("specs/y/SPEC.md", spec("[\"y/**\"]", "[w]")),
        ("specs/z/SPEC.md", spec("[\"z/**\"]", "[w]")),
        ("specs/w/SPEC.md", spec("[\"w/**\"]", "[]")),
        ("specs/m/SPEC.md", spec("[\"m/**\"]", "[n, w]")),
        ("specs/n/SPEC.md", spec("[\"n/**\"]", "[w]")),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t))
    .collect();
    let (graph, violations) = clm_model::load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn changed(paths: &[(&str, bool)]) -> Vec<Changed> {
    paths
        .iter()
        .map(|(p, exists)| Changed {
            path: p.to_string(),
            exists: *exists,
        })
        .collect()
}

fn map(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    entries
        .iter()
        .map(|(p, v)| (p.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

fn why(
    differing: &[(&str, bool)],
    cited: &[(&str, &[&str])],
    reads: &[(&str, &[&str])],
    path: &str,
) -> Vec<String> {
    why_chain(
        &graph(),
        &changed(differing),
        &map(cited),
        &map(reads),
        &["Cargo.lock".to_string()],
        path,
    )
}

#[test]
fn a_dependant_is_owned_through_its_edges() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("b/src/lib.rs", true)],
        &[("a/src/lib.rs", &[])],
        &[],
        "a/src/lib.rs",
    );
    assert_eq!(
        lines,
        [
            "a/src/lib.rs owned by a",
            "a depends on b",
            "b owns b/src/lib.rs (changed)"
        ]
    );
}

#[test]
fn a_changed_file_is_changed_alone() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(&[("b/src/lib.rs", true)], &[], &[], "b/src/lib.rs");
    assert_eq!(lines, ["b/src/lib.rs (changed)"]);
}

#[test]
fn a_path_the_selection_lacks_is_not_selected() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("b/src/lib.rs", true)],
        &[("c/src/lib.rs", &[])],
        &[],
        "c/src/lib.rs",
    );
    assert_eq!(lines, ["c/src/lib.rs not selected"]);
}

#[test]
fn a_deleted_changed_file_is_not_selected() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(&[("b/gone.rs", false)], &[], &[], "b/gone.rs");
    assert_eq!(lines, ["b/gone.rs not selected"]);
}

#[test]
fn an_all_selection_says_all() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("b/src/lib.rs", true), ("tools/build.sh", true)],
        &[],
        &[],
        "b/src/lib.rs",
    );
    assert_eq!(lines, ["all: tools/build.sh"]);
    let lines = why(&[("Cargo.lock", true)], &[], &[], "a/src/lib.rs");
    assert_eq!(lines, ["all: Cargo.lock"]);
}

#[test]
fn a_reader_reads_the_first_matching_path() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("ARCHITECTURE.md", true), ("docs/a.md", true)],
        &[],
        &[("c/tests/r.rs", &["docs/*.md", "ARCHITECTURE.md"])],
        "c/tests/r.rs",
    );
    assert_eq!(lines, ["c/tests/r.rs reads ARCHITECTURE.md"]);
}

#[test]
fn a_reader_beats_a_citation_and_ownership() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("b/src/lib.rs", true), ("ARCHITECTURE.md", true)],
        &[("a/tests/r.rs", &["b/x"])],
        &[("a/tests/r.rs", &["ARCHITECTURE.md"])],
        "a/tests/r.rs",
    );
    assert_eq!(lines, ["a/tests/r.rs reads ARCHITECTURE.md"]);
}

#[test]
fn a_citation_names_the_first_affected_component_by_id() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("b/src/lib.rs", true)],
        &[("c/tests/t.rs", &["c/none", "b/y", "a/x"])],
        &[],
        "c/tests/t.rs",
    );
    assert_eq!(
        lines,
        [
            "c/tests/t.rs cites a",
            "a depends on b",
            "b owns b/src/lib.rs (changed)"
        ]
    );
}

#[test]
fn a_citation_of_the_changed_component_owns_directly() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("b/src/a.rs", true), ("b/src/lib.rs", true)],
        &[("c/tests/t.rs", &["b/y"])],
        &[],
        "c/tests/t.rs",
    );
    assert_eq!(
        lines,
        ["c/tests/t.rs cites b", "b owns b/src/a.rs (changed)"]
    );
}

#[test]
fn the_shortest_path_wins_and_ties_go_to_the_first_id() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("w/lib.rs", true), ("y/lib.rs", true)],
        &[("x/src/lib.rs", &[])],
        &[],
        "x/src/lib.rs",
    );
    assert_eq!(
        lines,
        [
            "x/src/lib.rs owned by x",
            "x depends on y",
            "y owns y/lib.rs (changed)"
        ]
    );
    let lines = why(
        &[("w/lib.rs", true)],
        &[("x/src/lib.rs", &[])],
        &[],
        "x/src/lib.rs",
    );
    assert_eq!(
        lines,
        [
            "x/src/lib.rs owned by x",
            "x depends on y",
            "y depends on w",
            "w owns w/lib.rs (changed)"
        ]
    );
}

#[test]
fn a_doc_only_reader_reads_itself_rather_than_being_changed() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("docs/r.txt", true)],
        &[],
        &[("docs/r.txt", &["docs/**"])],
        "docs/r.txt",
    );
    assert_eq!(lines, ["docs/r.txt reads docs/r.txt"]);
}

#[test]
fn only_affected_components_can_be_cited() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("w/lib.rs", true)],
        &[("c/tests/t.rs", &["c/none", "y/q"])],
        &[],
        "c/tests/t.rs",
    );
    assert_eq!(
        lines,
        [
            "c/tests/t.rs cites y",
            "y depends on w",
            "w owns w/lib.rs (changed)"
        ]
    );
}

#[test]
fn a_citation_beats_ownership() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(
        &[("w/lib.rs", true)],
        &[("x/tests/t.rs", &["w/q"])],
        &[],
        "x/tests/t.rs",
    );
    assert_eq!(lines, ["x/tests/t.rs cites w", "w owns w/lib.rs (changed)"]);
}

#[test]
fn a_shorter_route_beats_an_earlier_id() {
    // req:check.affected/why-chain@c7f542ba
    let lines = why(&[("w/lib.rs", true)], &[("m/lib.rs", &[])], &[], "m/lib.rs");
    assert_eq!(
        lines,
        [
            "m/lib.rs owned by m",
            "m depends on w",
            "w owns w/lib.rs (changed)"
        ]
    );
}
