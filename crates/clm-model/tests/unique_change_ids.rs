//! A live change and an archived one that share an ID (`unique-change-ids`).

use std::path::{Path, PathBuf};

use clm_model::{ChangeStatus, Files, Violation, load_files};

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";

fn load(extra: &[(&str, &str)]) -> (clm_model::Graph, Vec<Violation>) {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
    for (path, text) in extra {
        files.insert(PathBuf::from(path), text.as_bytes().to_vec());
    }
    load_files(&files)
}

#[test]
fn live_and_archived_change_share_an_id() {
    // req:model/unique-change-ids@bfad0e08
    let (graph, violations) = load(&[
        ("changes/x/change.md", "---\nstatus: applied\n---\n"),
        ("changes/archive/x/change.md", "---\nstatus: done\n---\n"),
    ]);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/unique-change-ids");
    assert_eq!(violations[0].path, Path::new("changes/x/change.md"));
    assert!(
        violations[0]
            .message
            .contains("changes/archive/x/change.md"),
        "{violations:?}"
    );
    assert_eq!(graph.changes.len(), 1);
    let x = &graph.changes["x"];
    assert!(!x.archived);
    assert_eq!(x.frontmatter.status, ChangeStatus::Applied);
}

#[test]
fn a_change_not_loaded_makes_no_pair() {
    // req:model/unique-change-ids@bfad0e08
    // The live copy doesn't load, so the archived one is the change `x`.
    let (graph, violations) = load(&[
        ("changes/x/change.md", "---\nstatus: nope\n---\n"),
        ("changes/archive/x/change.md", "---\nstatus: done\n---\n"),
    ]);
    assert!(
        violations
            .iter()
            .all(|v| v.rule != "model/unique-change-ids"),
        "{violations:?}"
    );
    assert!(graph.changes["x"].archived);
    // The archived copy doesn't load: no pair, and the live change stays.
    let (graph, violations) = load(&[
        ("changes/x/change.md", "---\nstatus: applied\n---\n"),
        ("changes/archive/x/change.md", "---\nstatus: nope\n---\n"),
    ]);
    assert!(
        violations
            .iter()
            .all(|v| v.rule != "model/unique-change-ids"),
        "{violations:?}"
    );
    assert!(!graph.changes["x"].archived);
}

#[test]
fn the_archived_copys_own_violations_stay() {
    // req:model/unique-change-ids@bfad0e08
    // An archived delta with no frontmatter is reported, and the pair is too.
    let (graph, violations) = load(&[
        ("changes/x/change.md", "---\nstatus: applied\n---\n"),
        ("changes/archive/x/change.md", "---\nstatus: done\n---\n"),
        (
            "changes/archive/x/deltas/a.md",
            "---\n---\n\n## Design\nd\n",
        ),
    ]);
    let rules: Vec<&str> = violations.iter().map(|v| v.rule).collect();
    assert!(rules.contains(&"model/unique-change-ids"), "{violations:?}");
    assert!(
        violations
            .iter()
            .any(|v| v.path == Path::new("changes/archive/x/deltas/a.md")),
        "{violations:?}"
    );
    assert!(!graph.changes["x"].archived);
}
