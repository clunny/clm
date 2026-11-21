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
    // req:check/node-size@ce36a0e0
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
    // req:check/node-size@ce36a0e0
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
    // req:check/node-size@ce36a0e0
    let dir = repo("");
    let out = clm_size(dir.path(), Some("a"));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "a\tspec 1001/12000\tcode 100/40000\n"
    );
}

#[test]
fn spec_violations_dont_stop_the_report() {
    // req:check/node-size@ce36a0e0
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
    // req:check/node-size@ce36a0e0
    let dir = repo(BUDGETS);
    let out = clm_size(dir.path(), Some("zz"));
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "specs/zz/SPEC.md: check/node-size: unknown component `zz`\n"
    );
}

#[test]
fn settings_that_cant_be_loaded_exit_1() {
    // req:check/node-size@ce36a0e0
    let dir = repo("[component_size]\nspec_tokens = 0\n");
    for node in [None, Some("zz")] {
        let out = clm_size(dir.path(), node);
        assert_eq!(out.status.code(), Some(1), "{node:?}: {out:?}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        let lines: Vec<_> = stdout.lines().collect();
        assert_eq!(lines.len(), 1, "{node:?}: {stdout}");
        assert!(
            lines[0].contains("clm.toml") && lines[0].contains("component_size"),
            "{node:?}: {stdout}"
        );
        assert!(!lines[0].contains("zz"), "{node:?}: {stdout}");
    }
}

/// The one line `clm size --root <root>` prints, after checking it exits 1 printing only it.
fn the_one_failure(root: &Path) -> String {
    let out = clm_size(root, None);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "{stdout}");
    lines[0].to_string()
}

#[test]
fn files_it_cant_list_are_reported_from_the_root() {
    // req:check/node-size@ce36a0e0
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // A directory in no git repository.
    let inside = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(!inside.status.success(), "{root:?} is in a git work tree");
    write(
        root,
        "specs/SPEC.md",
        b"---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        root,
        "specs/a/SPEC.md",
        b"---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n",
    );
    write(root, "src/a/x.rs", &[b'a'; 400]);
    let line = the_one_failure(root);
    let prefix = format!("{}: check/node-size: ", root.display());
    assert!(line.starts_with(&prefix), "{line}");
    assert!(line.len() > prefix.len(), "{line}");
}

#[cfg(unix)]
#[test]
fn a_file_it_cant_read_is_reported_from_the_root() {
    // req:check/node-size@ce36a0e0
    use std::os::unix::fs::PermissionsExt;
    let dir = repo(BUDGETS);
    let file = dir.path().join("src/a/x.rs");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&file).is_ok() {
        // Run as root, which reads any file: nothing to show.
        return;
    }
    let line = the_one_failure(dir.path());
    let prefix = format!("{}: check/node-size: ", dir.path().display());
    assert!(line.starts_with(&prefix), "{line}");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
}

/// `a`'s and `b`'s code counts, from `clm size`'s lines.
fn code_counts(root: &Path) -> (String, String) {
    let out = clm_size(root, None);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let code = |node: &str| {
        let line = stdout
            .lines()
            .find(|l| l.starts_with(&format!("{node}\t")))
            .unwrap_or_else(|| panic!("{stdout}"));
        line.split('\t').nth(2).unwrap().to_string()
    };
    (code("a"), code("b"))
}

/// `git hash-object -w <rel>` in `root`.
fn hash(root: &Path, rel: &str) -> String {
    let out = Command::new("git")
        .args(["hash-object", "-w", rel])
        .current_dir(root)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[cfg(unix)]
#[test]
fn only_tracked_regular_files_count() {
    // req:check/size-tracked-through-git@95e512b5
    use std::os::unix::ffi::OsStrExt;
    let fixture = || {
        let dir = repo(BUDGETS);
        write(
            dir.path(),
            "specs/b/SPEC.md",
            b"---\ntitle: B\nstatus: active\nowners: []\npaths: [\"src/b/**\"]\n---\n\n## Requirements\n\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n",
        );
        dir
    };
    let cases: [(&str, &dyn Fn(&Path)); 7] = [
        ("a tracked link", &|root| {
            write(root, "big.txt", &[b'b'; 4000]);
            std::os::unix::fs::symlink("../../big.txt", root.join("src/a/link")).unwrap();
            git(root, &["add", "big.txt", "src/a/link"]);
        }),
        ("a tracked path that is a directory", &|root| {
            write(root, "src/a/sub", &[b's'; 4000]);
            git(root, &["add", "src/a/sub"]);
            fs::remove_file(root.join("src/a/sub")).unwrap();
            fs::create_dir(root.join("src/a/sub")).unwrap();
        }),
        ("a tracked file deleted", &|root| {
            write(root, "src/a/deleted.rs", &[b'd'; 4000]);
            git(root, &["add", "src/a/deleted.rs"]);
            fs::remove_file(root.join("src/a/deleted.rs")).unwrap();
        }),
        ("a file in conflict", &|root| {
            let sha = hash(root, "src/a/x.rs");
            git(root, &["rm", "-q", "--cached", "src/a/x.rs"]);
            let info: String = (1..=3)
                .map(|stage| format!("100644 {sha} {stage}\tsrc/a/x.rs\n"))
                .collect();
            let mut child = Command::new("git")
                .args(["update-index", "--index-info"])
                .current_dir(root)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            std::io::Write::write_all(child.stdin.as_mut().unwrap(), info.as_bytes()).unwrap();
            drop(child.stdin.take());
            assert!(child.wait().unwrap().success());
        }),
        ("an untracked file citing b", &|root| {
            let mut test = concat!("// req", ":b/x\n#[test]\nfn t() {}\n")
                .as_bytes()
                .to_vec();
            test.resize(800, b't');
            write(root, "src/a/t.rs", &test);
        }),
        ("a tracked file whose directory became a file", &|root| {
            write(root, "src/a/d/y.rs", &[b'y'; 4000]);
            git(root, &["add", "src/a/d/y.rs"]);
            fs::remove_dir_all(root.join("src/a/d")).unwrap();
            write(root, "src/a/d", b"now a file\n");
        }),
        ("a name that isn't UTF-8", &|root| {
            git(root, &["rm", "-qf", "src/a/x.rs"]);
            let name = std::ffi::OsStr::from_bytes(b"src/a/\xe9.rs");
            // Removing the only file took its directory with it.
            fs::create_dir_all(root.join("src/a")).unwrap();
            fs::write(root.join(name), [b'a'; 400]).unwrap();
            let status = Command::new("git")
                .arg("add")
                .arg(name)
                .current_dir(root)
                .status()
                .unwrap();
            assert!(status.success());
        }),
    ];
    for (case, setup) in cases {
        let dir = fixture();
        setup(dir.path());
        let (a, b) = code_counts(dir.path());
        assert_eq!(a, "code 100/1000", "{case}");
        assert_eq!(b, "code 0/1000", "{case}");
    }
}

#[test]
fn a_test_counts_toward_the_node_it_cites() {
    // req:check/node-size@ce36a0e0
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
