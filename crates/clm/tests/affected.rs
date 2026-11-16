//! `clm affected`, run on git repositories.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", message]);
}

fn spec(paths: &str, depends_on: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n---\n"
    )
}

/// A repository on `main` with `clm_toml`: a root holding `Cargo.lock`, `a`, `b` on `a`,
/// `c` on `b`, `d` on nothing, each holding `crates/<id>/**` and a `src/lib.rs`, then
/// switched to a branch `work`.
fn repo(clm_toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, "clm.toml", clm_toml);
    write(
        root,
        "specs/SPEC.md",
        &spec("[\"Cargo.lock\", \"clm.toml\"]", "[]"),
    );
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[]"));
    write(root, "specs/b/SPEC.md", &spec("[\"crates/b/**\"]", "[a]"));
    write(root, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", "[b]"));
    write(root, "specs/d/SPEC.md", &spec("[\"crates/d/**\"]", "[]"));
    write(root, "Cargo.lock", "lock\n");
    for node in ["a", "b", "c", "d"] {
        write(root, &format!("crates/{node}/src/lib.rs"), "fn f() {}\n");
    }
    write(root, "tools/build.sh", "true\n");
    commit_all(root, "start");
    git(root, &["switch", "-q", "-c", "work"]);
    dir
}

fn clm_affected_with(root: &Path, base: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(["affected", "--base", base, "--root"])
        .arg(root)
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn why_prints_the_chain_through_a_dependency() {
    // req:clm/affected-why-command@2b2b52c9
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected_with(root, "main", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "crates/b/src/lib.rs owned by b\nb depends on a\na owns crates/a/src/lib.rs (changed)\n"
    );
    let out = clm_affected_with(root, "main", &["--why", "crates/a/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "crates/a/src/lib.rs (changed)\n");
}

#[test]
fn why_with_readers_exits_1_naming_both_flags() {
    // req:clm/affected-why-command@2b2b52c9
    let dir = repo("");
    let out = clm_affected_with(
        dir.path(),
        "main",
        &["--readers", "--why", "crates/a/src/lib.rs"],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(err.contains("--readers") && err.contains("--why"), "{err}");
    assert_eq!(stdout(&out), "");
}
