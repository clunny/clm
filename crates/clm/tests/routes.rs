//! `crates/clm` only routes a command a crate offers as a call: the process prints what the
//! call writes, exits with what it returns, and passes it the settings `clm.toml` holds
//! (`routes-only`).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// `clm` with a user config dir holding no `config.toml` and no CLM variables, so the
/// developer's own settings never leak in.
fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .env_remove("CLM_AUTOSYNC")
        .env_remove("CLM_DEFECTS")
        .env_remove("CLM_MAX_BUILDS")
        .env_remove("CLM_WRITER")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

/// What a crate's call wrote and returned, given two buffers.
struct Called {
    out: String,
    err: String,
    code: u8,
}

fn called(call: impl FnOnce(&mut Vec<u8>, &mut Vec<u8>) -> u8) -> Called {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = call(&mut out, &mut err);
    Called {
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
        code,
    }
}

/// The process printed exactly what the call wrote, on the same streams, and exited with what
/// it returned.
fn assert_same(process: &Output, call: &Called) {
    assert_eq!(String::from_utf8_lossy(&process.stdout), call.out, "stdout");
    assert_eq!(String::from_utf8_lossy(&process.stderr), call.err, "stderr");
    assert_eq!(process.status.code(), Some(i32::from(call.code)), "status");
}

fn no_env(_: &str) -> Option<String> {
    None
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

const ROOT_SPEC: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

/// Node `a`'s spec, with `frontmatter` added and requirements `x` and `y`.
fn node_a(frontmatter: &str) -> String {
    format!(
        "---\ntitle: A\nstatus: active\nowners: []\n{frontmatter}---\n\n## Requirements\n\n\
         ### Requirement: x\nX.\n\n#### Scenario: s\n- s\n\n\
         ### Requirement: y\nY.\n\n#### Scenario: s\n- s\n"
    )
}

/// A git repository on `main` with the root's spec and node `a`, committed.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, "specs/SPEC.md", ROOT_SPEC);
    write(root, "specs/a/SPEC.md", &node_a(""));
    commit_all(root, "start");
    dir
}

#[test]
fn a_routed_command() {
    // req:clm/routes-only@4567a066
    let dir = fixture();
    let root = dir.path().to_str().unwrap();
    let run = |component: &str| {
        let process = clm(dir.path(), &["hashes", component, "--root", root]);
        let call = called(|out, err| {
            clm_apply::hashes_command(
                [component, "--root", root],
                Path::new("."),
                &no_env,
                out,
                err,
            )
        });
        assert_same(&process, &call);
        call
    };
    // The fixture shows something on each stream: `a`'s hashes, and the refusal of `nope`.
    let listed = run("a");
    assert!(listed.out.contains("requirement:x "), "{}", listed.out);
    let refused = run("nope");
    assert_ne!(refused.code, 0, "{}", refused.err);
    assert!(!refused.err.is_empty());
}

#[test]
fn a_status_the_call_returns() {
    // req:clm/routes-only@4567a066
    let dir = fixture();
    let marker = concat!("req", ":a/gone@00000000");
    write(dir.path(), "tests/t.rs", &format!("// {marker}\n"));
    commit_all(dir.path(), "a dangling marker");
    let root = dir.path().to_str().unwrap();
    let process = clm(dir.path(), &["check", "--root", root]);
    let inputs = clm_check::CheckInputs {
        detectors: &|_| Ok(Vec::new()),
        more: &|_| Vec::new(),
    };
    let call = called(|out, err| {
        clm_check::check_command(["--root", root], Path::new("."), &no_env, out, err, &inputs)
    });
    assert_ne!(call.code, 0, "the fixture's check fails: {}", call.out);
    assert_same(&process, &call);
}

