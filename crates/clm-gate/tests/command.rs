//! `clm gate` as `clm-gate`'s call: what it writes to each stream and the status it returns
//! (`commands-in-crate`).

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// What one call of `clm gate` wrote to each stream, and the status it returned.
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// The status a call returned, read as a process's exit status is.
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(i32::from(self.0))
    }
}

/// Calls `clm gate` with `args`, the root found from the current directory being `root`.
fn call(root: &Path, args: &[&OsStr]) -> Output {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_gate::command(
        args.iter().copied(),
        root,
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    Output {
        status: Status(status),
        stdout,
        stderr,
    }
}

/// `clm gate` with `args` and `--root <root>`, the root it is given being a directory that
/// holds no repository, so only `--root` can find one; its status and standard output.
fn clm(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let mut line: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    line.push("--root".as_ref());
    line.push(root.as_os_str());
    let out = call(Path::new("/nonexistent-clm-root"), &line);
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(d: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(d)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

/// A repository whose `main` holds `README.md` and whose `HEAD`, on branch `pr`, adds
/// `specs/a/SPEC.md` and `src/lib.rs`: a mixed branch.
fn mixed_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "README.md", "x\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "base"]);
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "specs/a/SPEC.md", "x\n");
    write(d, "src/lib.rs", "x\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "mixed"]);
    dir
}

#[test]
fn gate_prints_kind_and_blocks_mixed_pr() {
    // req:gate/classify@3c5847d8
    let dir = mixed_repo();
    let (code, stdout) = clm(dir.path(), &["--base", "main", "--approved"]);
    assert_eq!(code, Some(1), "{stdout}");
    assert_eq!(stdout.lines().next(), Some("mixed"));
    assert!(stdout.contains("gate/classify"), "{stdout}");
}

#[test]
fn first_line() {
    // req:gate/approval@492f6a25
    // req:gate/needs-review@48a7ac76
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "backlog/x.md", "---\ntitle: X\nkind: idea\n---\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "base"]);
    git(d, &["checkout", "-q", "-b", "pr"]);
    write(d, "backlog/x.md", "---\ntitle: X2\nkind: idea\n---\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "edit item"]);

    let (code, stdout) = clm(d, &["--base", "main"]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "spec\n");

    write(d, "ARCHITECTURE.md", "x\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "invariants"]);
    let (code, stdout) = clm(d, &["--base", "main"]);
    assert_eq!(code, Some(1), "{stdout}");
    assert_eq!(stdout.lines().next(), Some("spec review"));
    let violations: Vec<&str> = stdout.lines().skip(1).collect();
    assert_eq!(violations.len(), 1, "{stdout}");
    assert!(
        violations[0].contains("gate/approval") && violations[0].contains("`clm approve <branch>`"),
        "{stdout}"
    );
    let (code, stdout) = clm(d, &["--base", "main", "--approved"]);
    assert_eq!((code, stdout.as_str()), (Some(0), "spec review\n"));
}

#[test]
fn the_call_writes_what_clm_gate_prints() {
    // req:gate/commands-in-crate@66389c3a
    let dir = mixed_repo();
    let d = dir.path();
    let outcome = clm_gate::evaluate_at(d, "main", "HEAD", false).unwrap();
    let mut expected = format!("{}\n", outcome.kind);
    for v in &outcome.violations {
        expected.push_str(&format!("{v}\n"));
    }
    assert!(!outcome.violations.is_empty());

    // The root the call is given is the repository, with no `--root`.
    let out = call(d, &["--base", "main"].map(OsStr::new));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), expected);
    assert!(out.stderr.is_empty());

    // A clean branch: the kind alone, and 0.
    git(d, &["checkout", "-q", "main"]);
    let out = call(d, &["--base", "main"].map(OsStr::new));
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "code\n");
    assert!(out.stderr.is_empty());

    // `--root` overrides the root the call is given.
    let out = call(
        Path::new("/nonexistent-clm-root"),
        &[
            OsStr::new("--base"),
            OsStr::new("main"),
            OsStr::new("--root"),
            d.as_os_str(),
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "code\n");

    // A base git can't resolve: the error on the error stream, and 2.
    let out = call(d, &["--base", "no-such-ref"].map(OsStr::new));
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.starts_with("clm gate: "), "{err}");
}

#[test]
fn arguments_it_cant_parse_return_2() {
    // req:gate/commands-in-crate@66389c3a
    let dir = mixed_repo();
    let out = call(
        dir.path(),
        &["--base", "main", "--no-such-option"].map(OsStr::new),
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("--no-such-option"), "{err}");
    assert!(err.contains("Usage: clm gate"), "{err}");
}
