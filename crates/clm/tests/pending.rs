use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

fn clm_pending(root: &Path, node: Option<&str>) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_clm"));
    cmd.arg("pending");
    if let Some(node) = node {
        cmd.arg(node);
    }
    let out = cmd.arg("--root").arg(root).output().unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Node `a` with `a/x`, `a/y` and `a/z`, and an `applied` change that removed `a/gone`.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    let mut spec =
        String::from("---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n");
    for slug in ["x", "y", "z"] {
        spec.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    write(d, "specs/a/SPEC.md", &spec);
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/r/deltas/a.md",
        "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0123456789abcdef -->\n",
    );
    dir
}

fn version(root: &Path, id: &str) -> String {
    let (graph, _) = clm_model::load(root);
    graph.requirement(id).unwrap().version().to_string()
}

#[test]
fn scenario_states() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo();
    let d = dir.path();
    let (x, y, z) = (version(d, "a/x"), version(d, "a/y"), version(d, "a/z"));
    write(
        d,
        "t.rs",
        &format!("// {REQ}a/x@11111111\n// {REQ}a/z@{z}\n\n\n\n\n\n\n// {REQ}a/gone\n"),
    );
    let (code, out) = clm_pending(d, Some("a"));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        out,
        format!("a/gone obsolete t.rs:9\na/x {x} behind 11111111\na/y {y} unbuilt\n")
    );
}

#[test]
fn scenario_blocked() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/y]\n---\n",
    );
    let (_, out) = clm_pending(d, Some("a"));
    let y = out.lines().find(|l| l.starts_with("a/y ")).unwrap();
    assert!(y.ends_with(" unbuilt blocked backlog/q"), "{out}");
}

/// `repo()`, with `a/y` needing `b/z`, and node `b` holding `z`.
fn needs_repo() -> TempDir {
    let dir = repo();
    let d = dir.path();
    let a = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    write(
        d,
        "specs/a/SPEC.md",
        &a.replace("y.\n", "y.\n\n#### Needs\n- b/z\n"),
    );
    write(
        d,
        "specs/b/SPEC.md",
        "---\ntitle: B\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: z\nz.\n\n#### Scenario: s\n- ok\n",
    );
    dir
}

#[test]
fn scenario_blocked_by_a_need() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = needs_repo();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/y]\n---\n",
    );
    let (code, out) = clm_pending(d, Some("a"));
    assert_eq!(code, Some(0), "{out}");
    let y = out.lines().find(|l| l.starts_with("a/y ")).unwrap();
    assert!(y.ends_with(" unbuilt blocked backlog/q,b/z"), "{out}");
}

#[test]
fn scenario_need_satisfied() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = needs_repo();
    let d = dir.path();
    write(d, "t.rs", &format!("// {REQ}b/z@{}\n", version(d, "b/z")));
    let (_, out) = clm_pending(d, Some("a"));
    let y = out.lines().find(|l| l.starts_with("a/y ")).unwrap();
    assert_eq!(y, format!("a/y {} unbuilt", version(d, "a/y")));
}

#[test]
fn scenario_nothing_pending() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo();
    let d = dir.path();
    let cite = |id: &str| format!("// {REQ}{id}@{}\n", version(d, id));
    write(d, "t.rs", &(cite("a/x") + &cite("a/y") + &cite("a/z")));
    assert_eq!(clm_pending(d, Some("a")), (Some(0), String::new()));
    let (code, out) = clm_pending(d, None);
    assert_eq!((code, out.as_str()), (Some(0), ""));
}

#[test]
fn every_node_without_an_argument() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/b/SPEC.md",
        "---\ntitle: B\nstatus: specd\nowners: []\n---\n\n## Requirements\n\n### Requirement: q\nq.\n\n#### Scenario: s\n- ok\n",
    );
    let (_, out) = clm_pending(d, None);
    let ids: Vec<&str> = out.lines().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(ids, ["a/x", "a/y", "a/z", "b/q"]);
}

#[test]
fn violations_and_unknown_nodes_exit_1() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo();
    let d = dir.path();
    let (code, out) = clm_pending(d, Some("nope"));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("unknown component `nope`"), "{out}");
    write(d, "specs/b/SPEC.md", "---\ntitle: B\n---\n");
    let (code, out) = clm_pending(d, Some("a"));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("model/valid-frontmatter"), "{out}");
}

#[test]
fn scenario_removed_node_from_the_cli() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "changes/r/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    write(
        d,
        "specs/v/SPEC.md",
        "---\ntitle: V\nstatus: active\nowners: []\npaths: [\"crates/v/**\"]\n---\n",
    );
    write(d, "crates/v/tests/t.rs", &format!("\n\n\n// {REQ}g/x\n"));
    write(d, "old/t.rs", &format!("// {REQ}g/x\n"));
    assert_eq!(
        clm_pending(d, Some("v")),
        (Some(0), "g/x obsolete crates/v/tests/t.rs:4\n".to_string())
    );
    assert_eq!(
        clm_pending(d, Some("clm")),
        (Some(0), "g/x obsolete old/t.rs:1\n".to_string())
    );
    assert_eq!(clm_pending(d, Some("g")).0, Some(1));
}
