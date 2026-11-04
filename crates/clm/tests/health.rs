//! `clm health`, run on this repository and on git repositories made here.

// clm:reads specs/**
// clm:reads backlog/**

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod ledger_common;

fn clm_health(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(["health", "--root"])
        .arg(root)
        .output()
        .unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
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
fn scenario_a_clean_repository() {
    // req:check.health/health-report@a5dd24b1
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = clm_health(&root);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
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
    // req:check.health/health-report@a5dd24b1
    let dir = repo();
    let d = dir.path();
    write(d, "src/t.rs", &format!("// {}:a/ghost@00000000\n", "req"));
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
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!d.join("ran").exists(), "a detector ran");
}

#[test]
fn scenario_settings_cant_be_loaded() {
    // req:check.health/health-report@a5dd24b1
    let dir = repo();
    write(dir.path(), "clm.toml", "[health]\nreconsider = 0\n");
    let out = clm_health(dir.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        stderr.contains("clm.toml") && stderr.contains("`health`"),
        "{stderr}"
    );
}

#[test]
fn a_missing_root_spec_exits_1_naming_it() {
    // req:check.health/health-report@a5dd24b1
    let dir = TempDir::new().unwrap();
    let out = clm_health(dir.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr.contains("specs/SPEC.md"), "{stderr}");
}

/// `clm workaround <requirement> --title <title> --root <root>` with `body` on stdin.
fn clm_workaround(root: &Path, requirement: &str, title: &str, body: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["workaround", requirement, "--title", title, "--root"])
        .arg(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn workaround_writes_the_flag_from_stdin_and_commits_nothing() {
    // req:check.health/workaround-command@d9c5203d
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
    assert_eq!(out.status.code(), Some(0), "{out:?}");
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
    assert_eq!(out.status.code(), Some(1), "{out:?}");
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
    write(&root, "clm.toml", "host = \"none\"\n");
    root
}

/// `clm workaround a/x` in `root`, its user files and ledger writer from `root`.
fn flag_x(root: &Path) -> Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["workaround", "a/x", "--title", "Faked it", "--root"])
        .arg(root)
        .env("HOME", root)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("CLM_WRITER")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"b\n").unwrap();
    child.wait_with_output().unwrap()
}

const CROWDED: &str = "clm workaround: warning: 3 unsettled workarounds on capability/p: stop and ask whether to redesign it (redesign-before-workaround)\n";

#[test]
fn third_flag_exits_3() {
    // req:check.health/workaround-command@d9c5203d
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    for _ in 0..2 {
        assert_eq!(flag_x(&root).status.code(), Some(0));
    }
    let out = flag_x(&root);
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "backlog/workaround-a-x-3.md\n"
    );
    assert_eq!(String::from_utf8_lossy(&out.stderr), CROWDED);
    assert!(root.join("backlog/workaround-a-x-3.md").is_file());
}

#[test]
fn second_flag_exits_0() {
    // req:check.health/workaround-command@d9c5203d
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    assert_eq!(flag_x(&root).status.code(), Some(0));
    let out = flag_x(&root);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn settled_flags_not_counted() {
    // req:check.health/workaround-command@d9c5203d
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    for _ in 0..2 {
        assert_eq!(flag_x(&root).status.code(), Some(0));
    }
    let decide = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args([
            "capabilities",
            "decide",
            "p",
            "--value",
            "high",
            "--decision",
            "accept",
            "--root",
        ])
        .arg(&root)
        .env("HOME", &root)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&decide.stdout),
        "decided capability/p accept (value high), settles 2 flags\n",
        "{decide:?}"
    );
    let out = flag_x(&root);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn settled_slug_never_reused() {
    // req:check.health/workaround-command@d9c5203d
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    for _ in 0..2 {
        assert_eq!(flag_x(&root).status.code(), Some(0));
    }
    let decide = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args([
            "capabilities",
            "decide",
            "p",
            "--value",
            "low",
            "--decision",
            "redesign",
            "--root",
        ])
        .arg(&root)
        .env("HOME", &root)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("CLM_WRITER")
        .output()
        .unwrap();
    assert_eq!(decide.status.code(), Some(0), "{decide:?}");
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
    // req:check.health/workaround-command@d9c5203d
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    write(
        &root,
        "clm.toml",
        "host = \"none\"\n[effort]\nworkaround_flag = 2\n",
    );
    assert_eq!(flag_x(&root).status.code(), Some(0));
    let out = flag_x(&root);
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("2 unsettled workarounds on capability/p")
    );
}

#[test]
fn workarounds_not_counted_when_settings_fail() {
    // req:check.health/workaround-command@d9c5203d
    let dir = TempDir::new().unwrap();
    let root = served_repo(dir.path());
    write(
        &root,
        "clm.toml",
        "host = \"none\"\n[effort]\nworkaround_flag = 0\n",
    );
    let out = flag_x(&root);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
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
