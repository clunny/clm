mod common;

use clm_git::{current_branch, extract_tree, fetch, is_ancestor, worktrees};
use common::{commit, repo, sh};

#[test]
fn ancestry() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    let a = sh(dir.path(), &["rev-parse", "HEAD"]);
    let b = commit(dir.path(), &[("b", "b")], "b");
    assert!(is_ancestor(dir.path(), &a, &b).unwrap());
    assert!(!is_ancestor(dir.path(), &b, &a).unwrap());
}

#[test]
fn ancestry_of_an_unknown_revision_is_an_error() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    assert!(is_ancestor(dir.path(), "HEAD", "no-such-rev").is_err());
}

#[test]
fn current_branch_names_the_branch() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    sh(dir.path(), &["switch", "-q", "-c", "impl/a"]);
    assert_eq!(
        current_branch(dir.path()).unwrap().as_deref(),
        Some("impl/a")
    );
}

#[test]
fn detached_head_has_no_branch() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    sh(dir.path(), &["switch", "-q", "--detach", "HEAD"]);
    assert_eq!(current_branch(dir.path()).unwrap(), None);
}

#[test]
fn locked_worktree() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    repo(&root, &[]);
    let linked = dir.path().join("linked");
    let detached = dir.path().join("detached");
    let reasoned = dir.path().join("reasoned");
    sh(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            linked.to_str().unwrap(),
        ],
    );
    sh(&root, &["worktree", "lock", linked.to_str().unwrap()]);
    sh(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            detached.to_str().unwrap(),
        ],
    );
    sh(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/b",
            reasoned.to_str().unwrap(),
        ],
    );
    sh(
        &root,
        &[
            "worktree",
            "lock",
            "--reason",
            "a build",
            reasoned.to_str().unwrap(),
        ],
    );

    let list = worktrees(&root).unwrap();
    assert_eq!(list.len(), 4, "{list:?}");
    let find = |p: &std::path::Path| {
        let p = p.canonicalize().unwrap();
        list.iter()
            .find(|w| w.path.canonicalize().unwrap() == p)
            .unwrap_or_else(|| panic!("{p:?} not in {list:?}"))
    };
    let main = find(&root);
    assert_eq!((main.branch.as_deref(), main.locked), (Some("main"), false));
    let a = find(&linked);
    assert_eq!((a.branch.as_deref(), a.locked), (Some("impl/a"), true));
    let d = find(&detached);
    assert_eq!((d.branch.as_deref(), d.locked), (None, false));
    let r = find(&reasoned);
    assert_eq!((r.branch.as_deref(), r.locked), (Some("impl/b"), true));
}

#[test]
fn fetch_brings_origin_up_to_date() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("origin");
    std::fs::create_dir(&origin).unwrap();
    repo(&origin, &[]);
    let clone = dir.path().join("clone");
    sh(
        dir.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    let new = commit(&origin, &[("n", "n")], "new");
    fetch(&clone).unwrap();
    assert_eq!(sh(&clone, &["rev-parse", "origin/main"]), new);
}

#[test]
fn extracted_tree_is_the_committed_text() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("origin");
    std::fs::create_dir(&origin).unwrap();
    repo(&origin, &[("specs/SPEC.md", "committed\n")]);
    let root = dir.path().join("clone");
    sh(
        dir.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            root.to_str().unwrap(),
        ],
    );
    std::fs::write(root.join("specs/SPEC.md"), "edited\n").unwrap();
    let status = sh(&root, &["status", "--porcelain"]);
    let index = std::fs::read(root.join(".git/index")).unwrap();

    let tree = extract_tree(&root, "origin/main").unwrap();

    assert_eq!(
        std::fs::read_to_string(tree.path().join("specs/SPEC.md")).unwrap(),
        "committed\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("specs/SPEC.md")).unwrap(),
        "edited\n"
    );
    assert_eq!(sh(&root, &["status", "--porcelain"]), status);
    assert_eq!(std::fs::read(root.join(".git/index")).unwrap(), index);
}

#[test]
fn extracting_an_unknown_revision_is_an_error() {
    // req:vcs/repo-queries@e04fba5e
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    assert!(extract_tree(dir.path(), "no-such-rev").is_err());
}
