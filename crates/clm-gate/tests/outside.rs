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
    write(
        d,
        "crates/a/tests/t.rs",
        concat!("// req", ":a/x@00000000\n"),
    );
    commit(d, "base");
    dir
}

// req:gate/files-outside-spec-paths@9ebb011d
#[test]
fn a_code_file_among_spec_files() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "specs/a/SPEC.md", &format!("{A}\nmore\n"));
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    write(d, "crates/b/src/lib.rs", "pub fn b() {}\n");
    commit(d, "pr");
    // something else checked out: `main`, which moved on since, with an uncommitted edit
    git(d, &["checkout", "-q", "main"]);
    write(d, "crates/m/src/lib.rs", "pub fn m() {}\n");
    commit(d, "main moves");
    write(d, "crates/a/src/lib.rs", "pub fn changed() {}\n");
    let files = files_outside_spec_paths(d, "main", "pr").unwrap();
    assert_eq!(files, ["crates/b/src/lib.rs", "crates/x/src/lib.rs"]);
}

// req:gate/files-outside-spec-paths@9ebb011d
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

// req:gate/files-outside-spec-paths@9ebb011d
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

// req:gate/files-outside-spec-paths@9ebb011d
#[test]
fn a_rename_branch_with_an_extra_code_edit() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "-b", "pr"]);
    let change = "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nRename.\n";
    write(d, "changes/r/change.md", change);
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    write(d, "crates/a/src/lib.rs", "pub fn other() {}\n");
    commit(d, "rename and edit");
    let files = files_outside_spec_paths(d, "main", "HEAD").unwrap();
    assert!(
        files.contains(&"crates/a/src/lib.rs".to_string()),
        "{files:?}"
    );
}

// req:gate/files-outside-spec-paths@9ebb011d
#[test]
fn an_approval_change() {
    let dir = base();
    let d = dir.path();
    write(d, "clm.toml", "approval = \"other\"\n");
    commit(d, "policy");
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "clm.toml", "approval = \"any\"\n");
    write(d, "specs/a/SPEC.md", &format!("{A}\nmore\n"));
    commit(d, "pr");
    assert!(
        files_outside_spec_paths(d, "main", "HEAD")
            .unwrap()
            .is_empty()
    );
}

// req:gate/files-outside-spec-paths@9ebb011d
#[test]
fn another_setting_changed() {
    let dir = base();
    let d = dir.path();
    write(
        d,
        "clm.toml",
        "approval = \"other\"\n[land]\ncheck = \"a\"\n",
    );
    commit(d, "policy");
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(
        d,
        "clm.toml",
        "approval = \"other\"\n[land]\ncheck = \"b\"\n",
    );
    commit(d, "pr");
    assert_eq!(
        files_outside_spec_paths(d, "main", "HEAD").unwrap(),
        ["clm.toml"]
    );
}

// req:gate/files-outside-spec-paths@9ebb011d
#[test]
fn an_unparseable_clm_toml_is_read_from_the_revision() {
    let dir = base();
    let d = dir.path();
    write(d, "clm.toml", "approval = \"other\"\n");
    commit(d, "policy");
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "clm.toml", "approval = [\n");
    commit(d, "pr");
    git(d, &["checkout", "-q", "main"]);
    assert!(
        files_outside_spec_paths(d, "main", "pr")
            .unwrap()
            .is_empty()
    );
}
