//! `crates/clm` only routes a command a crate offers as a call: the process prints what the
//! call writes, exits with what it returns, and passes it the settings `clm.toml` holds
//! (`routes-only`).

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// `clm` with a user config dir holding no `config.toml` and no CLM variables, so the
/// developer's own settings never leak in.
fn clm_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clm"));
    command
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .env_remove("CLM_AUTOSYNC")
        .env_remove("CLM_DEFECTS")
        .env_remove("CLM_MAX_BUILDS")
        .env_remove("CLM_WRITER");
    command
}

fn clm(dir: &Path, args: &[&str]) -> Output {
    clm_command().args(args).current_dir(dir).output().unwrap()
}

/// `clm` run as [`clm`] is, with `stdin` on its standard input.
fn clm_with_stdin(dir: &Path, args: &[&str], stdin: &[u8]) -> Output {
    let mut child = clm_command()
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A process that exits before reading its input shows why in its output, not here.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().unwrap()
}

/// `clm`, set up as [`clm_command`] sets it, run by a shell that enters `gone` and removes it
/// first, so `clm` starts in a directory that no longer exists; its arguments follow.
fn clm_in_removed_dir(gone: &Path) -> Command {
    let command = clm_command();
    let mut sh = Command::new("sh");
    sh.arg("-c")
        .arg("cd \"$1\" && rmdir \"$1\" && shift && exec \"$@\"")
        .arg("sh")
        .arg(gone)
        .arg(env!("CARGO_BIN_EXE_clm"))
        .current_dir(gone.parent().unwrap());
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => sh.env(key, value),
            None => sh.env_remove(key),
        };
    }
    sh
}

fn rev_parse(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
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
    // req:clm/routes-only@0958d204
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
    // req:clm/routes-only@0958d204
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
    // req:clm/routes-only@0958d204
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
        .find(|l| l.contains("check.affected/affected-base"))
        .unwrap_or_else(|| panic!("no violation line: {printed}"));
    // The violation as `clm check` prints one, `<path>: <rule>: <message>`, the path
    // `clm.toml`'s, and the message naming the setting.
    let (_, violation) = line
        .split_once("clm.toml: check.affected/affected-base: ")
        .unwrap_or_else(|| panic!("not a violation of clm.toml: {line}"));
    assert!(violation.contains("all"), "{line}");
    assert!(!printed.contains("affected-all"), "{printed}");
}

#[test]
fn the_budgets_a_routed_command_is_passed() {
    // req:clm/routes-only@0958d204
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

#[test]
fn a_report_that_cant_be_read() {
    // req:clm/routes-only@0958d204
    let dir = fixture();
    write(dir.path(), "clm.toml", "defects = \"backlog\"\n");
    commit_all(dir.path(), "defects to the backlog");
    let head = rev_parse(dir.path());
    // Bytes that aren't UTF-8.
    let out = clm_with_stdin(
        dir.path(),
        &["defect", "--title", "t"],
        b"a report \xff\xfe\n",
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "", "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(err.starts_with("clm defect: reading the report:"), "{err}");
    // Files nothing: no backlog item, no commit, and the tree is as committed.
    assert!(!dir.path().join("backlog").exists(), "{err}");
    assert_eq!(rev_parse(dir.path()), head);
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&status.stdout), "");
}

#[test]
fn a_current_directory_that_cant_be_read() {
    // req:clm/routes-only@0958d204
    for (command, args) in [("defect", &["--title", "t"][..]), ("config", &["list"][..])] {
        let dir = TempDir::new().unwrap();
        let gone = dir.path().join("gone");
        fs::create_dir(&gone).unwrap();
        // The shell enters the directory and removes it, so `clm` starts in a directory that
        // no longer exists.
        let out = clm_in_removed_dir(&gone)
            .arg(command)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "clm {command}: {out:?}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "", "clm {command}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(err.lines().count(), 1, "clm {command}: {err}");
        assert!(
            err.starts_with(&format!("clm {command}: current directory: ")),
            "clm {command}: {err}"
        );
    }
}

#[test]
fn a_hook_whose_event_cant_be_read() {
    // req:clm/routes-only@0958d204
    // An event that can't be read is an empty one: the hook still runs and still exits as it
    // does for an empty event, so it never breaks a session.
    let dir = fixture();
    for hook in ["guard", "session-start", "autosync"] {
        let unread = clm_with_stdin(dir.path(), &["hook", hook], b"{\"cwd\": \"\xff\"}");
        let empty = clm_with_stdin(dir.path(), &["hook", hook], b"");
        assert_eq!(unread.status.code(), Some(0), "clm hook {hook}: {unread:?}");
        assert_eq!(unread.stdout, empty.stdout, "clm hook {hook}");
        assert_eq!(unread.stderr, empty.stderr, "clm hook {hook}");
    }
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
    // req:clm/routes-only@0958d204
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
