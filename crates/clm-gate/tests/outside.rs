use std::fs;
use std::path::Path;

use clm_gate::files_outside_spec_paths;
use clm_rename::rename;
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) -> String {
    clm_git::git(d, args).unwrap()
}

fn write(d: &Path, rel: &str, content: &str) {
    let path = d.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn commit(d: &Path, message: &str) {
    git(d, &["add", "-A"]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            message,
        ],
    );
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n\n## Design\nthe spine\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

fn base() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "crates/a/src/lib.rs", "pub fn a() {}\n");
    write(d, "crates/a/tests/t.rs", "// req:a/x@00000000\n");
    commit(d, "base");
    dir
}

// req:gate/files-outside-spec-paths@b74ab213
#[test]
fn a_code_file_among_spec_files() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "specs/a/SPEC.md", &format!("{A}\nmore\n"));
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    write(d, "crates/b/src/lib.rs", "pub fn b() {}\n");
    commit(d, "pr");
    // something else checked out, with an uncommitted edit
    git(d, &["checkout", "-q", "main"]);
    write(d, "crates/a/src/lib.rs", "pub fn changed() {}\n");
    let files = files_outside_spec_paths(d, "main", "pr").unwrap();
    assert_eq!(files, ["crates/b/src/lib.rs", "crates/x/src/lib.rs"]);
}

// req:gate/files-outside-spec-paths@b74ab213
#[test]
fn only_a_new_backlog_item() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "backlog/x.md", "---\nkind: idea\n---\nx\n");
    commit(d, "pr");
    assert!(
        files_outside_spec_paths(d, "main", "HEAD")
            .unwrap()
            .is_empty()
    );
}

// req:gate/files-outside-spec-paths@b74ab213
#[test]
fn a_rename_branch() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "-b", "pr"]);
    let change = "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nRename.\n";
    write(d, "changes/r/change.md", change);
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    commit(d, "rename");
    let tests = fs::read_to_string(d.join("crates/a/tests/t.rs")).unwrap();
    assert!(tests.contains("a/y"), "{tests}");
    assert!(
        files_outside_spec_paths(d, "main", "HEAD")
            .unwrap()
            .is_empty()
    );
}
