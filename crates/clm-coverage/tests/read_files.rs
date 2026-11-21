//! `markers-from-files`: the files `scan` and `read_files` read, outside git and in a
//! checkout whose index holds a file at several merge stages.
#![cfg(unix)]

use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tempfile::TempDir;

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = concat!("req", ":");

fn write(root: &Path, rel: impl AsRef<Path>, content: impl AsRef<[u8]>) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A Rust test citing `id`, its marker on line 3.
fn test_citing(id: &str) -> String {
    format!("#[test]\nfn t() {{\n    // {REQ}{id}\n}}\n")
}

fn located(markers: &[clm_coverage::Marker]) -> Vec<(String, String)> {
    markers
        .iter()
        .map(|m| (m.id.clone(), m.to_string()))
        .collect()
}

#[test]
fn scenario_a_tree_outside_git() {
    // req:check.coverage/markers-from-files@50921197
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    // Pin the directory outside any git work tree, whatever holds the temporary directory.
    // SAFETY: nextest runs each test in a process of its own.
    unsafe { std::env::set_var("GIT_CEILING_DIRECTORIES", d.parent().unwrap()) };
    let rev_parse = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(d)
        .output()
        .unwrap();
    assert!(!rev_parse.status.success(), "{d:?} is in a git work tree");

    write(d, "t.rs", test_citing("a/x"));
    write(d, ".hidden/t.rs", test_citing("a/w"));
    write(d, "x.MD", format!("{REQ}a/s\n"));
    write(d, "n.md", format!("{REQ}a/t\n"));
    write(d, ".git/t.rs", test_citing("a/y"));
    write(d, "target/t.rs", test_citing("a/y"));
    let mut bad = b"\xff\n".to_vec();
    bad.extend(test_citing("a/z").bytes());
    write(d, "b.rs", bad);
    write(d, OsStr::from_bytes(b"c\xff.rs"), test_citing("a/u"));
    std::os::unix::fs::symlink(d, d.join("loop")).unwrap();

    let scan = clm_coverage::scan(d);
    let mut req = located(&scan.req);
    req.sort();
    assert_eq!(
        req,
        [
            ("a/s".to_string(), "x.MD:1".to_string()),
            ("a/w".to_string(), ".hidden/t.rs:3".to_string()),
            ("a/x".to_string(), "t.rs:3".to_string()),
        ]
    );
    let files = clm_coverage::read_files(d);
    let expected: Vec<(PathBuf, String)> = [".hidden/t.rs", "n.md", "t.rs", "x.MD"]
        .into_iter()
        .map(|p| (PathBuf::from(p), fs::read_to_string(d.join(p)).unwrap()))
        .collect();
    assert_eq!(files, expected);
}

#[test]
fn a_link_to_a_file_is_read_through_it() {
    // req:check.coverage/markers-from-files@50921197
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    // SAFETY: nextest runs each test in a process of its own.
    unsafe { std::env::set_var("GIT_CEILING_DIRECTORIES", d.parent().unwrap()) };
    write(d, "real/t.rs", test_citing("a/x"));
    std::os::unix::fs::symlink(d.join("real/t.rs"), d.join("l.rs")).unwrap();
    std::os::unix::fs::symlink(d.join("real"), d.join("dir")).unwrap();
    let paths: Vec<PathBuf> = clm_coverage::read_files(d)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    assert_eq!(paths, [PathBuf::from("l.rs"), PathBuf::from("real/t.rs")]);
}

fn git(d: &Path, args: &[&str], stdin: Option<&str>) -> String {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(d)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.unwrap_or("").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn scenario_a_file_in_conflict() {
    // req:check.coverage/markers-from-files@50921197
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n",
    );
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    let version = graph.requirement("a/x").unwrap().version();
    write(d, "t.rs", test_citing(&format!("a/x@{version}")));
    git(d, &["init", "-q"], None);
    let blob = git(d, &["hash-object", "-w", "t.rs"], None);
    let blob = blob.trim();
    let stages: String = (1..=3)
        .map(|s| format!("100644 {blob} {s}\tt.rs\n"))
        .collect();
    git(d, &["update-index", "--index-info"], Some(&stages));
    let listed = git(d, &["ls-files", "--stage", "t.rs"], None);
    assert_eq!(listed.lines().count(), 3, "{listed}");

    let files = clm_coverage::read_files(d);
    let t: Vec<_> = files
        .iter()
        .filter(|(p, _)| p == Path::new("t.rs"))
        .collect();
    assert_eq!(t.len(), 1, "{files:?}");
    let scan = clm_coverage::scan(d);
    assert_eq!(
        located(&scan.req),
        [("a/x".to_string(), "t.rs:3".to_string())]
    );
    let lines: Vec<String> = clm_coverage::report(&graph, &scan.req)
        .iter()
        .map(|c| c.to_string())
        .collect();
    assert_eq!(lines, ["a/x current t.rs:3"]);
}
