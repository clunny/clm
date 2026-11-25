//! The lines `clm pending` prints, as `pending` returns them for a fixture checkout's graph
//! and scanned markers.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// What `clm pending [<node>]` prints for the checkout at `root`, which must be valid: one
/// line per pending requirement of every component, or of `node`.
fn pending_text(root: &Path, node: Option<&str>) -> String {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    clm_coverage::pending(&graph, &clm_coverage::scan(root).req)
        .iter()
        .filter(|p| node.is_none_or(|n| p.node == n))
        .map(|p| format!("{p}\n"))
        .collect()
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

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
}

#[test]
fn scenario_blocked_by_a_need() {
    // req:check.coverage/pending-report@f3f35088
    let dir = needs_repo();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/y]\n---\n",
    );
    let out = pending_text(d, Some("a"));
    let y = out.lines().find(|l| l.starts_with("a/y ")).unwrap();
    assert!(y.ends_with(" unbuilt blocked backlog/q,b/z"), "{out}");
}

#[test]
fn scenario_nothing_pending() {
    // req:check.coverage/pending-report@f3f35088
    let dir = repo();
    let d = dir.path();
    let cite = |id: &str| format!("// {REQ}{id}@{}\n", version(d, id));
    write(
        d,
        "t.rs",
        &in_test(&(cite("a/x") + &cite("a/y") + &cite("a/z"))),
    );
    assert_eq!(pending_text(d, Some("a")), String::new());
    let out = pending_text(d, None);
    assert_eq!(out.as_str(), "");
}
