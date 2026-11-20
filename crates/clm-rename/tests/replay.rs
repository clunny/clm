use std::fs;
use std::path::Path;

use clm_rename::{rename, replays};
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
            "commit",
            "-qm",
            message,
        ],
    );
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n\n## Design\nthe sprocket\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";
const CHANGE: &str =
    "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nRename.\n";

/// A repository with a base commit, `HEAD` after `clm rename` ran and was committed, and
/// the base's ID.
fn renamed() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "crates/x/src/lib.rs", "fn main() {}\n");
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(d, "changes/r/change.md", CHANGE);
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    commit(d, "rename");
    (dir, base)
}

#[test]
// req:rename/replay@c35e7a83
fn an_untouched_rename_branch() {
    let (dir, base) = renamed();
    assert!(replays(dir.path(), &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn a_marker_held_back_by_code_that_names_an_old_entry_replays() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    let a = A
        .replace(
            "owners: [\"@a\"]\n",
            "owners: [\"@a\"]\npaths: [\"plugin/**\"]\n",
        )
        .replace("X SHALL hold.", "X SHALL hold (`slow-entry`).");
    write(d, "specs/a/SPEC.md", &a);
    write(
        d,
        "ARCHITECTURE.md",
        "# Architecture\n\n## Constraints\n\n### Constraint: slow-entry\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n",
    );
    write(d, "plugin/s.md", "see `slow-entry`\n");
    let version = clm_model::load(d)
        .0
        .requirement("a/x")
        .unwrap()
        .version()
        .to_string();
    let cite = format!("// {}a/x@{version}\n", "req:");
    write(d, "crates/x/tests/t.rs", &cite);
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(
        d,
        "changes/r/change.md",
        "---\nstatus: proposed\nrenames:\n  - requirement: architecture/slow-entry\n    to: quick-entry\n---\n\n## Intent\nRename.\n",
    );
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    assert_eq!(
        fs::read_to_string(d.join("crates/x/tests/t.rs")).unwrap(),
        cite
    );
    commit(d, "rename");
    assert!(replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn an_extra_edit() {
    let (dir, base) = renamed();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "fn main() { println!(); }\n");
    commit(d, "edit");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn a_defect_report_filed() {
    let (dir, base) = renamed();
    let d = dir.path();
    write(d, "backlog/d.md", "---\ntitle: D\nkind: bug\n---\n\nbody\n");
    commit(d, "defect");
    assert!(replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn an_edit_of_the_spec_is_not_the_rename() {
    let (dir, base) = renamed();
    let d = dir.path();
    let spec = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    write(
        d,
        "specs/a/SPEC.md",
        &spec.replace("SHALL hold", "SHALL NOT hold"),
    );
    commit(d, "reword");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn no_added_change_is_no_replay() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(d, "specs/a/SPEC.md", A);
    commit(d, "more");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn a_change_without_renames_is_no_replay() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nx\n",
    );
    commit(d, "change");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/other-deltas@1f0dee44
fn a_rename_branch_that_rebased_another_delta() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &A.replace("c\n\n## Design", "Print the sprocket.\n\n## Design"),
    );
    let graph = clm_model::load(d).0;
    let hashes = clm_apply::hashes(&graph.nodes["a"]);
    let contract = &hashes.iter().find(|(e, _)| e == "contract").unwrap().1;
    write(
        d,
        "changes/y/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nEdit.\n",
    );
    write(
        d,
        "changes/y/deltas/a.md",
        &format!(
            "---\ncomponent: a\nbase: {{ contract: \"{contract}\" }}\n---\n## MODIFIED Contract\nPrint the sprocket.\n\nExit 0.\n"
        ),
    );
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(
        d,
        "changes/r/change.md",
        "---\nstatus: proposed\nrenames:\n  - term: sprocket\n    to: cogs\n---\n\n## Intent\nRename.\n",
    );
    git(d, &["add", "-A"]);
    let printed = rename(d, "r").unwrap();
    assert_eq!(printed[1], "rebased contract of a in changes/y/deltas/a.md");
    commit(d, "rename");
    assert!(replays(d, &base, "HEAD").unwrap());
}

#[cfg(unix)]
#[test]
// req:rename/replay@c35e7a83
fn a_symbolic_link_added() {
    let (dir, base) = renamed();
    let d = dir.path();
    std::os::unix::fs::symlink("README.md", d.join("l")).unwrap();
    commit(d, "link");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[cfg(unix)]
#[test]
// req:rename/replay@c35e7a83
fn a_mode_changed() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, base) = renamed();
    let d = dir.path();
    let lib = d.join("crates/x/src/lib.rs");
    fs::set_permissions(&lib, fs::Permissions::from_mode(0o755)).unwrap();
    commit(d, "mode");
    let mode = git(d, &["ls-tree", "HEAD", "crates/x/src/lib.rs"]);
    assert!(mode.starts_with("100755"), "{mode}");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[cfg(unix)]
#[test]
// req:rename/replay@c35e7a83
fn an_executable_file_in_the_base_replays() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "run.sh", "#!/bin/sh\n");
    fs::set_permissions(d.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(d, "changes/r/change.md", CHANGE);
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    commit(d, "rename");
    assert!(replays(d, &base, "HEAD").unwrap());
}

#[cfg(unix)]
#[test]
// req:rename/replay@c35e7a83
fn a_base_that_holds_a_symbolic_link() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    std::os::unix::fs::symlink("README.md", d.join("l")).unwrap();
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(d, "changes/r/change.md", CHANGE);
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    commit(d, "rename");
    assert!(replays(d, &base, "HEAD").unwrap());
    fs::remove_file(d.join("l")).unwrap();
    std::os::unix::fs::symlink("ARCHITECTURE.md", d.join("l")).unwrap();
    commit(d, "repoint");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn a_submodule_added() {
    let (dir, base) = renamed();
    let d = dir.path();
    let commit_id = git(d, &["rev-parse", "HEAD"]);
    let entry = format!("160000,{commit_id},sub");
    git(d, &["update-index", "--add", "--cacheinfo", &entry]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "submodule",
        ],
    );
    assert!(!replays(d, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn a_rename_that_refuses_on_the_base() {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(
        d,
        "changes/r/change.md",
        &CHANGE.replace("requirement: a/x", "requirement: a/none"),
    );
    commit(d, "change");
    assert!(!replays(d, &base, "HEAD").unwrap());
}

/// A git repository whose CLM root is `tool/`, with a base commit, `HEAD` after
/// `clm rename` ran in `tool/` and was committed, and the base's ID.
fn renamed_in_a_subdirectory() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "README.md", "top\n");
    let t = d.join("tool");
    write(&t, "specs/SPEC.md", ROOT);
    write(&t, "specs/a/SPEC.md", A);
    write(&t, "notes.txt", "a/x\n");
    commit(d, "base");
    let base = git(d, &["rev-parse", "HEAD"]);
    write(&t, "changes/r/change.md", CHANGE);
    git(d, &["add", "-A"]);
    rename(&t, "r").unwrap();
    commit(d, "rename");
    (dir, base)
}

#[test]
// req:rename/replay@c35e7a83
fn a_subdirectory_root() {
    let (dir, base) = renamed_in_a_subdirectory();
    let t = dir.path().join("tool");
    assert!(t.join("changes/archive/r/change.md").exists());
    assert!(replays(&t, &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn a_file_outside_a_subdirectory_root() {
    let (dir, base) = renamed_in_a_subdirectory();
    let d = dir.path();
    write(d, "README.md", "top, edited\n");
    commit(d, "outside");
    assert!(!replays(&d.join("tool"), &base, "HEAD").unwrap());
}

#[test]
// req:rename/replay@c35e7a83
fn two_added_changes_are_no_replay() {
    let (dir, base) = renamed();
    let d = dir.path();
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nx\n",
    );
    commit(d, "second");
    assert!(!replays(d, &base, "HEAD").unwrap());
}
