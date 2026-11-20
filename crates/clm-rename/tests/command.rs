//! `clm rename` as `clm-rename`'s call: what it writes to its streams and the status it
//! returns (`commands-in-crate`).

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// What one call wrote to each stream, and the status it returned.
struct Output {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// `clm rename` with the rest of `args`, as the command line gives them after the command's
/// name, in `root`.
fn call(root: &Path, args: &[&str]) -> Output {
    let (name, rest) = args.split_first().unwrap();
    assert_eq!(*name, "rename", "clm-rename offers no `{name}`");
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_rename::rename_command(rest.iter(), root, &|_| None, &mut stdout, &mut stderr);
    Output {
        status: Some(i32::from(status)),
        stdout,
        stderr,
    }
}

/// The command `args` names with `--root <root>`, the root the call is given being a directory
/// that holds no repository, so only `--root` can find one.
fn clm(root: &Path, args: &[&str]) -> Output {
    let root = root.to_str().unwrap();
    let mut line = args.to_vec();
    line.extend(["--root", root]);
    call(Path::new("/nonexistent-clm-root"), &line)
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

fn repo(change: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "notes.txt", "see a/x\n");
    write(d, "changes/r/change.md", change);
    let git = |args: &[&str]| {
        let ok = Command::new("git").current_dir(d).args(args).status();
        assert!(ok.unwrap().success());
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    dir
}

#[test]
// req:rename/finish@90976f4b
fn the_command_prints_what_it_did_and_exits_zero() {
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nr\n",
    );
    let out = clm(dir.path(), &["rename", "r"]);
    assert_eq!(out.status, Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "renamed a/x -> a/y\nstill names a/x notes.txt:1\n"
    );
    assert!(dir.path().join("changes/archive/r/change.md").exists());
}

#[test]
// req:rename/refusals@d7194f56
fn a_refusal_exits_one_and_writes_nothing() {
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/none\n    to: y\n---\n\n## Intent\nr\n",
    );
    let out = clm(dir.path(), &["rename", "r"]);
    assert_eq!(out.status, Some(1));
    assert!(String::from_utf8(out.stderr).unwrap().contains("a/none"));
    assert!(dir.path().join("changes/r/change.md").exists());
    assert!(!dir.path().join("changes/archive").exists());
}

#[test]
fn a_call_writes_what_the_command_prints_and_returns_its_status() {
    // req:rename/commands-in-crate@8b0741b4
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nr\n",
    );
    // Found from the root the call is given, with no `--root`.
    let out = call(dir.path(), &["rename", "r"]);
    assert_eq!(out.status, Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "renamed a/x -> a/y\nstill names a/x notes.txt:1\n"
    );
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "");
    assert!(dir.path().join("changes/archive/r/change.md").exists());

    // A refusal goes to the error stream alone, one line, with status 1.
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/none\n    to: y\n---\n\n## Intent\nr\n",
    );
    let out = call(dir.path(), &["rename", "r"]);
    assert_eq!(out.status, Some(1));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("a/none"), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.ends_with('\n'), "{stderr}");
}

#[test]
fn arguments_it_cant_parse_return_two_with_the_usage_error() {
    // req:rename/commands-in-crate@8b0741b4
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nr\n",
    );
    let out = call(dir.path(), &["rename", "r", "--no-such-option"]);
    assert_eq!(out.status, Some(2));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--no-such-option"), "{stderr}");
    assert!(stderr.contains("Usage: clm rename"), "{stderr}");
    // Nothing was renamed, and this process goes on.
    assert!(dir.path().join("changes/r/change.md").exists());

    // `--help` is no error: the help goes to the output stream with status 0.
    let out = call(dir.path(), &["rename", "--help"]);
    assert_eq!(out.status, Some(0));
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "");
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("Usage: clm rename")
    );
}
