//! The `exemptions` frontmatter key.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_model::{ExemptRule, Violation, check_graph, check_node_frontmatter, load};
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: unbuilt\nowners: [\"@a\"]";

/// A repo with a root and a component `a` whose frontmatter is `FM` plus `a_extra`.
fn repo(a_extra: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, extra) in [("specs/SPEC.md", ""), ("specs/a/SPEC.md", a_extra)] {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("---\n{FM}{extra}\n---\n")).unwrap();
    }
    dir
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

#[test]
fn an_exemption_parsed() {
    // req:model/exemptions@20ac503e
    let dir = repo(
        "\nexemptions: [{rule: undeclared-imports, files: [\"tests/test_api.py\"], until: backlog/x, why: \"the suite builds the whole app\"}, {rule: skipped-tests, files: [\"a/**\", \"b.py\"], until: backlog/y}]",
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let e = &graph.nodes["a"].frontmatter.exemptions;
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].rule, ExemptRule::UndeclaredImports);
    assert_eq!(e[0].files, ["tests/test_api.py"]);
    assert_eq!(e[0].until, "backlog/x");
    assert_eq!(e[0].why.as_deref(), Some("the suite builds the whole app"));
    assert_eq!(e[1].rule, ExemptRule::SkippedTests);
    assert_eq!(e[1].files, ["a/**", "b.py"]);
    assert_eq!(e[1].why, None);
}

#[test]
fn the_third_rule() {
    // req:model/exemptions@20ac503e
    let dir = repo("\nexemptions: [{rule: tests-cite-their-owner, files: [x], until: backlog/x}]");
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let rule = graph.nodes["a"].frontmatter.exemptions[0].rule;
    assert_eq!(rule, ExemptRule::TestsCiteTheirOwner);
}

#[test]
fn absent_means_none() {
    // req:model/exemptions@20ac503e
    let (graph, violations) = load(repo("").path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.nodes["a"].frontmatter.exemptions.is_empty());
}

#[test]
fn a_rule_that_cant_be_exempted() {
    // req:model/exemptions@20ac503e
    let dir = repo("\nexemptions: [{rule: paths-disjoint, files: [x], until: backlog/x}]");
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/exemptions"]);
    assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
    let m = &violations[0].message;
    assert!(
        m.contains("exemptions") && m.contains("paths-disjoint"),
        "{m}"
    );
}

#[test]
fn bad_entries_name_their_position_and_the_problem() {
    // req:model/exemptions@20ac503e
    let ok = "{rule: skipped-tests, files: [x], until: backlog/x}";
    for (entry, word) in [
        ("{rule: skipped-tests, files: [x]}", "until"),
        ("{rule: skipped-tests, until: backlog/x}", "files"),
        ("{files: [x], until: backlog/x}", "rule"),
        (
            "{rule: skipped-tests, files: [x], until: backlog/x, extra: 1}",
            "extra",
        ),
        (
            "{rule: skipped-tests, files: [], until: backlog/x}",
            "files",
        ),
        (
            "{rule: skipped-tests, files: [\"\"], until: backlog/x}",
            "files",
        ),
        ("{rule: skipped-tests, files: x, until: backlog/x}", "files"),
        (
            "{rule: skipped-tests, files: [1], until: backlog/x}",
            "files",
        ),
        ("{rule: skipped-tests, files: [x], until: x}", "until"),
        (
            "{rule: skipped-tests, files: [x], until: backlog/}",
            "until",
        ),
        (
            "{rule: skipped-tests, files: [x], until: [backlog/x]}",
            "until",
        ),
        (
            "{rule: skipped-tests, files: [x], until: backlog/x, why: 3}",
            "why",
        ),
        ("not-a-map", "map"),
    ] {
        let extra = format!("\nexemptions: [{ok}, {entry}]");
        let (_, violations) = load(repo(&extra).path());
        assert_eq!(rules(&violations), ["model/exemptions"], "{entry}");
        assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
        let m = &violations[0].message;
        let named = m.contains("exemptions") && m.contains("entry 2") && m.contains(word);
        assert!(named, "{entry}: {m}");
        // The frontmatter check `apply` runs rejects the same.
        let fm = format!("{FM}{extra}");
        let path = Path::new("specs/a/SPEC.md");
        let errs = check_node_frontmatter("a", path, &fm, &BTreeSet::new()).unwrap_err();
        assert_eq!(rules(&errs), ["model/exemptions"], "{entry}");
    }
}

#[test]
fn not_a_list_is_refused() {
    // req:model/exemptions@20ac503e
    for extra in [
        "\nexemptions: x",
        "\nexemptions:",
        "\nexemptions: {rule: skipped-tests}",
    ] {
        let (_, violations) = load(repo(extra).path());
        assert_eq!(rules(&violations), ["model/exemptions"], "{extra}");
        assert!(violations[0].message.contains("exemptions"));
    }
}

#[test]
fn the_item_is_not_looked_up_when_loading() {
    // req:model/exemptions@20ac503e
    let dir =
        repo("\nexemptions: [{rule: skipped-tests, files: [x], until: backlog/no-such-item}]");
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(check_graph(&graph).is_empty());
}
