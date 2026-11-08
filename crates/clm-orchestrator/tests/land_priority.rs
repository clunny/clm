//! `land_priority` against local commits of a fixture repository.

mod common;

use clm_model::Priority;
use clm_orchestrator::{Error, land_priority};
use common::*;
use std::path::Path;

/// Commits `files` (path, text; `None` deletes) on a branch off `origin/main` and returns
/// `(base, head)`.
fn branch(work: &Path, name: &str, files: &[(&str, Option<&str>)]) -> (String, String) {
    let base = git(work, &["rev-parse", "origin/main"]).trim().to_string();
    git(work, &["switch", "-q", "-C", name, &base]);
    for (path, text) in files {
        match text {
            Some(t) => write(work, path, t),
            None => std::fs::remove_file(work.join(path)).unwrap(),
        }
    }
    git(work, &["add", "-A"]);
    git(work, &["commit", "-qm", name]);
    let head = git(work, &["rev-parse", "HEAD"]).trim().to_string();
    (base, head)
}

fn x_priority(p: &str) -> String {
    change_md("applied", &format!("priority: {p}\n"))
}

#[test]
fn scenario_a_builds_land() {
    // req:orchestrator/land-priority@5393b0de
    let r = repo();
    write(&r.work, "changes/x/change.md", &x_priority("P2"));
    push_main(&r.work, "x is P2");
    let (base, head) = branch(&r.work, "impl/a", &[("src/t.rs", Some("// t\n"))]);
    assert_eq!(
        land_priority(&r.work, "impl/a", &base, &head).unwrap(),
        Priority::P2
    );
}

#[test]
fn scenario_a_spec_branch() {
    // req:orchestrator/land-priority@5393b0de
    let r = repo();
    let (base, head) = branch(
        &r.work,
        "spec/x2",
        &[(
            "changes/x2/change.md",
            Some(&change_md("proposed", "priority: P1\n")),
        )],
    );
    assert_eq!(
        land_priority(&r.work, "spec/x2", &base, &head).unwrap(),
        Priority::P1
    );
}

#[test]
fn scenario_a_fix_that_resolves_an_item() {
    // req:orchestrator/land-priority@5393b0de
    let r = repo();
    cite(&r.work, "a.rs", &["a/p", "a/q"]);
    write(
        &r.work,
        "backlog/b.md",
        "---\ntitle: B\nkind: bug\ncomponents: [a]\npriority: P3\n---\n",
    );
    push_main(&r.work, "item");
    let (base, head) = branch(&r.work, "impl/a", &[("backlog/b.md", None)]);
    assert_eq!(
        land_priority(&r.work, "impl/a", &base, &head).unwrap(),
        Priority::P3
    );
}

#[test]
fn scenario_nothing_to_give_priority() {
    // req:orchestrator/land-priority@5393b0de
    let r = repo();
    let (base, head) = branch(&r.work, "fix-typo", &[("docs/guide/index.md", Some("x\n"))]);
    assert_eq!(
        land_priority(&r.work, "fix-typo", &base, &head).unwrap(),
        Priority::P5
    );
}

#[test]
fn an_edited_change_and_an_unknown_component_give_nothing() {
    // req:orchestrator/land-priority@5393b0de
    let r = repo();
    let (base, head) = branch(
        &r.work,
        "impl/nope",
        &[("changes/x/change.md", Some(&x_priority("P1")))],
    );
    assert_eq!(
        land_priority(&r.work, "impl/nope", &base, &head).unwrap(),
        Priority::P5
    );
}

#[test]
fn scenario_specs_that_dont_load() {
    // req:orchestrator/land-priority@5393b0de
    let r = repo();
    write(&r.work, "specs/SPEC.md", "not a spec\n");
    push_main(&r.work, "broken");
    let (base, head) = branch(&r.work, "impl/a", &[("src/t.rs", Some("// t\n"))]);
    match land_priority(&r.work, "impl/a", &base, &head) {
        Err(Error::Spec(v)) => assert!(!v.is_empty()),
        other => panic!("{other:?}"),
    }
}
