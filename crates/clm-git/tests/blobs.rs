mod common;

use clm_git::{common_dir, tree_blobs};
use common::{repo, sh};

fn fixture(dir: &std::path::Path) {
    repo(
        dir,
        &[
            (".gitattributes", "a.txt ident\n"),
            ("a.txt", "$Id$\n"),
            ("sub/b.rs", "fn b() {}\n"),
        ],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("a.txt", dir.join("l")).unwrap();
        sh(dir, &["add", "l"]);
        sh(dir, &["commit", "-q", "-m", "link"]);
    }
    std::fs::write(dir.join("a.txt"), "edited\n").unwrap();
}

#[test]
fn a_commits_files() {
    // req:vcs/tree-blobs@850f6eda
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let got = tree_blobs(&dir.path().join("sub"), "HEAD", &|_| true).unwrap();
    let names: Vec<_> = got.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(names, [".gitattributes", "a.txt", "sub/b.rs"]);
    assert_eq!(got[1].1, b"$Id$\n");
    assert_eq!(sh(dir.path(), &["status", "--porcelain"]), "M a.txt");
}

#[test]
fn only_the_kept_paths() {
    // req:vcs/tree-blobs@850f6eda
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let got = tree_blobs(dir.path(), "HEAD", &|p| p.starts_with("sub/")).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, "sub/b.rs");
}

#[test]
fn a_missing_blob_and_a_bad_commit() {
    // req:vcs/tree-blobs@850f6eda
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("a.txt", "lost\n")]);
    assert!(tree_blobs(dir.path(), "nope", &|_| true).is_err());
    let oid = sh(dir.path(), &["rev-parse", "HEAD:a.txt"]);
    let object = dir
        .path()
        .join(".git/objects")
        .join(&oid[..2])
        .join(&oid[2..]);
    let mut perms = std::fs::metadata(&object).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&object, perms).unwrap();
    std::fs::remove_file(&object).unwrap();
    let err = tree_blobs(dir.path(), "HEAD", &|_| true).unwrap_err();
    assert!(err.to_string().contains("a.txt"), "{err}");
}

#[test]
fn common_dir_from_a_worktree() {
    // req:vcs/common-dir@b7afa91b
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("r");
    std::fs::create_dir(&main).unwrap();
    repo(&main, &[("a", "x")]);
    let linked = dir.path().join("w");
    sh(
        &main,
        &["worktree", "add", "-q", linked.to_str().unwrap(), "-b", "w"],
    );
    let want = main.join(".git").canonicalize().unwrap();
    assert!(common_dir(&linked).unwrap().is_absolute());
    assert_eq!(common_dir(&main).unwrap().canonicalize().unwrap(), want);
    assert_eq!(common_dir(&linked).unwrap().canonicalize().unwrap(), want);
}

#[test]
fn common_dir_outside_a_repository() {
    // req:vcs/common-dir@b7afa91b
    let dir = tempfile::tempdir().unwrap();
    assert!(common_dir(dir.path()).is_err());
}
