use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// `clm size [<node>]` on `root`, with no user settings.
fn clm_size(root: &Path, node: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_clm"));
    cmd.env(
        "XDG_CONFIG_HOME",
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
    )
    .arg("size")
    .args(node)
    .arg("--root")
    .arg(root);
    cmd.output().unwrap()
}

fn write(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// A git repo with `clm.toml` holding `clm_toml`, an empty root node, and nodes `b` and
/// `a`; `a` has paths `["src/a/**"]`, a `SPEC.md` of 4001 bytes and a tracked `src/a/x.rs`
/// of 400 bytes.
fn repo(clm_toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    write(root, "clm.toml", clm_toml.as_bytes());
    write(
        root,
        "specs/SPEC.md",
        b"---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        root,
        "specs/b/SPEC.md",
        b"---\ntitle: B\nstatus: active\nowners: []\n---\n",
    );
    let mut a = b"---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n".to_vec();
    a.resize(4000, b'x');
    a.push(b'\n');
    write(root, "specs/a/SPEC.md", &a);
    write(root, "src/a/x.rs", &[b'a'; 400]);
    git(root, &["add", "src/a/x.rs"]);
    dir
}

const BUDGETS: &str = "[component_size]\nspec_tokens = 1000\ncode_tokens = 1000\n";

#[test]
fn prints_the_node_with_the_repository_budgets() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(BUDGETS);
    let out = clm_size(dir.path(), Some("a"));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "a\tspec 1001/1000\tcode 100/1000\toversized\n"
    );
}

#[test]
fn every_node_sorted_by_id() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(BUDGETS);
    let out = clm_size(dir.path(), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let nodes: Vec<_> = stdout
        .lines()
        .map(|l| l.split('\t').next().unwrap())
        .collect();
    assert_eq!(nodes, ["a", "b", "clm"], "{stdout}");
}

#[test]
fn defaults_from_config_when_unset() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("");
    let out = clm_size(dir.path(), Some("a"));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "a\tspec 1001/12000\tcode 100/40000\n"
    );
}

#[test]
fn spec_violations_dont_stop_the_report() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(BUDGETS);
    write(
        dir.path(),
        "specs/b/SPEC.md",
        b"---\ntitle: B\nstatus: active\nowners: []\ndepends_on: [nowhere]\n---\n",
    );
    let out = clm_size(dir.path(), Some("a"));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "a\tspec 1001/1000\tcode 100/1000\toversized\n"
    );
}

#[test]
fn unknown_node_exits_1() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(BUDGETS);
    let out = clm_size(dir.path(), Some("zz"));
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("zz"), "{text}");
}

#[test]
fn settings_that_cant_be_loaded_exit_1() {
    // req:check/node-size@7fb9fb0c
    let dir = repo("[component_size]\nspec_tokens = 0\n");
    let out = clm_size(dir.path(), None);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("clm.toml") && text.contains("component_size"),
        "{text}"
    );
}

#[test]
fn a_test_counts_toward_the_node_it_cites() {
    // req:check/node-size@7fb9fb0c
    let dir = repo(BUDGETS);
    let mut test = concat!("// req", ":b/x\n").as_bytes().to_vec();
    test.resize(800, b't');
    write(dir.path(), "src/a/tests/b.rs", &test);
    git(dir.path(), &["add", "src/a/tests/b.rs"]);
    let out = clm_size(dir.path(), None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("a\tspec 1001/1000\tcode 100/1000\t"),
        "{stdout}"
    );
    let b = stdout.lines().find(|l| l.starts_with("b\t")).unwrap();
    assert!(b.ends_with("\tcode 200/1000"), "{stdout}");
}
