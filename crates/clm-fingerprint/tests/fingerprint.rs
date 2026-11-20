use std::path::{Path, PathBuf};
use std::process::Command;

use clm_fingerprint::{
    Entry, added_changes, adds_change, fingerprint, fingerprint_of, parse_ls_tree,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const ID: [&str; 4] = ["-c", "user.name=t", "-c", "user.email=t@t"];

fn git(repo: &Path, args: &[&str]) -> String {
    clm_git::git(repo, &[&ID[..], args].concat()).unwrap()
}

fn write(repo: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let path = repo.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn commit(repo: &Path, files: &[(&str, &str)]) {
    write(repo, files);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "c"]);
}

fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    commit(dir.path(), files);
    dir
}

fn fp(repo: &Path) -> String {
    fingerprint(repo, "main").unwrap()
}

const A: &str = "specs/a/SPEC.md";

fn five_lines() -> String {
    (1..=5).map(|i| format!("line {i}\n")).collect()
}

#[test]
fn pure_rebase() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = repo(&[(A, &five_lines()), ("specs/b/SPEC.md", "b\n")]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[(A, &five_lines().replace("line 1", "one"))]);
    let before = fp(d.path());
    git(d.path(), &["switch", "-q", "main"]);
    commit(d.path(), &[("specs/b/SPEC.md", "changed\n")]);
    git(d.path(), &["switch", "-q", "topic"]);
    git(d.path(), &["rebase", "-q", "main"]);
    assert_eq!(fp(d.path()), before);
}

#[test]
fn content_changed() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = repo(&[(A, &five_lines())]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[(A, &five_lines().replace("line 1", "one"))]);
    let before = fp(d.path());
    commit(d.path(), &[(A, &five_lines().replace("line 1", "uno"))]);
    assert_ne!(fp(d.path()), before);
}

#[test]
fn main_ahead_of_the_fork_point() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let d = repo(&[(A, &five_lines()), ("specs/b/SPEC.md", "b\n")]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[(A, &five_lines().replace("line 1", "one"))]);
    let before = fp(d.path());
    git(d.path(), &["switch", "-q", "main"]);
    commit(d.path(), &[("specs/b/SPEC.md", "changed\n")]);
    git(d.path(), &["switch", "-q", "topic"]);
    assert_eq!(fp(d.path()), before);
}

#[test]
fn same_file_changed_on_main() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = repo(&[(A, &five_lines())]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[(A, &five_lines().replace("line 1", "one"))]);
    let before = fp(d.path());
    git(d.path(), &["switch", "-q", "main"]);
    commit(d.path(), &[(A, &five_lines().replace("line 5", "five"))]);
    git(d.path(), &["switch", "-q", "topic"]);
    git(d.path(), &["merge", "-q", "-m", "m", "main"]);
    assert_ne!(fp(d.path()), before);
}

#[test]
fn git_config_ignored() {
    // req:gate.fingerprint/fingerprint-ignores-config@006f7b67
    let d = repo(&[
        ("old name.txt", &five_lines()),
        ("bin.dat", "x\0y"),
        ("caf\u{e9}/keep.txt", "k\n"),
    ]);
    git(d.path(), &["switch", "-qc", "topic"]);
    git(d.path(), &["mv", "old name.txt", "new name.txt"]);
    write(
        d.path(),
        &[("new name.txt", &five_lines().replace("line 3", "three"))],
    );
    std::fs::write(d.path().join("bin.dat"), b"x\0z\xff").unwrap();
    std::fs::write(d.path().join("latin1.txt"), b"caf\xe9\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-qm", "c"]);
    let plain = fp(d.path());
    for (k, v) in [
        ("core.abbrev", "16"),
        ("core.quotePath", "false"),
        ("diff.renames", "copies"),
        ("diff.algorithm", "patience"),
        ("diff.noprefix", "true"),
        ("diff.context", "8"),
    ] {
        git(d.path(), &["config", k, v]);
    }
    assert_eq!(fp(d.path()), plain);
}

const CHANGE: &str = "---\nstatus: applied\n---\n";

