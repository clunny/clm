mod common;
mod exec;

use clm_git::{extract_paths, extract_tree};
use common::{repo, sh};
use std::path::{Path, PathBuf};

/// Every file under `dir`, relative to it, sorted.
fn files(dir: &Path) -> Vec<PathBuf> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.push(path.strip_prefix(base).unwrap().to_path_buf());
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn names(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

#[test]
fn only_the_paths_asked_for() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[
            (".gitattributes", "*.rs text\n"),
            ("specs/SPEC.md", "root\n"),
            ("specs/a/SPEC.md", "a\n"),
            ("src/main.rs", "fn main() {}\n"),
        ],
    );
    std::fs::write(dir.path().join("specs/SPEC.md"), "edited\n").unwrap();
    let status = sh(dir.path(), &["status", "--porcelain"]);
    let index = std::fs::read(dir.path().join(".git/index")).unwrap();

    let tree = extract_paths(dir.path(), "HEAD", &["specs"]).unwrap();

    assert_eq!(
        files(tree.path()),
        names(&["specs/SPEC.md", "specs/a/SPEC.md"])
    );
    let read = |p: &str| std::fs::read_to_string(tree.path().join(p)).unwrap();
    assert_eq!(read("specs/SPEC.md"), "root\n");
    assert_eq!(read("specs/a/SPEC.md"), "a\n");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap(),
        "edited\n"
    );
    assert_eq!(sh(dir.path(), &["status", "--porcelain"]), status);
    assert_eq!(std::fs::read(dir.path().join(".git/index")).unwrap(), index);
}

#[test]
fn files_directories_and_a_trailing_slash() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[
            ("specs/SPEC.md", "root\n"),
            ("specs/a/SPEC.md", "a\n"),
            ("specsx/b", "b\n"),
            ("top.rs", "top\n"),
        ],
    );
    let tree = extract_paths(dir.path(), "HEAD", &["specs/a/", "top.rs"]).unwrap();
    assert_eq!(files(tree.path()), names(&["specs/a/SPEC.md", "top.rs"]));
    let overlapping = extract_paths(dir.path(), "HEAD", &["specs", "specs/a"]).unwrap();
    assert_eq!(
        files(overlapping.path()),
        names(&["specs/SPEC.md", "specs/a/SPEC.md"])
    );
}

#[test]
fn no_paths_is_an_empty_directory() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("a", "a\n")]);
    let tree = extract_paths(dir.path(), "HEAD", &[]).unwrap();
    assert!(files(tree.path()).is_empty());
}

#[test]
fn a_missing_path() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("src/main.rs", "fn main() {}\n")]);
    let err = extract_paths(dir.path(), "HEAD", &["src", "specs"]).unwrap_err();
    assert!(err.to_string().contains("specs"), "{err}");
}

#[test]
fn malformed_paths_are_errors_naming_them() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("specs/SPEC.md", "x\n")]);
    for bad in ["", "/specs", "./specs", "specs/../specs", "specs/.", ".."] {
        let err = extract_paths(dir.path(), "HEAD", &[bad]).unwrap_err();
        assert!(
            err.to_string().contains(&format!("`{bad}`")),
            "{bad}: {err}"
        );
    }
}

#[test]
fn literal_top_relative_paths() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("sub/a.rs", "a\n"), ("top.rs", "top\n")]);
    let sub = dir.path().join("sub");
    let tree = extract_paths(&sub, "HEAD", &["top.rs"]).unwrap();
    assert_eq!(files(tree.path()), names(&["top.rs"]));
    let err = extract_paths(&sub, "HEAD", &["*.rs"]).unwrap_err();
    assert!(err.to_string().contains("*.rs"), "{err}");
}

#[test]
fn no_hook_runs() {
    // req:vcs/extract-paths@e3b00c99
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("specs/SPEC.md", "x\n")]);
    let hook = dir.path().join(".git/hooks/post-checkout");
    let hooked = dir.path().join("hooked");
    exec::write_exec(&hook, &format!("#!/bin/sh\ntouch '{}'\n", hooked.display()));
    // The hook is live: a real checkout runs it.
    sh(dir.path(), &["checkout", "-q", "HEAD", "--", "specs"]);
    assert!(hooked.exists());
    std::fs::remove_file(&hooked).unwrap();

    extract_paths(dir.path(), "HEAD", &["specs"]).unwrap();
    assert!(!hooked.exists());
}

/// `specs/SPEC.md` as a fresh clone of `dir` checks it out.
fn checked_out(dir: &Path) -> Vec<u8> {
    let scratch = tempfile::tempdir().unwrap();
    let clone = scratch.path().join("clone");
    sh(
        scratch.path(),
        &[
            "clone",
            "-q",
            dir.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    std::fs::read(clone.join("specs/SPEC.md")).unwrap()
}

#[test]
fn line_endings_from_revs_attributes() {
    // req:vcs/extract-as-checkout@2d3dc0bd
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[
            (".gitattributes", "*.md text eol=crlf\n"),
            ("specs/SPEC.md", "a\nb\n"),
        ],
    );
    assert_eq!(checked_out(dir.path()), b"a\r\nb\r\n");
    let tree = extract_tree(dir.path(), "HEAD").unwrap();
    let paths = extract_paths(dir.path(), "HEAD", &["specs"]).unwrap();
    for out in [&tree, &paths] {
        assert_eq!(
            std::fs::read(out.path().join("specs/SPEC.md")).unwrap(),
            checked_out(dir.path())
        );
    }
}

#[test]
fn smudge_filter_applied() {
    // req:vcs/extract-as-checkout@2d3dc0bd
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    sh(dir.path(), &["config", "filter.up.smudge", "tr a-z A-Z"]);
    sh(dir.path(), &["config", "filter.up.clean", "cat"]);
    common::commit(
        dir.path(),
        &[
            (".gitattributes", "specs/SPEC.md filter=up\n"),
            ("specs/SPEC.md", "committed text\n"),
        ],
        "filtered",
    );
    let paths = extract_paths(dir.path(), "HEAD", &["specs"]).unwrap();
    assert_eq!(
        std::fs::read_to_string(paths.path().join("specs/SPEC.md")).unwrap(),
        "COMMITTED TEXT\n"
    );
}

#[test]
fn ident_expanded() {
    // req:vcs/extract-as-checkout@2d3dc0bd
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[
            (".gitattributes", "*.md ident\n"),
            ("specs/SPEC.md", "$Id$\n"),
        ],
    );
    let blob = sh(dir.path(), &["rev-parse", "HEAD:specs/SPEC.md"]);
    let paths = extract_paths(dir.path(), "HEAD", &["specs"]).unwrap();
    assert_eq!(
        std::fs::read_to_string(paths.path().join("specs/SPEC.md")).unwrap(),
        format!("$Id: {blob} $\n")
    );
}

#[test]
fn export_attributes_ignored() {
    // req:vcs/extract-as-checkout@2d3dc0bd
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[
            (
                ".gitattributes",
                "specs/** export-ignore\nspecs/SPEC.md export-subst\n",
            ),
            ("specs/SPEC.md", "$Format:%H$\n"),
        ],
    );
    let tree = extract_tree(dir.path(), "HEAD").unwrap();
    let paths = extract_paths(dir.path(), "HEAD", &["specs"]).unwrap();
    for out in [&tree, &paths] {
        assert_eq!(
            std::fs::read_to_string(out.path().join("specs/SPEC.md")).unwrap(),
            "$Format:%H$\n"
        );
    }
}
