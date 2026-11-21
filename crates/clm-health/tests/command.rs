//! `clm health` and `clm workaround` as `clm-health`'s calls, on this repository and on git
//! repositories made here (`commands-in-crate`).

// clm:reads specs/**
// clm:reads backlog/**
// clm:reads docs/**
// clm:reads changes/**
// clm:reads ARCHITECTURE.md
// clm:reads CAPABILITIES.md
// clm:reads CLAUDE.md
// clm:reads AGENTS.md

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod ledger_common;

/// What a call wrote to standard output and standard error, and the status it returned.
#[derive(Debug)]
struct Output {
    status: u8,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// `clm health --root <root>` with `extra` arguments after it, and `settings` as `crates/clm`
/// passes them.
fn call_health(
    root: &Path,
    extra: &[&str],
    settings: Result<clm_health::Settings, clm_check::Violation>,
) -> Output {
    let mut args = vec!["--root".into(), root.as_os_str().to_owned()];
    args.extend(extra.iter().map(|a| (*a).into()));
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::health_command(
        args,
        Path::new("/nonexistent"),
        settings,
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    Output {
        status,
        stdout,
        stderr,
    }
}

/// `clm health --root <root>` with `config`'s defaults.
fn clm_health(root: &Path) -> Output {
    call_health(root, &[], Ok(common::settings(12000, 40000)))
}

fn write(root: &Path, rel: &str, content: &str) {
    common::write(root, rel, content)
}

/// A git repository with an empty root node and node `a` owning `src/**`.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let out = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        dir.path(),
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/**\"]\n---\n",
    );
    dir
}

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:check.health/commands-in-crate@bb8aa7f2
    let dir = repo();
    write(dir.path(), "specs/a/SPEC.md", &common::spec("[]", &["x"]));
    let out = clm_health(dir.path());
    assert_eq!(out.status, 0, "{out:?}");
    let want = common::report(dir.path());
    assert_eq!(String::from_utf8_lossy(&out.stdout), want);
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");

    // With no `--root`, the root the caller found; the first call passed `/nonexistent` and `--root`.
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::health_command(
        std::iter::empty::<&str>(),
        dir.path(),
        Ok(common::settings(12000, 40000)),
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(status, 0);
    assert_eq!(String::from_utf8_lossy(&stdout), want);

    // A missing root spec: exit 1 on the error stream, nothing on standard output.
    let empty = TempDir::new().unwrap();
    let out = clm_health(empty.path());
    assert_eq!(out.status, 1, "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).starts_with("clm health: "),
        "{out:?}"
    );

    // `clm workaround`: the path on standard output, nothing on the error stream.
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::workaround_command(
        ["a/x", "--title", "t"],
        dir.path(),
        Ok(3),
        &mut "body\n".as_bytes(),
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        String::from_utf8_lossy(&stdout),
        "backlog/workaround-a-x-1.md\n"
    );
    assert!(stderr.is_empty());
}

#[test]
fn scenario_arguments_it_cant_parse() {
    // req:check.health/commands-in-crate@bb8aa7f2
    let dir = repo();
    let out = call_health(dir.path(), &["--bogus"], Ok(common::settings(12000, 40000)));
    assert_eq!(out.status, 2, "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--bogus") && stderr.contains("Usage: clm health"),
        "{stderr}"
    );

    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::workaround_command(
        ["a/x", "--bogus"],
        dir.path(),
        Ok(3),
        &mut "".as_bytes(),
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(status, 2);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8_lossy(&stderr);
    assert!(stderr.contains("Usage: clm workaround"), "{stderr}");
    assert!(!dir.path().join("backlog").exists(), "a flag was written");
}

#[test]
fn scenario_a_clean_repository() {
    // req:check.health/health-report@d9222477
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = clm_health(&root);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status, 0, "{out:?}");
    assert!(
        stdout.lines().any(|l| l.starts_with("health/check:")),
        "{stdout}"
    );
    assert!(
        stdout.lines().any(|l| l.starts_with("health/reconsider:")),
        "{stdout}"
    );
}

#[test]
fn scenario_findings_dont_fail_it() {
    // req:check.health/health-report@d9222477
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "src/t.rs",
        &in_test(&format!("// {}:a/ghost@00000000\n", "req")),
    );
    write(
        d,
        "clm.toml",
        "[[detectors]]\nname = \"bad\"\ncommand = [\"sh\", \"-c\", \"touch ran; exit 3\"]\n",
    );
    let out = clm_health(d);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("health/check: check/dangling-markers:"),
        "{stdout}"
    );
    assert_eq!(out.status, 0, "{out:?}");
    assert!(!d.join("ran").exists(), "a detector ran");
}

#[test]
fn scenario_settings_cant_be_loaded() {
    // req:check.health/health-report@d9222477
    let dir = repo();
    write(dir.path(), "clm.toml", "[health]\nreconsider = 0\n");
    // What `crates/clm` passes when `config` rejects that file.
    let rejected = clm_check::Violation {
        path: "clm.toml".into(),
        rule: "check.health/health-report",
        message: "`health`: `reconsider` must be at least 1".into(),
    };
    let out = call_health(dir.path(), &[], Err(rejected));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status, 1, "{out:?}");
    assert!(
        stderr.contains("clm.toml") && stderr.contains("`health`"),
        "{stderr}"
    );
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn a_missing_root_spec_exits_1_naming_it() {
    // req:check.health/health-report@d9222477
    let dir = TempDir::new().unwrap();
    let out = clm_health(dir.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status, 1, "{out:?}");
    assert!(stderr.contains("specs/SPEC.md"), "{stderr}");
}

/// `clm workaround <requirement> --title <title> --root <root>` with `body` on stdin, and the
/// `[effort] workaround_flag` `crates/clm` passes.
fn call_workaround(
    root: &Path,
    requirement: &str,
    title: &str,
    body: &str,
    workaround_flag: Result<u64, String>,
) -> Output {
    let args = [
        requirement.into(),
        "--title".into(),
        title.into(),
        "--root".into(),
        root.as_os_str().to_owned(),
    ];
    let env = ledger_common::env(root, &[]);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::workaround_command::<_, std::ffi::OsString>(
        args,
        Path::new("/nonexistent"),
        workaround_flag,
        &mut body.as_bytes(),
        &env,
        &mut stdout,
        &mut stderr,
    );
    Output {
        status,
        stdout,
        stderr,
    }
}

fn clm_workaround(root: &Path, requirement: &str, title: &str, body: &str) -> Output {
    call_workaround(root, requirement, title, body, Ok(3))
}

#[test]
fn workaround_writes_the_flag_from_stdin_and_commits_nothing() {
    // req:check.health/workaround-command@36affded
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: []\n---\n\n## Requirements\n\n\
### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n",
    );
    let body = "The scenario needs a fixed date, so the test fakes the clock.\n";
    let out = clm_workaround(d, "a/x", "Faked the clock", body);
    assert_eq!(out.status, 0, "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "backlog/workaround-a-x-1.md\n"
    );
    let text = fs::read_to_string(d.join("backlog/workaround-a-x-1.md")).unwrap();
    assert!(text.ends_with(body), "{text}");
    let log = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .current_dir(d)
        .output()
        .unwrap();
    assert_eq!(log.status.code(), Some(1), "a commit was made: {log:?}");

    let out = clm_workaround(d, "a/ghost", "x", "b\n");
    assert_eq!(out.status, 1, "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("a/ghost"));
    assert!(!d.join("backlog/workaround-a-ghost-1.md").exists());
}

/// A CLM clone with a bare `origin`: capability `p`, and node `a` whose rule `x` serves it.
fn served_repo(dir: &Path) -> std::path::PathBuf {
    let root = ledger_common::clone_with_origin(dir, "repo", "Ann");
    write(
        &root,
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n",
    );
    write(
        &root,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: p\nA person can p.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        &root,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\nserves: [capability/p]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    write(&root, "clm.toml", "host = \"git\"\n");
    root
}

/// `clm workaround a/x` in `root`, its user files and ledger writer from `root`, with
/// `config`'s default `workaround_flag`.
fn flag_x(root: &Path) -> Output {
    flag_x_with(root, Ok(3))
}

fn flag_x_with(root: &Path, workaround_flag: Result<u64, String>) -> Output {
    call_workaround(root, "a/x", "Faked it", "b\n", workaround_flag)
}

/// `clm capabilities decide p --value <value> --decision <decision>` in `root`: the lines it
/// prints.
fn decide(root: &Path, value: clm_ledger::Rating, decision: clm_ledger::Decision) -> String {
    let env = ledger_common::env(root, &[]);
    let done = clm_effort::decide(root, "p", value, decision, &env).unwrap();
    done.lines.iter().map(|l| format!("{l}\n")).collect()
}

const CROWDED: &str = "clm workaround: warning: 3 unsettled workarounds on capability/p: stop and ask whether to redesign it (redesign-before-workaround)\n";

#[test]
fn third_flag_exits_3() {
    // req:check.health/workaround-command@36affded
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    for _ in 0..2 {
        assert_eq!(flag_x(&root).status, 0);
    }
    let out = flag_x(&root);
    assert_eq!(out.status, 3, "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "backlog/workaround-a-x-3.md\n"
    );
    assert_eq!(String::from_utf8_lossy(&out.stderr), CROWDED);
    assert!(root.join("backlog/workaround-a-x-3.md").is_file());
}

#[test]
fn second_flag_exits_0() {
    // req:check.health/workaround-command@36affded
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    assert_eq!(flag_x(&root).status, 0);
    let out = flag_x(&root);
    assert_eq!(out.status, 0, "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn settled_flags_not_counted() {
    // req:check.health/workaround-command@36affded
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    for _ in 0..2 {
        assert_eq!(flag_x(&root).status, 0);
    }
    assert_eq!(
        decide(
            &root,
            clm_ledger::Rating::High,
            clm_ledger::Decision::Accept
        ),
        "decided capability/p accept (value high), settles 2 flags\n",
    );
    let out = flag_x(&root);
    assert_eq!(out.status, 0, "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn settled_slug_never_reused() {
    // req:check.health/workaround-command@36affded
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    for _ in 0..2 {
        assert_eq!(flag_x(&root).status, 0);
    }
    decide(
        &root,
        clm_ledger::Rating::Low,
        clm_ledger::Decision::Redesign,
    );
    // The spec change that rewrote the requirement deleted its flags.
    for n in 1..=2 {
        fs::remove_file(root.join(format!("backlog/workaround-a-x-{n}.md"))).unwrap();
    }
    let out = flag_x(&root);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "backlog/workaround-a-x-3.md\n",
        "{out:?}"
    );
}

#[test]
fn workaround_flag_from_settings() {
    // req:check.health/workaround-command@36affded
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    // `crates/clm` passes `[effort] workaround_flag = 2` from `clm.toml`.
    assert_eq!(flag_x_with(&root, Ok(2)).status, 0);
    let out = flag_x_with(&root, Ok(2));
    assert_eq!(out.status, 3, "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("2 unsettled workarounds on capability/p")
    );
}

#[test]
fn workarounds_not_counted_when_settings_fail() {
    // req:check.health/workaround-command@36affded
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    // What `crates/clm` passes when `config` rejects `[effort] workaround_flag = 0`.
    let rejected = "clm.toml: `effort`: `workaround_flag` must be at least 1".to_string();
    let out = flag_x_with(&root, Err(rejected));
    assert_eq!(out.status, 0, "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "backlog/workaround-a-x-1.md\n"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.starts_with("clm workaround: warning: the workarounds weren't counted: "),
        "{stderr}"
    );
}

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
}

/// A directory that isn't a git repository, holding the root spec and component `a` with
/// requirements `x` and `y`, neither serving a capability.
fn plain_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &common::spec("[]", &["x", "y"]),
    );
    let inside = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        !inside.status.success(),
        "the temporary directory is in a git work tree"
    );
    dir
}

#[test]
fn scenario_nothing_to_count() {
    // req:check.health/workaround-command@36affded
    let dir = plain_dir();
    let out = call_workaround(dir.path(), "a/y", "x", "b\n", Ok(3));
    assert_eq!(out.status, 0, "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "backlog/workaround-a-y-1.md\n"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let warning = "clm workaround: warning: the workarounds weren't counted: ";
    assert!(
        stderr.starts_with(warning)
            && stderr.len() > warning.len() + 1
            && stderr.lines().count() == 1,
        "{stderr}"
    );
    assert!(dir.path().join("backlog/workaround-a-y-1.md").is_file());
}

/// A stream whose every read and write fails.
struct Broken;

impl std::io::Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("stdin is gone"))
    }
}

impl std::io::Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("stdout is gone"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn scenario_the_body_cant_be_read_or_written() {
    // req:check.health/workaround-command@36affded
    let run = |root: &Path, stdin: &mut dyn std::io::Read, out: &mut dyn std::io::Write| {
        let mut stderr = Vec::new();
        let status = clm_health::workaround_command(
            ["a/x", "--title", "x"],
            root,
            Ok(3),
            stdin,
            &|_| None,
            out,
            &mut stderr,
        );
        (status, String::from_utf8(stderr).unwrap())
    };

    // A standard input whose read fails: nothing written.
    let dir = repo();
    write(dir.path(), "specs/a/SPEC.md", &common::spec("[]", &["x"]));
    let mut stdout = Vec::new();
    let (status, stderr) = run(dir.path(), &mut Broken, &mut stdout);
    assert_eq!(status, 1, "{stderr}");
    assert_eq!(stderr, "clm workaround: reading the body: stdin is gone\n");
    assert!(stdout.is_empty());
    assert!(!dir.path().join("backlog").exists(), "a flag was written");

    // A file named `backlog` where the directory would be.
    write(dir.path(), "backlog", "");
    let (status, stderr) = run(dir.path(), &mut "b\n".as_bytes(), &mut stdout);
    assert_eq!(status, 1, "{stderr}");
    assert!(stderr.starts_with("clm workaround: backlog: "), "{stderr}");
    assert!(
        stderr.len() > "clm workaround: backlog: \n".len(),
        "{stderr}"
    );
    assert!(stdout.is_empty());

    // An output stream that refuses writes: the flag stays written.
    fs::remove_file(dir.path().join("backlog")).unwrap();
    let (status, stderr) = run(dir.path(), &mut "b\n".as_bytes(), &mut Broken);
    assert_eq!(status, 1, "{stderr}");
    assert_eq!(stderr, "clm workaround: stdout is gone\n");
    assert!(dir.path().join("backlog/workaround-a-x-1.md").is_file());
}
