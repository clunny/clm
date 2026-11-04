use std::fs;
use std::path::Path;

use clm_check::run;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    dir
}

fn change(root: &Path, id: &str, archived: bool, frontmatter: &str) {
    let dir = if archived {
        format!("changes/archive/{id}")
    } else {
        format!("changes/{id}")
    };
    write(
        root,
        &format!("{dir}/change.md"),
        &format!("---\n{frontmatter}\n---\n"),
    );
}

fn item(root: &Path, slug: &str, priority: Option<&str>) {
    let p = priority.map_or(String::new(), |p| format!("priority: {p}\n"));
    write(
        root,
        &format!("backlog/{slug}.md"),
        &format!("---\ntitle: T\nkind: idea\n{p}---\n"),
    );
}

fn caps(root: &Path) -> Vec<String> {
    run(root)
        .into_iter()
        .filter(|v| v.rule == "check/priority-caps")
        .map(|v| format!("{}: {}", v.path.display(), v.message))
        .collect()
}

#[test]
fn scenario_within_the_caps() {
    // req:check/priority-caps@7f8bcea1
    let dir = repo();
    let d = dir.path();
    change(d, "x", false, "status: applied\npriority: P1");
    item(d, "a", Some("P2"));
    item(d, "b", Some("P2"));
    item(d, "c", None);
    item(d, "e", None);
    item(d, "f", None);
    item(d, "g", None);
    item(d, "h", None);
    assert_eq!(caps(d), Vec::<String>::new());
}

#[test]
fn scenario_two_p1s() {
    // req:check/priority-caps@7f8bcea1
    let dir = repo();
    let d = dir.path();
    change(d, "x", false, "status: applied\npriority: P1");
    item(d, "a", Some("P1"));
    let found = caps(d);
    assert_eq!(found.len(), 1, "{found:?}");
    let msg = &found[0];
    assert!(msg.starts_with("backlog/a.md: "), "{msg}");
    assert!(msg.contains("P1") && msg.contains("cap 1"), "{msg}");
    assert!(
        msg.contains("backlog/a") && msg.contains("changes/x"),
        "{msg}"
    );
    assert!(
        msg.find("backlog/a").unwrap() < msg.find("changes/x").unwrap(),
        "holders sorted: {msg}"
    );
}

#[test]
fn each_overfull_priority_is_one_violation() {
    // req:check/priority-caps@7f8bcea1
    let dir = repo();
    let d = dir.path();
    for slug in ["a", "b", "c"] {
        item(d, slug, Some("P2"));
    }
    for slug in ["p", "q", "r", "s", "t"] {
        item(d, slug, Some("P4"));
    }
    for slug in ["u", "v", "w", "y", "z", "zz"] {
        item(d, slug, Some("P5"));
    }
    let found = caps(d);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].contains("P2") && found[0].contains("cap 2"),
        "{found:?}"
    );
    assert!(
        found[1].contains("P4") && found[1].contains("cap 4"),
        "{found:?}"
    );
    assert!(
        found[1].contains("backlog/p, backlog/q, backlog/r, backlog/s, backlog/t"),
        "{found:?}"
    );
}

#[test]
fn scenario_resolved_item_not_counted_twice() {
    // req:check/priority-caps@7f8bcea1
    let dir = repo();
    let d = dir.path();
    change(
        d,
        "x",
        false,
        "status: applied\npriority: P1\nresolves: [a]",
    );
    item(d, "a", Some("P1"));
    assert_eq!(caps(d), Vec::<String>::new());
}

#[test]
fn scenario_archived_and_abandoned_changes() {
    // req:check/priority-caps@7f8bcea1
    let dir = repo();
    let d = dir.path();
    change(d, "x", true, "status: done\npriority: P1");
    // Not yet archived (which `archive-done` reports separately): still not a holder.
    change(d, "y", false, "status: abandoned\npriority: P1");
    item(d, "a", Some("P1"));
    assert_eq!(caps(d), Vec::<String>::new());
}

#[test]
fn p3_holds_three() {
    // req:check/priority-caps@7f8bcea1
    let dir = repo();
    let d = dir.path();
    for slug in ["a", "b", "c"] {
        item(d, slug, Some("P3"));
    }
    assert_eq!(caps(d), Vec::<String>::new());
    item(d, "e", Some("P3"));
    let found = caps(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("P3") && found[0].contains("cap 3"),
        "{found:?}"
    );
}
