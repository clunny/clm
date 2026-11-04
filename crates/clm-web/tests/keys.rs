//! The code key, computed without git from the Trees API's listings, against
//! `clm_ledger::code_key`.

use std::path::Path;
use std::process::Command;

use clm_web_platform::keys::{TreeLine, code_key};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn write(dir: &Path, files: &[(&str, &[u8])]) {
    for (rel, bytes) in files {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

fn commit(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "c", "--allow-empty"]);
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    dir
}

#[test]
fn code_key_matches_ledger() {
    let dir = repo();
    let d = dir.path();
    write(
        d,
        &[
            ("src/a.rs", b"fn a() {}\n"),
            ("src/a-b.rs", b"fn ab() {}\n"),
            ("src.rs", b"\n"),
            ("specs/SPEC.md", b"# spec\n"),
            ("changes/c/change.md", b"change\n"),
            ("backlog/b.md", b"item\n"),
            ("ARCHITECTURE.md", b"arch\n"),
            ("CLAUDE.md", b"claude\n"),
            ("docs/x.md", b"doc\n"),
            ("docs/guide/start.md", b"guide\n"),
            ("README.md", b"readme\n"),
            ("web/README.md", b"not top-level\n"),
        ],
    );
    commit(d);
    // What the Trees API lists: every entry, trees too, in no order we rely on.
    let listing = git(d, &["ls-tree", "-r", "-t", "HEAD"]);
    let mut entries: Vec<(String, String, String, String)> = listing
        .lines()
        .map(|l| {
            let (meta, path) = l.split_once('\t').unwrap();
            let mut f = meta.split(' ');
            let (mode, kind, sha) = (f.next().unwrap(), f.next().unwrap(), f.next().unwrap());
            (mode.into(), kind.into(), sha.into(), path.into())
        })
        .collect();
    entries.reverse();
    let lines = entries.iter().map(|(mode, kind, sha, path)| TreeLine {
        mode,
        kind,
        sha,
        path,
    });
    assert_eq!(
        code_key(lines),
        clm_ledger::code_key(d, "HEAD").unwrap(),
        "the code key `clm land` records"
    );
}
