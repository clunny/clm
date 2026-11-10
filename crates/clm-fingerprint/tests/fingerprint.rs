use std::path::{Path, PathBuf};
use std::process::Command;

use clm_fingerprint::{Entry, adds_change, fingerprint, fingerprint_of, parse_ls_tree};
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
    // req:gate.fingerprint/fingerprint-stable@d81343df
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
    // req:gate.fingerprint/fingerprint@5c898d3d
    let d = repo(&[(A, &five_lines())]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[(A, &five_lines().replace("line 1", "one"))]);
    let before = fp(d.path());
    commit(d.path(), &[(A, &five_lines().replace("line 1", "uno"))]);
    assert_ne!(fp(d.path()), before);
}

#[test]
fn main_ahead_of_the_fork_point() {
    // req:gate.fingerprint/fingerprint@5c898d3d
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
    // req:gate.fingerprint/fingerprint-stable@d81343df
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
    // req:gate.fingerprint/fingerprint-stable@d81343df
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
    // req:gate.fingerprint/fingerprint-stable@d81343df
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
    // req:gate.fingerprint/fingerprint-stable@d81343df
    let d = spec_branch();
    let before = fp(d.path());
    commit(d.path(), &[(A, "by hand\n")]);
    assert_eq!(fp(d.path()), before);
}

#[test]
fn empty_when_nothing_differs() {
    // req:gate.fingerprint/fingerprint@5c898d3d
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
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update([0]);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn listings_not_a_process() {
    // req:gate.fingerprint/fingerprint@5c898d3d
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
    // req:gate.fingerprint/fingerprint@5c898d3d
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
    // req:gate.fingerprint/fingerprint@5c898d3d
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
    // req:gate.fingerprint/fingerprint@5c898d3d
    let adds = |path: &str| adds_change(&[], &[entry("blob", "c", path)]);
    assert!(adds("changes/x/change.md"));
    assert!(adds("changes/archive/x/change.md"));
    assert!(!adds("changes/archive/change.md"));
    assert!(!adds("changes/x/deltas/change.md"));
    assert!(!adds("changes//change.md"));
}

#[test]
fn paths_are_bytes() {
    // req:gate.fingerprint/fingerprint@5c898d3d
    let listing = b"100644 blob aaa\tcaf\xe9.txt\x00040000 tree ttt\tdir\0junk\0";
    let entries = parse_ls_tree(listing);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, b"caf\xe9.txt");
    assert_ne!(fingerprint_of(&[], &entries), "empty");
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn builds_for_wasm() {
    // req:gate.fingerprint/fingerprint@5c898d3d
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
fn git_runs_through_the_layer() {
    // req:gate.fingerprint/fingerprint@5c898d3d
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(src).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains("Command::new"), "runs a process itself");
    }
}

#[test]
fn mode_change_changes_the_fingerprint() {
    // req:gate.fingerprint/fingerprint-stable@d81343df
    let d = repo(&[("run.sh", "echo\n")]);
    git(d.path(), &["switch", "-qc", "topic"]);
    commit(d.path(), &[("other.txt", "o\n")]);
    let before = fp(d.path());
    git(d.path(), &["update-index", "--chmod=+x", "run.sh"]);
    git(d.path(), &["commit", "-qm", "mode"]);
    assert_ne!(fp(d.path()), before);
}
