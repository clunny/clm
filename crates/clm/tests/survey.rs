//! `clm survey` as the binary runs it: from a subdirectory, and its `--components` flag.
//! The rest of its tests are `init`'s, in `crates/clm-init/tests/survey.rs`.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A git repository holding `files`, all added to the index.
fn repo(files: &[(&str, &[u8])]) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    git(dir.path(), &["add", "-A"]);
    dir
}

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

fn survey(dir: &Path) -> String {
    let out = clm(dir, &["survey"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

const TWO_CRATES: &[(&str, &[u8])] = &[
    (
        "Cargo.toml",
        b"[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
    ),
    (
        "crates/a/Cargo.toml",
        b"[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    ),
    (
        "crates/a/src/lib.rs",
        b"mod parse;\nuse crate::util::helper;\n\npub fn a() -> u32 {\n    helper() + b::b()\n}\n",
    ),
    ("crates/a/src/parse.rs", b"pub fn parse() {}\n"),
    ("crates/a/src/util/mod.rs", b"pub fn helper() -> u32 {\n    1\n}\n"),
    ("crates/a/tests/a.rs", b"#[test]\nfn t() {}\n"),
    (
        "crates/b/Cargo.toml",
        b"[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    ),
    ("crates/b/src/lib.rs", b"pub fn b() -> u32 {\n    2\n}\n"),
    ("README.md", b"# Two crates\n"),
    ("docs/decisions/use-rust.md", b"# Use Rust\n"),
    ("docs/decisions/two-crates.md", b"# Two crates\n"),
];

#[test]
fn from_a_subdirectory() {
    // req:clm/root-from-cwd@839330d9
    // Before `clm init` there's no CLM root; the survey still maps the whole repository.
    let repo = repo(TWO_CRATES);
    let out = clm(&repo.path().join("crates/a"), &["survey"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), survey(repo.path()));
}

#[test]
fn components_flag_checks_the_map() {
    // req:clm/component-flags@4d6d18e7
    let repo = repo(TWO_CRATES);
    fs::write(repo.path().join("nodes.yaml"), "components: []\n").unwrap();
    let new = clm(repo.path(), &["survey", "--components", "nodes.yaml"]);
    assert_eq!(new.status.code(), Some(1), "{new:?}");
    let help = String::from_utf8(clm(repo.path(), &["survey", "--help"]).stdout).unwrap();
    assert!(help.contains("--components"), "{help}");
}