/// A repository whose root owns `Cargo.lock` and `clm.toml`, with `clm_toml` committed on
/// `main`, switched to a branch that changes only `Cargo.lock`.
fn affected_fixture(clm_toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(
        root,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\npaths: [\"Cargo.lock\", \"clm.toml\"]\n---\n",
    );
    write(
        root,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\n---\n",
    );
    write(root, "crates/a/src/lib.rs", "fn f() {}\n");
    write(root, "clm.toml", clm_toml);
    write(root, "Cargo.lock", "lock\n");
    commit_all(root, "start");
    git(root, &["switch", "-q", "-c", "work"]);
    write(root, "Cargo.lock", "lock 2\n");
    commit_all(root, "lock");
    dir
}

#[test]
fn the_settings_a_routed_command_is_passed() {
    // req:clm/routes-only@4567a066
    let set = affected_fixture("[affected]\nall = [\"Cargo.lock\"]\n");
    let out = clm(set.path(), &["affected", "--base", "main"]);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "all Cargo.lock\n");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "", "{out:?}");
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    let invalid = affected_fixture("[affected]\nall = 3\n");
    let out = clm(invalid.path(), &["affected", "--base", "main"]);
    assert_ne!(out.status.code(), Some(0), "{out:?}");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let line = printed
        .lines()
        .find(|l| l.contains("check.affected/affected-all"))
        .unwrap_or_else(|| panic!("no violation line: {printed}"));
    // The violation as `clm check` prints one, `<path>: <rule>: <message>`, the path
    // `clm.toml`'s, and the message naming the setting.
    let (_, violation) = line
        .split_once("clm.toml: check.affected/affected-all: ")
        .unwrap_or_else(|| panic!("not a violation of clm.toml: {line}"));
    assert!(violation.contains("all"), "{line}");
}

#[test]
fn the_budgets_a_routed_command_is_passed() {
    // req:clm/routes-only@4567a066
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT_SPEC);
    write(d, "specs/a/SPEC.md", &node_a("paths: [\"src/a/**\"]\n"));
    write(
        d,
        "specs/b/SPEC.md",
        "---\ntitle: B\nstatus: active\nowners: []\n---\n",
    );
    write(d, "clm.toml", "[component_size]\ncode_tokens = 100\n");
    // 800 characters: 200 tokens, twice the budget.
    write(d, "src/a/x.rs", &"a".repeat(800));
    git(d, &["init", "-q", "-b", "main"]);
    commit_all(d, "start");
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let out = clm(d, &["status"]);
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let row = |node: &str| {
        let section = text.split("\n\ncomponents\n").nth(1).unwrap_or_default();
        section
            .lines()
            .find(|l| l.starts_with(&format!("  {node} ")))
            .unwrap_or_else(|| panic!("no {node} row in {text}"))
            .to_string()
    };
    assert!(
        row("a").split("  ").any(|cell| cell.trim() == "oversized"),
        "{text}"
    );
    assert!(!row("b").contains("oversized"), "{text}");
}

/// The command names `clm --help` lists under `Commands:`.
fn listed_commands(help: &str) -> Vec<String> {
    help.split("Commands:\n")
        .nth(1)
        .expect("a Commands section")
        .lines()
        .take_while(|l| !l.trim().is_empty())
        .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

#[test]
fn every_command_routes() {
    // req:clm/routes-only@4567a066
    let dir = TempDir::new().unwrap();
    let help = clm(dir.path(), &["--help"]);
    assert_eq!(help.status.code(), Some(0), "{help:?}");
    let commands = listed_commands(&String::from_utf8_lossy(&help.stdout));
    assert!(commands.iter().any(|c| c == "hashes"), "{commands:?}");
    // `help` is the parser's own, which takes command names, not options.
    for name in commands.iter().filter(|name| *name != "help") {
        let out = clm(dir.path(), &[name, "--help"]);
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "clm {name} --help: {out:?}");
        assert!(
            text.contains(&format!("Usage: clm {name}")),
            "clm {name} --help: {text}"
        );
    }
}
