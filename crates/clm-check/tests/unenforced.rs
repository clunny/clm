//! `check/unenforced`, through `clm_check::run`.

use std::fs;
use std::path::Path;

use clm_check::run;
use clm_model::Violation;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";
const A: &str = "---\ntitle: A\nstatus: active\nowners: []\nserves: [architecture/a]\n---\n\n## Requirements\n\n### Requirement: x\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// An architecture entry `slug` with `extra` blocks after its scenario.
fn entry(slug: &str, extra: &str) -> String {
    format!("### Constraint: {slug}\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n{extra}")
}

fn unenforced(architecture: &str) -> Vec<Violation> {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "ARCHITECTURE.md", architecture);
    run(d)
        .into_iter()
        .filter(|v| v.rule == "check/unenforced")
        .collect()
}

#[test]
fn entry_with_neither_block_reported() {
    // req:check/unenforced@99f18cf4
    let text = format!(
        "# Architecture\n\n## Constraints\n\n{}{}{}{}",
        entry("d", ""),
        entry("a", "#### Enforced by\n- a/x\n\n"),
        entry("b", "#### Untested\nA judgement.\n\n"),
        entry("c", ""),
    );
    let found = unenforced(&text);
    let lines: Vec<_> = found
        .iter()
        .map(|v| format!("{}: {}", v.path.display(), v.message))
        .collect();
    assert_eq!(
        lines,
        [
            "ARCHITECTURE.md: architecture/d names no enforcing rule and no reason it is untested",
            "ARCHITECTURE.md: architecture/c names no enforcing rule and no reason it is untested",
        ]
    );
}

#[test]
fn no_architecture_entries_no_report() {
    // req:check/unenforced@99f18cf4
    assert_eq!(unenforced("# Architecture\n\nFree-form prose.\n"), []);
}
