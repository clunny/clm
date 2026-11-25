//! `keys::code_key` is the ledger's (`code-key-doc-only`): the doc-only
//! paths come from `clm_ledger::doc_only`, and the key equals the one `clm land` records;
//! and `keys::fingerprint` is what `clm fingerprint` prints (`fingerprint-from-listings`).

use std::path::Path;
use std::process::Command;

use clm_web_platform::keys::{TreeLine, code_key, fingerprint, listing_of};

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
    // req:web.worker.platform/code-key-doc-only@c6fcc964
    let all = [
        "docs/x.md",
        "docs/guide/intro.md",
        "backlog/y.md",
        "CLAUDE.md",
        "AGENTS.md",
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
    // req:web.worker.platform/code-key-doc-only@c6fcc964
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
    // A commit whose order `.`, `-` and `/` decide: `src.rs` before `src/`, and
    // `src/a-b.rs` before `src/a.rs`.
    git(d, &["switch", "-qc", "order", "main"]);
    write(
        d,
        &[
            ("src.rs", b"s\n"),
            ("src/a.rs", b"a\n"),
            ("src/a-b.rs", b"ab\n"),
        ],
    );
    commit(d);
    let ours = listing_of(&trees_api(d, "HEAD"))
        .unwrap()
        .code_key()
        .unwrap();
    assert_eq!(
        ours,
        clm_ledger::code_key(d, "HEAD").unwrap(),
        "in git's order"
    );
}

#[test]
fn a_truncated_listing_has_no_code_key() {
    // req:web.worker.platform/code-key-doc-only@c6fcc964
    let dir = repo();
    let d = dir.path();
    write(
        d,
        &[("src/lib.rs", b"fn a() {}\n"), ("docs/x.md", b"doc\n")],
    );
    commit(d);
    let whole = trees_api(d, "HEAD");
    let mut cut = whole.clone();
    cut["truncated"] = serde_json::json!(true);
    assert_eq!(
        listing_of(&whole).unwrap().code_key(),
        Some(clm_ledger::code_key(d, "HEAD").unwrap())
    );
    assert_eq!(listing_of(&cut).unwrap().code_key(), None);
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

#[test]
fn same_as_the_cli() {
    // req:web.worker.platform/fingerprint-from-listings@a7991f74
    // The CLM root is the top of the git repository: `specs/SPEC.md` sits at its top.
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
    // req:web.worker.platform/fingerprint-from-listings@a7991f74
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
