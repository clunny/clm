use std::fs;
use std::path::Path;

use clm_apply::archive;
use tempfile::TempDir;

fn repo(status: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    for (rel, text) in [
        (
            "specs/SPEC.md",
            "---\ntitle: R\nstatus: active\nowners: []\n---\n".to_string(),
        ),
        (
            "changes/x/change.md",
            format!("---\nstatus: {status}\n---\n"),
        ),
        (
            "changes/x/deltas/clm.md",
            "---\ncomponent: clm\n---\n".to_string(),
        ),
    ] {
        let path = d.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    dir
}

#[test]
fn moves_done_change_into_archive() {
    // req:apply/archive-command@08442b52
    let dir = repo("done");
    let d = dir.path();
    let to = archive(d, "x").unwrap();
    assert_eq!(to, Path::new("changes/archive/x"));
    assert!(!d.join("changes/x").exists());
    assert!(d.join("changes/archive/x/change.md").is_file());
    assert!(d.join("changes/archive/x/deltas/clm.md").is_file());
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.changes["x"].archived);
}

#[test]
fn refuses_unfinished_change_or_existing_target() {
    // req:apply/archive-command@08442b52
    let dir = repo("applied");
    let d = dir.path();
    let v = archive(d, "x").unwrap_err();
    assert_eq!(v[0].rule, "apply/archive-command");
    assert!(d.join("changes/x/change.md").is_file());

    let dir = repo("abandoned");
    let d = dir.path();
    fs::create_dir_all(d.join("changes/archive/x")).unwrap();
    let v = archive(d, "x").unwrap_err();
    assert_eq!(v[0].rule, "apply/archive-command");
    assert!(v[0].message.contains("changes/archive/x"), "{v:?}");
    assert!(d.join("changes/x/change.md").is_file());
}

#[test]
fn the_archive_holds_the_same_change() {
    // req:apply/archive-command@08442b52
    let dir = repo("done");
    let d = dir.path();
    fs::create_dir_all(d.join("changes/archive/x")).unwrap();
    fs::write(
        d.join("changes/archive/x/change.md"),
        "---\nstatus: done\n---\n",
    )
    .unwrap();
    let lines: Vec<String> = archive(d, "x")
        .unwrap_err()
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("apply/archive-command") && l.contains("changes/archive/x")),
        "{lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("no unarchived change")),
        "{lines:?}"
    );
    assert!(d.join("changes/x/change.md").is_file());
}

#[test]
fn no_such_change() {
    // req:apply/archive-command@08442b52
    let dir = repo("done");
    let lines: Vec<String> = archive(dir.path(), "nope")
        .unwrap_err()
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("apply/archive-command")
                && l.contains("no unarchived change `nope`")),
        "{lines:?}"
    );
}
