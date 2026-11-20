//! A write that fails part way through `clm apply` leaves every file as it was
//! (`failed-write`).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use clm_apply::{Journal, apply, apply_into};
use clm_model::load;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";

fn node(title: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n## Contract\nC.\n\n## Design\nD.\n\n## Requirements\n\n### Requirement: r\nThe r SHALL hold.\n\n#### Scenario: s\n- ok\n"
    )
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read_only(path: &Path, on: bool) {
    let mode = if on { 0o444 } else { 0o644 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Every file under `root`, with its bytes, sorted.
fn snapshot(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let name = path.strip_prefix(root).unwrap().display().to_string();
                out.push((name, fs::read_to_string(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

/// The hash `clm hashes` prints for `element` of component `id`.
fn hash(root: &Path, id: &str, element: &str) -> String {
    let (graph, _) = load(root);
    clm_apply::hashes(&graph.nodes[id])
        .into_iter()
        .find(|(e, _)| e == element)
        .unwrap()
        .1
}

#[test]
fn a_later_write_fails() {
    // req:apply/failed-write@1e66cb47
    let dir = two_designs();
    let d = dir.path();
    let before = snapshot(d);
    read_only(&d.join("specs/b/SPEC.md"), true);
    let refused = apply(d, "c").unwrap_err();
    read_only(&d.join("specs/b/SPEC.md"), false);
    let lines: Vec<String> = refused.iter().map(|v| v.to_string()).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("specs/b/SPEC.md: apply/atomic: ")
            && lines[0].contains("write failed"),
        "{lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("not restored")),
        "{lines:?}"
    );
    assert_eq!(snapshot(d), before);
}

#[test]
fn apply_into_leaves_the_undo_to_its_caller() {
    // req:apply/journal@e3e1076a
    let dir = two_designs();
    let d = dir.path();
    let before = snapshot(d);
    read_only(&d.join("specs/b/SPEC.md"), true);
    let mut journal = Journal::new(d);
    let refused = apply_into(d, "c", &mut journal).unwrap_err();
    read_only(&d.join("specs/b/SPEC.md"), false);
    assert!(refused[0].message.contains("write failed"), "{refused:?}");
    // `a`'s file was written through the journal and is still written.
    assert!(
        fs::read_to_string(d.join("specs/a/SPEC.md"))
            .unwrap()
            .contains("New."),
    );
    assert!(journal.undo().is_empty());
    assert_eq!(snapshot(d), before);

    // And when nothing fails, it writes every file `apply` would.
    let mut journal = Journal::new(d);
    let ops = apply_into(d, "c", &mut journal).unwrap();
    let paths: Vec<_> = ops
        .iter()
        .map(|op| op.path().display().to_string())
        .collect();
    assert_eq!(
        paths,
        ["specs/a/SPEC.md", "specs/b/SPEC.md", "changes/c/change.md"]
    );
    assert!(
        fs::read_to_string(d.join("changes/c/change.md"))
            .unwrap()
            .contains("status: applied")
    );
}

/// Components `a` and `b`, and a proposed change `c` with a valid delta modifying each one's
/// Design, `a`'s written first.
fn two_designs() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("A"));
    write(d, "specs/b/SPEC.md", &node("B"));
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n\n# c\n",
    );
    for id in ["a", "b"] {
        let base = hash(d, id, "design");
        write(
            d,
            &format!("changes/c/deltas/{id}.md"),
            &format!(
                "---\ncomponent: {id}\nbase: {{ design: \"{base}\" }}\n---\n## MODIFIED Design\nNew.\n"
            ),
        );
    }
    let (graph, _) = load(d);
    assert_eq!(clm_apply::validate(&graph, &graph.changes["c"]), []);
    dir
}

#[test]
fn a_removed_file_comes_back() {
    // req:apply/failed-write@1e66cb47
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("A"));
    write(d, "specs/a/assets/old.png", "png bytes");
    write(d, "specs/a/b/SPEC.md", &node("B"));
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n\n# c\n",
    );
    let old = hash(d, "a", "asset:old.png");
    write(
        d,
        "changes/c/deltas/a.md",
        &format!("---\ncomponent: a\n---\n## REMOVED Assets\n- `old.png` <!-- base: {old} -->\n"),
    );
    let bases: Vec<String> = ["frontmatter", "contract", "design"]
        .iter()
        .map(|e| format!("{e}: \"{}\"", hash(d, "a.b", e)))
        .collect();
    let r = hash(d, "a.b", "requirement:r");
    write(
        d,
        "changes/c/deltas/a.b.md",
        &format!(
            "---\ncomponent: a.b\nremove: true\nbase: {{ {} }}\n---\n## REMOVED Requirements\n### Requirement: r <!-- base: {r} -->\n",
            bases.join(", ")
        ),
    );
    let (graph, _) = load(d);
    assert_eq!(clm_apply::validate(&graph, &graph.changes["c"]), []);
    let before = snapshot(d);
    // The change's `change.md`, written last, can't be written.
    read_only(&d.join("changes/c/change.md"), true);
    let refused = apply(d, "c").unwrap_err();
    read_only(&d.join("changes/c/change.md"), false);
    let lines: Vec<String> = refused.iter().map(|v| v.to_string()).collect();
    assert!(
        lines[0].starts_with("changes/c/change.md: apply/atomic: ")
            && lines[0].contains("write failed"),
        "{lines:?}"
    );
    assert_eq!(
        fs::read(d.join("specs/a/assets/old.png")).unwrap(),
        b"png bytes"
    );
    assert!(d.join("specs/a/b/SPEC.md").is_file());
    assert_eq!(snapshot(d), before);
}