/// A spec branch adding change `x` to a repo whose `main` has `specs/a/SPEC.md`.
fn spec_branch() -> TempDir {
    let d = repo(&[(A, "base\n")]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(
        d.path(),
        &[
            ("changes/x/change.md", CHANGE),
            ("changes/x/deltas/a.md", "delta 1\n"),
            (A, "base\nr\n"),
        ],
    );
    d
}

#[test]
fn regenerated_onto_a_main_that_changed_the_same_file() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = spec_branch();
    let before = fp(d.path());
    git(d.path(), &["switch", "-q", "main"]);
    commit(d.path(), &[(A, "base\ns\n")]);
    git(d.path(), &["switch", "-q", "topic"]);
    git(d.path(), &["reset", "-q", "--hard", "main"]);
    commit(
        d.path(),
        &[
            ("changes/x/change.md", CHANGE),
            ("changes/x/deltas/a.md", "delta 1\n"),
            (A, "base\ns\nr\n"),
        ],
    );
    assert_eq!(fp(d.path()), before);
}

#[test]
fn delta_edited() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = spec_branch();
    let before = fp(d.path());
    commit(
        d.path(),
        &[("changes/x/deltas/a.md", "delta 2\n"), (A, "base\ns\nr2\n")],
    );
    assert_ne!(fp(d.path()), before);
}

#[test]
fn specs_edited_by_hand_on_a_spec_branch() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = spec_branch();
    let before = fp(d.path());
    commit(d.path(), &[(A, "by hand\n")]);
    assert_eq!(fp(d.path()), before);
}

#[test]
fn empty_when_nothing_differs() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let d = repo(&[(A, "x\n")]);
    git(d.path(), &["switch", "-qc", "topic"]);
    assert_eq!(fp(d.path()), "empty");
}

fn entry(kind: &str, id: &str, path: &str) -> Entry {
    let mode = if kind == "tree" { "040000" } else { "100644" };
    Entry {
        mode: mode.into(),
        kind: kind.into(),
        id: id.into(),
        path: path.as_bytes().to_vec(),
    }
}

fn hex(parts: &[&str]) -> String {
    hex_bytes(&parts.iter().map(|p| p.as_bytes()).collect::<Vec<_>>())
}

fn hex_bytes(parts: &[&[u8]]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
        h.update([0]);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn listings_not_a_process() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let base = vec![
        entry("blob", "aaa", "a.txt"),
        entry("blob", "same", "b.txt"),
    ];
    let mut head = vec![
        entry("blob", "bbb", "a.txt"),
        entry("blob", "same", "b.txt"),
    ];
    let want = hex(&["a.txt", "100644 blob aaa", "100644 blob bbb"]);
    assert_eq!(fingerprint_of(&base, &head), want);
    head.push(entry("tree", "ttt", "dir"));
    assert_eq!(fingerprint_of(&base, &head), want);
}

#[test]
fn listings_of_a_branch_that_adds_a_change() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let base = vec![entry("blob", "s1", A)];
    let head = |id: &str| {
        vec![
            entry("blob", id, A),
            entry("blob", "c", "changes/x/change.md"),
        ]
    };
    let want = hex(&["changes/x/change.md", "-", "100644 blob c"]);
    assert_eq!(fingerprint_of(&base, &head("s2")), want);
    assert_eq!(fingerprint_of(&base, &head("s3")), want);
}

#[test]
fn listings_of_an_archive_move() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let base = vec![
        entry("blob", "s1", A),
        entry("blob", "c", "changes/x/change.md"),
    ];
    let head = vec![
        entry("blob", "s2", A),
        entry("blob", "c", "changes/archive/x/change.md"),
    ];
    assert!(!adds_change(&base, &head));
    let want = hex(&[
        "changes/archive/x/change.md",
        "-",
        "100644 blob c",
        "changes/x/change.md",
        "100644 blob c",
        "-",
        A,
        "100644 blob s1",
        "100644 blob s2",
    ]);
    assert_eq!(fingerprint_of(&base, &head), want);
}

#[test]
fn adds_change_reads_one_component_ids() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let adds = |path: &str| adds_change(&[], &[entry("blob", "c", path)]);
    assert!(adds("changes/x/change.md"));
    assert!(adds("changes/archive/x/change.md"));
    assert!(!adds("changes/archive/change.md"));
    assert!(!adds("changes/x/deltas/change.md"));
    assert!(!adds("changes//change.md"));
}

