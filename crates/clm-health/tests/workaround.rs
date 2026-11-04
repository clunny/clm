mod common;

use std::fs;

use clm_health::Failure;
use common::{repo, spec, write};

/// Node `a` with requirement `a/x`, and `a/x`'s current version.
fn node_a() -> (tempfile::TempDir, String) {
    let dir = repo();
    write(dir.path(), "specs/a/SPEC.md", &spec("[]", &["x"]));
    let (graph, _) = clm_model::load(dir.path());
    let version = graph.requirement("a/x").unwrap().version().to_string();
    (dir, version)
}

#[test]
fn scenario_first_flag() {
    // req:check.health/workaround-command@d9c5203d
    let (dir, version) = node_a();
    let d = dir.path();
    let body = "The scenario needs a fixed date, so the test fakes the clock.\n";
    let path = clm_health::workaround(d, "a/x", "Faked the clock", body).unwrap();
    assert_eq!(path, std::path::Path::new("backlog/workaround-a-x-1.md"));
    let text = fs::read_to_string(d.join(&path)).unwrap();
    for want in [
        "kind: workaround",
        "components: [a]",
        &format!("requirement: a/x@{version}"),
        "title: \"Faked the clock\"",
        body.trim(),
    ] {
        assert!(text.contains(want), "{want} not in {text}");
    }
    // The model loads it with no violation, and as a workaround flag on `a/x`.
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    let item = &graph.backlog["workaround-a-x-1"];
    assert_eq!(item.frontmatter.requirement.as_ref().unwrap().id, "a/x");
}

#[test]
fn scenario_second_flag() {
    // req:check.health/workaround-command@d9c5203d
    let (dir, _) = node_a();
    let d = dir.path();
    clm_health::workaround(d, "a/x", "Faked the clock", "one\n").unwrap();
    let first = fs::read_to_string(d.join("backlog/workaround-a-x-1.md")).unwrap();
    let path = clm_health::workaround(d, "a/x", "Workaround", "two\n").unwrap();
    assert_eq!(path, std::path::Path::new("backlog/workaround-a-x-2.md"));
    let after = fs::read_to_string(d.join("backlog/workaround-a-x-1.md")).unwrap();
    assert_eq!(first, after);
}

#[test]
fn a_title_with_quotes_reads_back() {
    // req:check.health/workaround-command@d9c5203d
    let (dir, _) = node_a();
    let d = dir.path();
    clm_health::workaround(d, "a/x", "Said \"no\": then: yes", "b\n").unwrap();
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        graph.backlog["workaround-a-x-1"].frontmatter.title,
        "Said \"no\": then: yes"
    );
}

#[test]
fn scenario_no_such_requirement() {
    // req:check.health/workaround-command@d9c5203d
    let (dir, _) = node_a();
    let d = dir.path();
    for id in ["a/ghost", "ghost", "b/x"] {
        match clm_health::workaround(d, id, "x", "b\n") {
            Err(Failure::Refused(why)) => assert!(why.contains(id), "{why}"),
            other => panic!("{id}: {other:?}"),
        }
    }
    assert!(!d.join("backlog").exists());
}
