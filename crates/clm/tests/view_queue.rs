//! `clm view queue`: the build loop's plan, read from the clone (`view-queue`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// `clm <args> --root <root>`, with no user settings.
fn clm(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(dir: &Path, args: &[&str]) -> String {
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
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// A repository whose component `a` has one requirement no test cites, so `a` is ready, on
/// `main` with `origin/main` at the same commit.
fn ready_a() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n## Requirements\n\n\
         ### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
    );
    git(d, &["init", "-q", "-b", "main"]);
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "base"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    dir
}

/// `<git common dir>/clm/loop`, where the loop keeps its file.
fn loop_file(d: &Path) -> PathBuf {
    d.join(".git/clm/loop")
}

fn state(d: &Path) -> (String, String) {
    (
        git(d, &["status", "--porcelain", "--untracked-files=all"]),
        git(d, &["write-tree"]),
    )
}

/// A loop file held now, running `build:a`, with a fold batch `x` waiting on the human and
/// a stored plan.
fn write_loop(d: &Path) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let file = serde_json::json!({
        "version": 1,
        "lease": now,
        "started_main": "abc",
        "session_model": "opus",
        "runs": [{
            "name": "build:a", "kind": "build", "model": "sonnet", "state": "running",
            "task_id": "t1", "started": now, "round": 1,
        }],
        "drafted": [],
        "review": ["x"],
        "batches": [{"id": "x", "approved": false}],
        "red_verifies": [],
        "escalations": [],
        "conditions": {"disk": false, "limit_until": null},
        "stop": null,
        "plan": {"main": "abc", "at": now, "next": [
            {"name": "build:b", "kind": "build", "priority": "P2", "reason": "waits for a place"}
        ]},
    });
    fs::create_dir_all(d.join(".git/clm")).unwrap();
    fs::write(loop_file(d), file.to_string()).unwrap();
}

#[test]
fn no_loop_yet() {
    // req:view.repo/view-queue@7aa0fdff
    let dir = ready_a();
    let d = dir.path();
    let before = state(d);
    let out = clm(d, &["view", "queue"]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(
        text.lines().next().unwrap().starts_with("commit "),
        "{text}"
    );
    assert!(text.contains("loop not held"), "{text}");
    let after_next = text.split("next up").nth(1).unwrap();
    assert!(after_next.contains("build:a"), "{text}");
    assert!(text.contains("no loop has run on this clone"), "{text}");
    assert!(!loop_file(d).exists());
    // Reading the loop's file may make its directory, but no file goes in it.
    let files = fs::read_dir(d.join(".git/clm")).map_or(0, Iterator::count);
    assert_eq!(files, 0, "no file is written");
    assert_eq!(state(d), before);
}

#[test]
fn no_loop_yet_at_a_commit_reads_that_commit() {
    // req:view.repo/view-queue@7aa0fdff
    let dir = ready_a();
    let d = dir.path();
    // `origin/main` gains a commit that builds `a`, so only the working tree lists it.
    let hash = git(d, &["rev-parse", "HEAD"]);
    let x = {
        let (graph, _) = clm_model::load(d);
        graph.requirement("a/x").unwrap().version().to_string()
    };
    write(
        d,
        "tests/t.rs",
        &format!("// {}a/x@{x}\n", concat!("req", ":")),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "cite"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(d, &["reset", "-q", "--hard", hash.trim()]);
    let here = stdout(&clm(d, &["view", "queue"]));
    assert!(here.contains("build:a"), "{here}");
    let at = stdout(&clm(d, &["view", "queue", "--at", "origin/main"]));
    assert!(at.starts_with("origin/main "), "{at}");
    assert!(!at.contains("build:a"), "{at}");
}

#[test]
fn the_loops_plan() {
    // req:view.repo/view-queue@7aa0fdff
    let dir = ready_a();
    let d = dir.path();
    write_loop(d);
    let before = state(d);
    let loop_before = fs::read(loop_file(d)).unwrap();
    let out = clm(
        d,
        &["view", "queue", "--at", "origin/main", "--format", "json"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["view"], "queue");
    assert_eq!(v["running"][0]["name"], "build:a");
    // The plan's stored next-up entries, not the working tree's queue (where `a` is ready).
    assert_eq!(v["next"][0]["name"], "build:b");
    assert_eq!(v["next"].as_array().unwrap().len(), 1);
    assert_eq!(v["at"]["rev"], "origin/main");
    // What waits on the human is whatever the loop's plan says: the same as `clm build`'s.
    let names: Vec<&str> = v["waiting_on_you"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    let stored = clm_loop::stored_plan(d).unwrap();
    let expected: Vec<&str> = stored
        .waiting_on_you
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, expected);
    assert_eq!(fs::read(loop_file(d)).unwrap(), loop_before);
    assert_eq!(state(d), before);
}

#[test]
fn at_changes_only_the_checkout_line_of_a_stored_plan() {
    // req:view.repo/view-queue@7aa0fdff
    let dir = ready_a();
    let d = dir.path();
    write_loop(d);
    let here = stdout(&clm(d, &["view", "queue"]));
    let at = stdout(&clm(d, &["view", "queue", "--at", "origin/main"]));
    assert!(here.starts_with("commit "), "{here}");
    assert!(at.starts_with("origin/main "), "{at}");
    let rest = |s: &str| s.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(rest(&here), rest(&at));
    assert!(here.contains("loop held, renewed 0m ago"), "{here}");
}

#[test]
fn the_queue_has_no_svg_form_and_a_missing_commit_is_named() {
    // req:view.repo/view-queue@7aa0fdff
    let dir = ready_a();
    let d = dir.path();
    let svg = clm(d, &["view", "queue", "--format", "svg"]);
    assert_eq!(svg.status.code(), Some(1));
    assert!(stdout(&svg).is_empty());
    assert!(String::from_utf8_lossy(&svg.stderr).contains("svg"));
    let none = clm(d, &["view", "queue", "--at", "nowhere"]);
    assert_eq!(none.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&none.stderr).contains("nowhere"));
}
