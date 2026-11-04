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

fn change(root: &Path, dir: &str, frontmatter: &str) {
    write(
        root,
        &format!("{dir}/change.md"),
        &format!("---\n{frontmatter}\n---\n"),
    );
}

fn item(root: &Path, slug: &str) {
    write(
        root,
        &format!("backlog/{slug}.md"),
        "---\ntitle: T\nkind: idea\n---\n",
    );
}

/// `<path>: <message>` for each `check/backlog-entry-shape` violation.
fn shapes(root: &Path) -> Vec<String> {
    run(root)
        .into_iter()
        .filter(|v| v.rule == "check/backlog-entry-shape")
        .map(|v| format!("{}: {}", v.path.display(), v.message))
        .collect()
}

#[test]
fn scenario_resolves_written_as_a_claim_path() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(d, "changes/c", "status: proposed\nresolves: [backlog/x, y]");
    item(d, "x");
    item(d, "y");
    assert_eq!(
        shapes(d),
        vec![
            "changes/c/change.md: change `c` resolves `backlog/x`: `resolves` entries are bare slugs; write `x`"
        ]
    );
}

#[test]
fn scenario_resolves_written_as_a_file_name() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(d, "changes/c", "status: applied\nresolves: [x.md]");
    assert_eq!(
        shapes(d),
        vec![
            "changes/c/change.md: change `c` resolves `x.md`: `resolves` entries are bare slugs; write `x`"
        ]
    );
}

#[test]
fn scenario_from_written_as_a_bare_slug() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(d, "changes/c", "status: applied\nfrom: x");
    assert_eq!(
        shapes(d),
        vec![
            "changes/c/change.md: change `c` has `from: x`: `from` is `backlog/<slug>`; write `backlog/x`"
        ]
    );
}

#[test]
fn scenario_nested_path() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(d, "changes/c", "status: applied\nresolves: [backlog/sub/x]");
    assert_eq!(
        shapes(d),
        vec![
            "changes/c/change.md: change `c` resolves `backlog/sub/x`: `resolves` entries are bare slugs; name a top-level backlog item"
        ]
    );
}

#[test]
fn scenario_well_formed_entries_naming_deleted_items() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(
        d,
        "changes/c",
        "status: applied\nfrom: backlog/a\nresolves: [b]",
    );
    assert_eq!(shapes(d), Vec::<String>::new());
}

#[test]
fn scenario_archived_change() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(
        d,
        "changes/archive/c",
        "status: done\nresolves: [backlog/x]",
    );
    // Archived in a live status is `check/archive-done`'s; this rule still skips it.
    change(
        d,
        "changes/archive/e",
        "status: applied\nfrom: x\nresolves: [backlog/y]",
    );
    assert_eq!(shapes(d), Vec::<String>::new());
}

#[test]
fn abandoned_changes_and_odd_entries() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    // Abandoned outside the archive is also `check/archive-done`'s, but still tested here.
    change(
        d,
        "changes/c",
        "status: abandoned\nfrom: backlog/x.md\nresolves: [\"\", backlog/y.md, backlog/backlog/z, x.md.md]",
    );
    assert_eq!(
        shapes(d),
        vec![
            "changes/c/change.md: change `c` resolves ``: `resolves` entries are bare slugs; name a top-level backlog item",
            "changes/c/change.md: change `c` resolves `backlog/y.md`: `resolves` entries are bare slugs; write `y`",
            "changes/c/change.md: change `c` resolves `backlog/backlog/z`: `resolves` entries are bare slugs; name a top-level backlog item",
            "changes/c/change.md: change `c` resolves `x.md.md`: `resolves` entries are bare slugs; name a top-level backlog item",
            "changes/c/change.md: change `c` has `from: backlog/x.md`: `from` is `backlog/<slug>`; write `backlog/x`",
        ]
    );
}

#[test]
fn from_nested_suggests_nothing() {
    // req:check/backlog-entry-shape@c484c1ef
    let dir = repo();
    let d = dir.path();
    change(d, "changes/c", "status: applied\nfrom: backlog/sub/x");
    assert_eq!(
        shapes(d),
        vec![
            "changes/c/change.md: change `c` has `from: backlog/sub/x`: `from` is `backlog/<slug>`; name a top-level backlog item"
        ]
    );
}
