//! `run_listed` reads the listing it is given where `run` reads the index (`run-listed`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn node(paths: &str, outside: Option<&str>) -> String {
    let outside = outside
        .map(|o| format!("outside: {o}\n"))
        .unwrap_or_default();
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n{outside}---\n## Contract\n"
    )
}

fn listing(files: &[&str]) -> Vec<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

fn lines(violations: &[clm_check::Violation]) -> Vec<String> {
    violations.iter().map(clm_check::format).collect()
}

#[test]
fn hidden_directory_listed() {
    // req:check/run-listed@162a7860
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "specs/SPEC.md",
        &node("[]", Some("[\".azure-pipelines/**\"]")),
    );
    write(root, "specs/a/SPEC.md", &node("[\"crates/a/**\"]", None));
    write(root, "crates/a/lib.rs", "x\n");
    write(root, ".azure-pipelines/ci.yml", "x\n");
    let files = listing(&[
        "specs/SPEC.md",
        "specs/a/SPEC.md",
        "crates/a/lib.rs",
        ".azure-pipelines/ci.yml",
    ]);
    let found = lines(&clm_check::run_listed(root, &files));
    let unowned: Vec<_> = found
        .iter()
        .filter(|l| l.contains("check/unowned-files"))
        .collect();
    assert!(unowned.is_empty(), "{found:?}");
}

#[test]
fn shared_path_in_a_listing() {
    // req:check/run-listed@162a7860
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", None));
    write(root, "specs/a/SPEC.md", &node("[\"src/**\"]", None));
    write(root, "specs/b/SPEC.md", &node("[\"src/b/**\"]", None));
    write(root, "src/b/x.rs", "x\n");
    let found = lines(&clm_check::run_listed(root, &listing(&["src/b/x.rs"])));
    assert_eq!(
        found
            .into_iter()
            .filter(|l| l.contains("check/paths-disjoint"))
            .collect::<Vec<_>>(),
        ["src/b/x.rs: check/paths-disjoint: owned by `a`, `b`"]
    );
}

#[test]
fn a_listed_file_the_directory_lacks() {
    // req:check/run-listed@162a7860
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", Some("[]")));
    let found = lines(&clm_check::run_listed(root, &listing(&["gone/lib.rs"])));
    assert_eq!(
        found,
        [
            "gone/lib.rs: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it"
        ]
    );
}

#[test]
fn the_listing_wins_inside_a_work_tree() {
    // req:check/run-listed@162a7860
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", Some("[]")));
    write(root, "tools/y.sh", "x\n");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    let found = lines(&clm_check::run_listed(root, &listing(&["other/z.rs"])));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("other/z.rs: check/unowned-files"));
}

#[test]
fn run_is_run_listed_on_the_index() {
    // req:check/run-listed@162a7860
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", Some("[\"gone/**\"]")));
    write(root, "specs/a/SPEC.md", &node("[\"src/**\"]", None));
    write(root, "specs/b/SPEC.md", &node("[\"src/b/**\"]", None));
    write(root, "src/b/x.rs", "x\n");
    write(root, "tools/y.sh", "x\n");
    write(root, "untracked/z.rs", "x\n");
    for args in [&["init", "-q"][..], &["add", "specs", "src", "tools"]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }
    let files = clm_git::tracked_files(root).unwrap();
    let run = lines(&clm_check::run(root));
    assert_eq!(run, lines(&clm_check::run_listed(root, &files)));
    assert!(run.iter().any(|l| l.contains("check/paths-disjoint")));
    assert!(
        run.iter()
            .any(|l| l.starts_with("tools/y.sh: check/unowned-files"))
    );
    assert!(!run.iter().any(|l| l.contains("untracked/z.rs")));
}
