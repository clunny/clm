#![cfg(unix)]
// The only test in this binary, so setting the process's environment disturbs no other.

mod common;

use clm_git::{Options, git, run, tracked_files};
use common::sh;
use std::path::Path;

/// `git` against git directory `g` and work tree `w`, for setup before the environment is set.
fn with(g: &Path, w: &Path, args: &[&str]) -> String {
    let mut all = vec![
        "--git-dir",
        g.to_str().unwrap(),
        "--work-tree",
        w.to_str().unwrap(),
    ];
    all.extend_from_slice(args);
    sh(w, &all)
}

fn names(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    (dir.join("w"), dir.join("g"))
}

fn named_clone(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let (w, g) = names(dir);
    sh(
        dir,
        &[
            "init",
            "-q",
            "-b",
            "main",
            "--separate-git-dir",
            g.to_str().unwrap(),
            w.to_str().unwrap(),
        ],
    );
    std::fs::remove_file(w.join(".git")).unwrap();
    with(&g, &w, &["config", "user.email", "t@example.com"]);
    with(&g, &w, &["config", "user.name", "t"]);
    (w, g)
}

#[test]
fn commands_follow_their_own_clone() {
    // req:vcs/git-follows-its-clone@a8190b8f
    let dir = tempfile::tempdir().unwrap();
    let (w, g) = named_clone(dir.path());
    std::fs::write(w.join("a"), "a").unwrap();
    std::fs::write(w.join(".gitignore"), ".clm/\n").unwrap();
    with(&g, &w, &["add", "-A"]);
    with(&g, &w, &["commit", "-q", "-m", "first"]);
    let first = with(&g, &w, &["rev-parse", "HEAD"]);
    std::fs::write(w.join("a"), "a2").unwrap();
    with(&g, &w, &["commit", "-q", "-am", "second"]);
    let second = with(&g, &w, &["rev-parse", "HEAD"]);
    let c = dir.path().join("c");
    std::fs::create_dir(&c).unwrap();
    std::fs::write(c.join("b"), "b").unwrap();
    let t = dir.path().join("t");
    with(
        &g,
        &w,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            t.to_str().unwrap(),
            &first,
        ],
    );
    let n = w.join(".clm").join("n");
    with(
        &g,
        &w,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            n.to_str().unwrap(),
            &first,
        ],
    );

    let other = tempfile::tempdir().unwrap();
    let (w2, g2) = named_clone(other.path());
    std::fs::write(w2.join("z"), "z").unwrap();
    with(&g2, &w2, &["add", "-A"]);
    with(&g2, &w2, &["commit", "-q", "-m", "only"]);

    // SAFETY: no other thread of this binary reads or writes the environment.
    unsafe {
        std::env::set_var("GIT_DIR", &g);
        std::env::set_var("GIT_WORK_TREE", &w);
    }

    // A plain copy beside a clone the environment names.
    for err in [
        git(&c, &["ls-files"]).unwrap_err().to_string(),
        tracked_files(&c).unwrap_err().to_string(),
    ] {
        assert!(err.contains("not a git repository"), "{err}");
    }
    assert_eq!(git(&w, &["ls-files"]).unwrap(), ".gitignore\na");

    // A temporary worktree beside it.
    git(&t, &["checkout", "--quiet", "--detach", &second]).unwrap();
    assert_eq!(git(&t, &["rev-parse", "HEAD"]).unwrap(), second);
    assert_eq!(
        git(&w, &["symbolic-ref", "--short", "HEAD"]).unwrap(),
        "main"
    );
    assert_eq!(git(&w, &["status", "--porcelain"]).unwrap(), "");

    // A worktree nested in the clone.
    git(&n, &["checkout", "--quiet", "--detach", &second]).unwrap();
    let shown = git(&n, &["rev-parse", "--show-toplevel", "HEAD"]).unwrap();
    let n_real = std::fs::canonicalize(&n).unwrap();
    assert_eq!(shown, format!("{}\n{second}", n_real.display()));
    assert_eq!(
        git(&w, &["symbolic-ref", "--short", "HEAD"]).unwrap(),
        "main"
    );
    assert_eq!(git(&w, &["status", "--porcelain"]).unwrap(), "");

    // The caller's variables are kept outside.
    let idx = dir.path().join("idx");
    let g_index = std::fs::read(g.join("index")).unwrap();
    let t_index = std::fs::read(g.join("worktrees/t/index")).unwrap();
    let env = [("GIT_INDEX_FILE", idx.as_os_str())];
    run(
        &t,
        "git",
        &["read-tree", "HEAD"],
        Options {
            stdin: None,
            env: &env,
        },
    )
    .unwrap();
    assert!(idx.exists());
    assert_eq!(std::fs::read(g.join("index")).unwrap(), g_index);
    assert_eq!(std::fs::read(g.join("worktrees/t/index")).unwrap(), t_index);

    // run_within follows the rule too; a plain subdirectory of the clone is inside it.
    let limit = std::time::Duration::from_secs(10);
    let err = clm_git::run_within(&c, "git", &["ls-files"], Options::default(), limit)
        .unwrap_err()
        .to_string();
    assert!(err.contains("not a git repository"), "{err}");
    std::fs::create_dir(w.join("sub")).unwrap();
    assert_eq!(
        git(&w.join("sub"), &["rev-parse", "--show-prefix"]).unwrap(),
        "sub/"
    );

    // A program other than git sees the environment as the process has it.
    let out = run(&c, "sh", &["-c", "echo $GIT_DIR"], Options::default()).unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), g.to_str().unwrap());

    // The work tree is found again when a variable's value changes.
    unsafe {
        std::env::set_var("GIT_DIR", &g2);
        std::env::set_var("GIT_WORK_TREE", &w2);
    }
    assert_eq!(git(&w2, &["ls-files"]).unwrap(), "z");
    let err = git(&w, &["ls-files"]).unwrap_err().to_string();
    assert!(err.contains("not a git repository"), "{err}");

    // With neither variable set, nothing is removed.
    unsafe {
        std::env::remove_var("GIT_DIR");
        std::env::remove_var("GIT_WORK_TREE");
    }
    assert_eq!(git(&t, &["rev-parse", "HEAD"]).unwrap(), second);
}