#[test]
fn paths_are_bytes() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let listing = b"100644 blob aaa\tcaf\xe9.txt\x00040000 tree ttt\tdir\0junk\0";
    let entries = parse_ls_tree(listing);
    assert_eq!(entries.len(), 2);
    let first = &entries[0];
    assert_eq!(
        (first.mode.as_str(), first.kind.as_str(), first.id.as_str()),
        ("100644", "blob", "aaa")
    );
    assert_eq!(first.path, b"caf\xe9.txt");
    assert_eq!(entries[1].kind, "tree");
    let want = hex_bytes(&[b"caf\xe9.txt", b"-", b"100644 blob aaa"]);
    assert_eq!(fingerprint_of(&[], &entries), want);
}

#[test]
fn parse_ls_tree_reads_each_piece_by_its_shape() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let listing: &[u8] = b"m t\tone-space\0\xff t i\tnot-utf8\0no tab here\0\
100644 blob aaa extra\tlong\0100644 blob bbb\t\0160000 commit ccc\tlast";
    let entries = parse_ls_tree(listing);
    let read: Vec<(&str, &str, &str, &[u8])> = entries
        .iter()
        .map(|e| (e.mode.as_str(), e.kind.as_str(), e.id.as_str(), &e.path[..]))
        .collect();
    assert_eq!(
        read,
        [
            ("100644", "blob", "aaa", &b"long"[..]),
            ("100644", "blob", "bbb", &b""[..]),
            ("160000", "commit", "ccc", &b"last"[..]),
        ]
    );
    let tabbed = parse_ls_tree(b"100644 blob aaa\ta\tb\0");
    assert_eq!(tabbed[0].path, b"a\tb");
}

#[test]
fn added_changes_named() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let base = vec![entry("blob", "z", "changes/z/change.md")];
    let head = vec![
        entry("blob", "x", "changes/x/change.md"),
        entry("blob", "y", "changes/archive/y/change.md"),
        entry("blob", "z", "changes/archive/z/change.md"),
        entry("tree", "w", "changes/w/change.md"),
    ];
    let want: Vec<Vec<u8>> = vec![
        b"changes/x/change.md".to_vec(),
        b"changes/archive/y/change.md".to_vec(),
    ];
    assert_eq!(added_changes(&base, &head), want);
    let reversed: Vec<Entry> = head.iter().rev().cloned().collect();
    let mut want_reversed = want.clone();
    want_reversed.reverse();
    assert_eq!(added_changes(&base, &reversed), want_reversed);
    assert!(added_changes(&head, &head).is_empty());
}

#[test]
fn clm_root_in_a_subdirectory() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let d = repo(&[("tool/a.txt", "a\n"), ("b.txt", "b\n")]);
    let old = git(d.path(), &["rev-parse", "main:tool/a.txt"]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[("tool/a.txt", "a2\n"), ("b.txt", "b2\n")]);
    let new = git(d.path(), &["rev-parse", "HEAD:tool/a.txt"]);
    let want = hex(&[
        "a.txt",
        &format!("100644 blob {old}"),
        &format!("100644 blob {new}"),
    ]);
    let tool = d.path().join("tool");
    assert_eq!(fingerprint(&tool, "main").unwrap(), want);
    commit(d.path(), &[("b.txt", "b3\n")]);
    assert_eq!(fingerprint(&tool, "main").unwrap(), want);
}

#[test]
fn no_merge_base() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let d = repo(&[("a.txt", "a\n")]);
    git(d.path(), &["switch", "-q", "--orphan", "lonely"]);
    commit(d.path(), &[("b.txt", "b\n")]);
    git(d.path(), &["switch", "-q", "main"]);
    assert!(fingerprint(d.path(), "lonely").is_err());
    assert!(fingerprint(d.path(), "main").is_ok());
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn builds_for_wasm() {
    // req:gate.fingerprint/fingerprint@a2cadb29
    let dir = workspace().join("target/wasm-test");
    let out = Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args(["build", "-p", "clm-fingerprint", "--locked"])
        .args(["--target", "wasm32-unknown-unknown"])
        .args(["--target-dir", dir.to_str().unwrap()])
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn mode_change_changes_the_fingerprint() {
    // req:gate.fingerprint/fingerprint-stable@6857ec60
    let d = repo(&[("run.sh", "echo\n")]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[("other.txt", "o\n")]);
    let before = fp(d.path());
    git(d.path(), &["update-index", "--chmod=+x", "run.sh"]);
    git(d.path(), &["commit", "-qm", "mode"]);
    assert_ne!(fp(d.path()), before);
}
