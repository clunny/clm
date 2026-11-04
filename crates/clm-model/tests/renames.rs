//! A change's recorded renames (`rename-records`).

use std::fs;
use std::path::Path;

use clm_model::{Rename, RenameKind, load};
use tempfile::TempDir;

fn repo(status_dir: &str, renames: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let write = |rel: &str, text: &str| {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    };
    write(
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n",
    );
    write(
        &format!("{status_dir}/x/change.md"),
        &format!("---\nstatus: proposed\n{renames}---\n"),
    );
    write("changes/y/change.md", "---\nstatus: proposed\n---\n");
    dir
}

fn problems(renames: &str) -> Vec<String> {
    let dir = repo("changes", renames);
    let (graph, violations) = load(dir.path());
    assert!(!graph.changes.contains_key("x"));
    violations
        .iter()
        .inspect(|v| {
            assert_eq!(v.rule, "model/rename-records");
            assert_eq!(v.path, Path::new("changes/x/change.md"));
        })
        .map(|v| v.message.clone())
        .collect()
}

#[test]
fn valid_entries_load_in_order() {
    // req:model/rename-records@25a8a149
    // req:model/parse-changes@e3b5847d
    let dir = repo(
        "changes",
        "renames:\n  - component: check.coverage\n    to: check.markers\n  - requirement: apply/stale-base\n    to: base-moved\n  - term: sprocket\n    to: cogs\n",
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let want = |kind, name: &str, to: &str| Rename {
        kind,
        name: name.into(),
        to: to.into(),
    };
    assert_eq!(
        graph.changes["x"].renames,
        [
            want(RenameKind::Component, "check.coverage", "check.markers"),
            want(RenameKind::Requirement, "apply/stale-base", "base-moved"),
            want(RenameKind::Term, "sprocket", "cogs"),
        ]
    );
    assert!(graph.changes["y"].renames.is_empty());
}

#[test]
fn a_move_is_not_a_rename() {
    // req:model/rename-records@25a8a149
    let p = problems("renames:\n  - component: check.coverage\n    to: model.coverage\n");
    assert_eq!(p.len(), 1);
    assert!(p[0].starts_with("entry 1:"), "{p:?}");
}

#[test]
fn two_keys_in_one_entry() {
    // req:model/rename-records@25a8a149
    let p = problems("renames:\n  - term: sprocket\n    requirement: a/x\n    to: y\n");
    assert_eq!(p.len(), 1);
    assert!(p[0].starts_with("entry 1:"), "{p:?}");
}

#[test]
fn bad_entries_are_reported_by_position() {
    // req:model/rename-records@25a8a149
    for (entry, position) in [
        ("{to: y}", 1),
        ("{term: sprocket}", 1),
        ("{term: sprocket, to: y, extra: z}", 1),
        ("{term: '', to: y}", 1),
        ("{term: sprocket, to: 3}", 1),
        ("{term: \"a\\nb\", to: y}", 1),
        ("{component: Bad, to: Worse}", 1),
        ("{requirement: a, to: y}", 1),
        ("{requirement: a/x, to: Y}", 1),
        ("{term: sprocket, to: sprocket}", 1),
        ("{requirement: a/x, to: x}", 1),
        ("{component: a.b, to: a.B}", 1),
        ("{component: a.b, to: a.b}", 1),
        ("{requirement: a/X, to: y}", 1),
        ("{term: \"a\\rb\", to: y}", 1),
    ] {
        let p = problems(&format!(
            "renames:\n  - {{term: ok, to: fine}}\n  - {entry}\n"
        ));
        assert_eq!(p.len(), 1, "{entry}: {p:?}");
        assert!(
            p[0].starts_with(&format!("entry {}:", position + 1)),
            "{entry}: {p:?}"
        );
    }
}

#[test]
fn duplicates_are_reported() {
    // req:model/rename-records@25a8a149
    let twice = problems("renames:\n  - term: a\n    to: b\n  - term: a\n    to: c\n");
    assert_eq!(twice.len(), 1);
    assert!(twice[0].starts_with("entry 2:"), "{twice:?}");
    let same_new = problems("renames:\n  - term: a\n    to: c\n  - term: b\n    to: c\n");
    assert_eq!(same_new.len(), 1);
    assert!(same_new[0].starts_with("entry 2:"), "{same_new:?}");
    let same_slug =
        problems("renames:\n  - requirement: n/a\n    to: c\n  - requirement: n/b\n    to: c\n");
    assert_eq!(same_slug.len(), 1);
    // The same slug under different nodes is two different new names.
    let dir = repo(
        "changes",
        "renames:\n  - requirement: n/a\n    to: c\n  - requirement: m/b\n    to: c\n",
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["x"].renames.len(), 2);
}

#[test]
fn an_archived_change_is_loaded_unchecked() {
    // req:model/rename-records@25a8a149
    let dir = repo(
        "changes/archive",
        "renames:\n  - component: check.coverage\n    to: model.coverage\n  - term: a\n    to: a\n  - nonsense: 1\n",
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    let x = &graph.changes["x"];
    assert!(x.archived);
    assert_eq!(x.renames.len(), 2);
}

#[test]
fn duplicates_are_reported_beside_other_problems() {
    // req:model/rename-records@25a8a149
    let p = problems(
        "renames:\n  - term: a\n    to: A\n  - term: b\n    to: c\n  - {to: y}\n  - term: b\n    to: d\n",
    );
    assert_eq!(p.len(), 2, "{p:?}");
    assert!(p[0].starts_with("entry 3:"), "{p:?}");
    assert!(p[1].starts_with("entry 4:"), "{p:?}");
}

#[test]
fn human_layer_requirements_can_be_renamed() {
    // req:model/rename-records@25a8a149
    let dir = repo(
        "changes",
        "renames:\n  - requirement: architecture/a-b\n    to: c\n  - requirement: capability/d\n    to: e\n",
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["x"].renames.len(), 2);
}
