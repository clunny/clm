//! The commands compare the checkout with the remote's main branch.

mod common;

use std::path::Path;

use clm_view::{Format, Name};
use clm_view_repo::Source;
use common::specd;

/// Runs `git` in `root`, which must succeed.
fn git(root: &Path, args: &[&str]) -> String {
    clm_git::git(root, args).unwrap_or_else(|e| panic!("git {args:?}: {e}"))
}

#[test]
fn the_command_compares_the_checkout_with_the_remotes_main() {
    // req:view/spec-component-lines@fcbd78c9
    // req:view.repo/shared-plumbing@c3db052e
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (path, text) in [
        ("specs/SPEC.md", specd("", &[])),
        ("specs/a/SPEC.md", specd("", &["x"])),
        ("specs/a/c/SPEC.md", specd("", &["x"])),
        ("specs/b/SPEC.md", specd("", &[])),
    ] {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "t"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "specs"]);
    let head = git(root, &["rev-parse", "HEAD"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "ahead"]);
    let ahead = git(root, &["rev-parse", "HEAD"]);
    let main = format!("refs/remotes/{}/main", clm_git::REMOTE);
    git(root, &["update-ref", &main, &ahead]);
    git(root, &["reset", "-q", "--hard", &head]);

    let out = clm_view_repo::view(root, Name::Spec, Format::Ansi, 100, Source::Tree(None)).unwrap();
    let text =
        clm_view_repo::view(root, Name::Spec, Format::Text, 100, Source::Tree(None)).unwrap();
    let out = common::strip_ansi(&out);
    assert_eq!(out, text);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!("commit {}  ⚠ 1 behind origin/main", &head[..7]),
        "{out}"
    );
    assert_eq!(lines.len(), 5, "{out}");
    for (line, start) in lines[1..]
        .iter()
        .zip(["clm ", "├─ a ", "│  └─ a.c ", "└─ b "])
    {
        assert!(line.starts_with(start), "{line:?} in {out}");
    }
    // The same four lines after the one naming the commit, with `--at origin/main`.
    let rev = format!("{}/main", clm_git::REMOTE);
    let source = Source::At {
        rev: &rev,
        budgets: None,
    };
    let at = clm_view_repo::view(root, Name::Spec, Format::Ansi, 100, source).unwrap();
    let at = common::strip_ansi(&at);
    let at_lines: Vec<&str> = at.lines().collect();
    assert!(
        at_lines[0].starts_with(&format!("{rev} {} · ", &ahead[..7])),
        "{at}"
    );
    assert_eq!(at_lines[1..], lines[1..], "{at}");
}
