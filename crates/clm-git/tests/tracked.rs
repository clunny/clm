mod common;

use clm_git::{tracked_files, tracked_matching};
use common::{commit, repo, sh};
use std::path::PathBuf;
use std::process::Command;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn globs_match_by_git_rules() {
    // req:vcs/tracked-files@8462e6d9
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[("src/a/x.rs", ""), ("src/a/b/y.rs", ""), ("src/b/z.rs", "")],
    );
    let found = tracked_matching(dir.path(), &["src/a/**".to_string()]).unwrap();
    assert_eq!(found, paths(&["src/a/b/y.rs", "src/a/x.rs"]));
    // Under git's glob rules `*` stops at `/`, which plain pathspecs don't.
    let star = tracked_matching(dir.path(), &["src/*/x.rs".to_string(), "*.rs".to_string()]);
    assert_eq!(star.unwrap(), paths(&["src/a/x.rs"]));
    let all = tracked_files(dir.path()).unwrap();
    assert_eq!(all, paths(&["src/a/b/y.rs", "src/a/x.rs", "src/b/z.rs"]));
}

#[test]
fn paths_are_relative_to_root() {
    // req:vcs/tracked-files@8462e6d9
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("top.rs", ""), ("src/a/x.rs", "")]);
    let src = dir.path().join("src");
    assert_eq!(tracked_files(&src).unwrap(), paths(&["a/x.rs"]));
    let found = tracked_matching(&src, &["a/*".to_string()]).unwrap();
    assert_eq!(found, paths(&["a/x.rs"]));
}

#[test]
fn no_globs_runs_no_git() {
    // req:vcs/tracked-files@8462e6d9
    let dir = tempfile::tempdir().unwrap();
    assert!(tracked_matching(dir.path(), &[]).unwrap().is_empty());
    assert!(tracked_files(dir.path()).is_err());
}

#[test]
fn a_conflicted_file_once() {
    // req:vcs/tracked-files@8462e6d9
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("src/a/x.rs", "base\n")]);
    sh(dir.path(), &["switch", "-q", "-c", "other"]);
    commit(dir.path(), &[("src/a/x.rs", "other\n")], "other");
    sh(dir.path(), &["switch", "-q", "main"]);
    commit(dir.path(), &[("src/a/x.rs", "main\n")], "main");
    let merge = Command::new("git")
        .args(["merge", "-q", "other"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!merge.status.success());
    let stages = sh(dir.path(), &["ls-files", "--stage"]);
    assert_eq!(stages.lines().count(), 3, "{stages}");

    assert_eq!(tracked_files(dir.path()).unwrap(), paths(&["src/a/x.rs"]));
    let found = tracked_matching(dir.path(), &["src/**".to_string()]).unwrap();
    assert_eq!(found, paths(&["src/a/x.rs"]));
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf8() {
    // req:vcs/tracked-files@8462e6d9
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("src/ok.rs", "")]);
    let name = OsStr::from_bytes(b"src/\xff.rs");
    if std::fs::write(dir.path().join(name), "").is_err() {
        // A filesystem that refuses the name can't hold the fixture.
        return;
    }
    sh(dir.path(), &["add", "-A"]);
    let found = tracked_files(dir.path()).unwrap();
    assert!(
        found
            .iter()
            .any(|p| p.as_os_str().as_bytes() == b"src/\xff.rs"),
        "{found:?}"
    );
}
