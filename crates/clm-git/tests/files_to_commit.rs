mod common;

use clm_git::files_to_commit;
use common::repo;
use std::path::PathBuf;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn tracked_and_untracked_not_ignored() {
    // req:vcs/files-to-commit@fb39736c
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("src/a.rs", ""), (".gitignore", "build/\n")]);
    std::fs::write(dir.path().join("notes.txt"), "n").unwrap();
    std::fs::create_dir(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/out.o"), "o").unwrap();
    let found = files_to_commit(dir.path()).unwrap();
    assert_eq!(found, paths(&[".gitignore", "notes.txt", "src/a.rs"]));
}

#[test]
fn a_file_both_staged_and_changed_once() {
    // req:vcs/files-to-commit@fb39736c
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("src/a.rs", "one\n")]);
    std::fs::write(dir.path().join("src/a.rs"), "two\n").unwrap();
    assert_eq!(files_to_commit(dir.path()).unwrap(), paths(&["src/a.rs"]));
}

#[test]
fn a_deleted_file_is_listed_and_tracked_files_ignore_untracked() {
    // req:vcs/files-to-commit@fb39736c
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("gone.rs", ""), ("kept.rs", "")]);
    std::fs::remove_file(dir.path().join("gone.rs")).unwrap();
    std::fs::write(dir.path().join("new.rs"), "").unwrap();
    assert_eq!(
        files_to_commit(dir.path()).unwrap(),
        paths(&["gone.rs", "kept.rs", "new.rs"])
    );
    assert_eq!(
        clm_git::tracked_files(dir.path()).unwrap(),
        paths(&["gone.rs", "kept.rs"])
    );
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf8() {
    // req:vcs/files-to-commit@fb39736c
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("a.rs", "")]);
    let name = std::ffi::OsStr::from_bytes(b"\xff.rs");
    std::fs::write(dir.path().join(name), "").unwrap();
    let found = files_to_commit(dir.path()).unwrap();
    assert!(found.iter().any(|p| p.as_os_str().as_bytes() == b"\xff.rs"));
}
