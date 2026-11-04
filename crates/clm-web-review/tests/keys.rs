//! The fingerprint, computed without git from the Trees API's listings, against
//! `clm fingerprint` (`tree-diff-fingerprint`).

use std::path::Path;
use std::process::Command;

use clm_web_platform::keys::{fingerprint, listing_of};

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

fn long(n: usize, mark: &str) -> String {
    (0..n)
        .map(|i| {
            if i == 3 || i == 40 {
                format!("line {i} {mark}\n")
            } else {
                format!("line {i}\n")
            }
        })
        .collect()
}

/// The Trees API's listing of `rev`, built from `git ls-tree -r -t`: every entry, trees
/// too, with full IDs and paths as text, in an order we don't rely on.
fn trees_api(dir: &Path, rev: &str) -> serde_json::Value {
    let listing = git(dir, &["ls-tree", "-r", "-t", rev]);
    let mut tree: Vec<serde_json::Value> = listing
        .lines()
        .map(|l| {
            let (meta, path) = l.split_once('\t').unwrap();
            let mut f = meta.split(' ');
            let (mode, kind, sha) = (f.next().unwrap(), f.next().unwrap(), f.next().unwrap());
            serde_json::json!({"path": path, "mode": mode, "type": kind, "sha": sha})
        })
        .collect();
    tree.reverse();
    serde_json::json!({"sha": rev, "truncated": false, "tree": tree})
}

#[test]
fn same_as_the_cli() {
    // req:web.worker.review/fingerprint-from-trees@24bcb92e
    let dir = repo();
    let d = dir.path();
    git(d, &["config", "core.abbrev", "16"]);
    git(d, &["config", "diff.renames", "copies"]);
    write(
        d,
        &[
            ("moved.txt", long(20, "m").as_bytes()),
            ("bin.dat", &[0, 1, 2, 3, 255, 0]),
            ("specs/SPEC.md", b"# spec\n"),
        ],
    );
    commit(d);
    git(d, &["switch", "-qc", "spec/x"]);
    std::fs::create_dir_all(d.join("dir")).unwrap();
    std::fs::remove_file(d.join("moved.txt")).unwrap();
    write(
        d,
        &[
            ("dir/moved.txt", long(20, "moved").as_bytes()),
            ("bin.dat", &[0, 9, 9, 9, 255, 0]),
            ("latin1.txt", b"caf\xe9 cr\xe8me\n"),
        ],
    );
    commit(d);
    let base = git(d, &["merge-base", "main", "HEAD"]);
    let ours = fingerprint(
        &listing_of(&trees_api(d, base.trim())).unwrap(),
        &listing_of(&trees_api(d, "HEAD")).unwrap(),
    );
    let cli = clm_gate::fingerprint(d, "main").unwrap();
    assert_ne!(cli, "empty");
    assert_eq!(
        ours.as_deref(),
        Some(cli.as_str()),
        "what `clm fingerprint` prints"
    );
}

#[test]
fn truncated_listing_has_no_fingerprint() {
    // req:web.worker.review/fingerprint-from-trees@24bcb92e
    let dir = repo();
    let d = dir.path();
    write(d, &[("a.txt", b"a\n")]);
    commit(d);
    let whole = trees_api(d, "HEAD");
    let mut cut = whole.clone();
    cut["truncated"] = serde_json::json!(true);
    let (whole, cut) = (listing_of(&whole).unwrap(), listing_of(&cut).unwrap());
    assert_eq!(fingerprint(&whole, &whole).as_deref(), Some("empty"));
    assert_eq!(fingerprint(&cut, &whole), None);
    assert_eq!(fingerprint(&whole, &cut), None);
}
