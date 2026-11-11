//! `keys::code_key` is the ledger's (`code-key-doc-only`): the doc-only
//! paths come from `clm_ledger::doc_only`, and the key equals the one `clm land` records.

use std::path::Path;
use std::process::Command;

use clm_web_platform::keys::{TreeLine, code_key, listing_of};

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

/// A `blob` entry at `path`, with a made-up mode and sha.
fn line(path: &str) -> TreeLine<'_> {
    TreeLine {
        mode: "100644",
        kind: "blob",
        sha: "0123456789abcdef0123456789abcdef01234567",
        path,
    }
}

#[test]
fn doc_only_paths_come_from_the_ledger() {
    // req:web.worker.platform/code-key-doc-only@609e1b0d
    let all = [
        "docs/x.md",
        "docs/guide/intro.md",
        "backlog/y.md",
        "CLAUDE.md",
        "web/README.md",
        "crates/a/src/lib.rs",
        "docsy/z.md",
    ];
    let kept: Vec<&str> = all
        .iter()
        .copied()
        .filter(|p| !clm_ledger::doc_only(p))
        .collect();
    assert_eq!(kept, ["web/README.md", "crates/a/src/lib.rs", "docsy/z.md"]);
    let key = code_key(all.iter().map(|p| line(p)));
    assert_eq!(key, code_key(kept.iter().map(|p| line(p))));
    for gone in &kept {
        let fewer = kept.iter().filter(|p| *p != gone).map(|p| line(p));
        assert_ne!(key, code_key(fewer), "{gone} counts toward the key");
    }
}

#[test]
fn the_guides_files_are_doc_only() {
    // req:web.worker.platform/code-key-doc-only@609e1b0d
    let dir = repo();
    let d = dir.path();
    write(
        d,
        &[
            ("src/lib.rs", b"fn a() {}\n"),
            ("docs/x.md", b"doc\n"),
            ("docs/guide/start.md", b"guide\n"),
            ("docs/spec-format.md", b"format\n"),
            ("crates/a/src/lib.rs", b"fn e() {}\n"),
        ],
    );
    commit(d);
    let mut keys = Vec::new();
    for (i, change) in [
        None,
        Some("docs/x.md"),
        Some("docs/guide/start.md"),
        Some("docs/spec-format.md"),
        Some("crates/a/src/lib.rs"),
    ]
    .into_iter()
    .enumerate()
    {
        // Each commit changes only its own file, from `a`'s tree.
        git(d, &["switch", "-qC", &format!("c{i}"), "main"]);
        if let Some(path) = change {
            write(d, &[(path, b"changed\n")]);
            commit(d);
        }
        let ours = listing_of(&trees_api(d, "HEAD"))
            .unwrap()
            .code_key()
            .unwrap();
        assert_eq!(
            ours,
            clm_ledger::code_key(d, "HEAD").unwrap(),
            "{change:?}: the key `clm land` records"
        );
        keys.push(ours);
    }
    assert_eq!(keys[0], keys[1], "docs/x.md is doc-only");
    assert_eq!(keys[0], keys[2], "docs/guide/ is doc-only");
    assert_eq!(keys[0], keys[3], "docs/spec-format.md is doc-only");
    assert_ne!(keys[0], keys[4], "crates/a/src/lib.rs is code");
}
