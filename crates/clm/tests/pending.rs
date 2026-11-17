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
    write(
        d,
        "crates/v/tests/t.rs",
        &in_test(&format!("\n\n\n// {REQ}g/x\n")),
    );
    write(d, "old/t.rs", &in_test(&format!("// {REQ}g/x\n")));
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
