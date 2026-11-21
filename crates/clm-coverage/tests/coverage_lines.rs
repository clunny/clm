//! The lines `clm coverage` prints, as `report` returns them for a fixture checkout's graph
//! and scanned markers.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// What `clm coverage` prints for the checkout at `root`, which must be valid and hold no
/// marker outside a test: one line per requirement that a marker cites or that is pending.
fn coverage_text(root: &Path) -> String {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    clm_coverage::report(&graph, &clm_coverage::scan(root).req)
        .iter()
        .map(|c| format!("{c}\n"))
        .collect()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
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

const NODE: &str = "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: r\nR.\n\n#### Scenario: s\n- ok\n";

fn change_repo(delta: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", NODE);
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n",
    );
    write(dir.path(), "changes/c/deltas/clm.md", delta);
    dir
}

#[test]
fn coverage_command_prints_report() {
    // req:check.coverage/coverage-report@72845fef
    let dir = change_repo("---\ncomponent: clm\n---\n");
    write(
        dir.path(),
        "t.rs",
        concat!("// req", ":clm/r\n#[test]\nfn t() {}\n"),
    );
    let out = coverage_text(dir.path());
    assert_eq!(out, "clm/r behind t.rs:1\n");
    let (graph, _) = clm_model::load(dir.path());
    let v = graph.requirement("clm/r").unwrap().version().to_string();
    write(
        dir.path(),
        "t.rs",
        &in_test(&format!("// {}clm/r@{v}\n", concat!("req", ":"))),
    );
    let out = coverage_text(dir.path());
    assert_eq!(out, "clm/r current t.rs:1\n");
}
