//! `clm size` through `clm-check`'s call (`commands-in-crate`), with the budgets `crates/clm`
//! would load from `config` given as an argument.

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_check::{Budgets, Violation, size_command};
use tempfile::TempDir;

/// The exit status a call returned, read as a process's.
#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }
}

/// What a call wrote to its two streams, and what it returned.
#[derive(Debug)]
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// `[component_size]` of `root`'s `clm.toml`, as `crates/clm` passes it: 12000 spec and 40000
/// code tokens where it sets none, and why it can't be loaded for a budget below 1.
fn repository_budgets(root: &Path) -> Result<Budgets, Violation> {
    let path = root.join("clm.toml");
    let table: toml::Table = fs::read_to_string(&path)
        .unwrap_or_default()
        .parse()
        .unwrap();
    let size = table.get("component_size");
    let budget = |key: &str, default: u64| match size.and_then(|s| s.get(key)) {
        None => Ok(default),
        Some(v) => match v.as_integer().unwrap() {
            n if n >= 1 => Ok(n as u64),
            n => Err(Violation {
                path: path.clone(),
                rule: "check/node-size",
                message: format!("`component_size`: `{key}` must be a positive integer, not {n}"),
            }),
        },
    };
    Ok(Budgets {
        spec_tokens: budget("spec_tokens", 12000)?,
        code_tokens: budget("code_tokens", 40000)?,
    })
}

/// `clm size [<node>]` on `root`, with the budgets its `clm.toml` gives.
fn clm_size(root: &Path, node: Option<&str>) -> Output {
    let mut args: Vec<std::ffi::OsString> = node.into_iter().map(Into::into).collect();
    args.push("--root".into());
    args.push(root.into());
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = size_command(
        &args,
        Path::new("."),
        &|_| None,
        &mut stdout,
        &mut stderr,
        &repository_budgets,
    );
    Output {
        status: Status(code),
        stdout,
        stderr,
    }
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
    // req:check/node-size@a70a5200
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
    // req:check/node-size@a70a5200
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
    // req:check/node-size@a70a5200
    let dir = repo("");
    let out = clm_size(dir.path(), Some("a"));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "a\tspec 1001/12000\tcode 100/40000\n"
    );
}

#[test]
fn spec_violations_dont_stop_the_report() {
    // req:check/node-size@a70a5200
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
    // req:check/node-size@a70a5200
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
    // req:check/node-size@a70a5200
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
    // req:check/node-size@a70a5200
    let dir = repo(BUDGETS);
    let mut test = concat!("// req", ":b/x\n#[test]\nfn t() {}\n")
        .as_bytes()
        .to_vec();
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
