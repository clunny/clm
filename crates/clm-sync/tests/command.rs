//! `clm rebase` and `clm autosync` through `clm-sync`'s calls (`commands-in-crate`): what they
//! print and how they exit, with the settings `crates/clm` would load given as an argument.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn commit(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", msg]);
}

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

/// `config`'s defaults, with the size budgets it gives.
fn defaults() -> clm_health::Settings {
    clm_health::Settings {
        reconsider: 3,
        scenarios: 6,
        exceptions: 4,
        stale_days: 30,
        lcov: None,
        land_check_target_seconds: 120,
        component_size: clm_check::Budgets {
            spec_tokens: 12000,
            code_tokens: 40000,
        },
    }
}

/// The settings `crates/clm` loads from `root`'s `clm.toml`, as far as these tests set them:
/// `[health] reconsider` and `[component_size] spec_tokens`, which must be at least 1; any
/// other key is unknown. An error is the line `clm check` prints for it.
fn repository_settings(root: &Path) -> Result<clm_health::Settings, String> {
    let path = root.join("clm.toml");
    let mut settings = defaults();
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(settings);
    };
    let fail = |msg: String| {
        Err(format!(
            "{}: check.health/health-report: {msg}",
            path.display()
        ))
    };
    let table: toml::Table = match text.parse() {
        Ok(table) => table,
        Err(e) => return fail(e.to_string()),
    };
    for (key, value) in &table {
        match key.as_str() {
            "health" => {
                if let Some(n) = value.get("reconsider").and_then(|n| n.as_integer()) {
                    settings.reconsider = n as u64;
                }
            }
            "component_size" => {
                let n = value.get("spec_tokens").and_then(|n| n.as_integer());
                match n {
                    Some(n) if n < 1 => {
                        return fail("`component_size`: `spec_tokens` must be at least 1".into());
                    }
                    Some(n) => settings.component_size.spec_tokens = n as u64,
                    None => {}
                }
            }
            other => return fail(format!("unknown field `{other}`")),
        }
    }
    Ok(settings)
}

/// `clm <args>` in `dir`, with `env` as the environment: the call `args`' first word names,
/// given the rest.
fn clm_with(dir: &Path, args: &[&str], env: &dyn Fn(&str) -> Option<String>) -> Output {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let rest = args[1..].iter().map(OsString::from);
    let code = match args[0] {
        "rebase" => clm_sync::rebase_command(
            rest,
            dir,
            env,
            &mut stdout,
            &mut stderr,
            repository_settings,
        ),
        "autosync" => clm_sync::autosync_command(rest, dir, env, &mut stdout, &mut stderr),
        other => panic!("no call for `clm {other}`"),
    };
    Output {
        status: Status(code),
        stdout,
        stderr,
    }
}

fn no_env(_: &str) -> Option<String> {
    None
}

fn clm(dir: &Path, args: &[&str]) -> Output {
    clm_with(dir, args, &no_env)
}

/// A work clone of a bare `origin`, on branch `pr`, where `pr` and `origin/main` both
/// changed `src/lib.rs`.
fn conflicting() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let (origin, work) = (dir.path().join("origin"), dir.path().join("work"));
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &["clone", "-q", origin.to_str().unwrap(), "work"],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(
        &work,
        "specs/SPEC.md",
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    );
    write(&work, "src/lib.rs", "one\n");
    commit(&work, "base");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    write(&work, "src/lib.rs", "theirs\n");
    commit(&work, "main moves");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", "pr", "HEAD~1"]);
    write(&work, "src/lib.rs", "mine\n");
    commit(&work, "work");
    (dir, work)
}

#[test]
fn a_command_called_in_a_test_writes_its_streams_and_returns_its_status() {
    // req:sync/commands-in-crate@8e0be5d8 req:sync/autosync@462cfce4
    let (dir, work) = conflicting();
    // `--root` overrides the root the call is given.
    let elsewhere = dir.path().join("not-a-repo");
    fs::create_dir_all(&elsewhere).unwrap();
    let root = work.to_str().unwrap();
    let out = clm(&elsewhere, &["rebase", "--root", root]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "conflict src/lib.rs\nrebase incomplete\n"
    );
    assert!(out.stderr.is_empty(), "{out:?}");

    // A refusal goes to the error stream with the command's prefix, its outcome to stdout.
    git(&work, &["rebase", "--abort"]);
    git(&work, &["checkout", "-q", "main"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).starts_with("clm rebase: "),
        "{out:?}"
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "rebase incomplete\n");

    // `clm autosync` off a `spec/*` branch, here outside any repository, does nothing.
    let out = clm(&work, &["autosync", "--root", elsewhere.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
}

#[test]
fn the_temporary_worktree_goes_under_the_tmpdir_the_environment_names() {
    // req:sync/commands-in-crate@8e0be5d8 req:sync/judged-by-check@bfefb55e
    let (dir, work) = node_a("");
    // A `TMPDIR` that is a file: no worktree can be made under it, so the run fails as git
    // does, whatever the process's own `TMPDIR` is.
    let file = dir.path().join("a-file");
    fs::write(&file, "").unwrap();
    let env = |key: &str| (key == "TMPDIR").then(|| file.to_string_lossy().into_owned());
    let out = clm_with(&work, &["rebase"], &env);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "rebase failed\n");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(&*file.to_string_lossy()),
        "{out:?}"
    );
}

#[test]
fn a_failed_autosync_writes_why_and_returns_the_failures_status() {
    // req:sync/commands-in-crate@8e0be5d8 req:sync/autosync@462cfce4
    let (dir, work) = conflicting();
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    write(&work, "specs/SPEC.md", "edited\n");
    let gone = dir.path().join("gone");
    git(
        &work,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let out = clm(&work, &["autosync"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).starts_with("clm autosync: "),
        "{out:?}"
    );
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn arguments_it_cant_parse_return_2_with_the_usage_error() {
    // req:sync/commands-in-crate@8e0be5d8 req:sync/outcome-line@cac8dfb4
    let (_dir, work) = conflicting();
    for name in ["rebase", "autosync"] {
        let out = clm(&work, &[name, "--no-such-option"]);
        assert_eq!(out.status.code(), Some(2), "{name}: {out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("--no-such-option"), "{name}: {stderr}");
        assert!(stderr.contains(&format!("clm {name}")), "{name}: {stderr}");
        assert!(out.stdout.is_empty(), "{name}: {out:?}");
    }
}

#[test]
fn help_names_each_command() {
    // req:sync/commands-in-crate@8e0be5d8
    let (_dir, work) = conflicting();
    for (name, about) in [
        (
            "rebase",
            "Rebase the branch onto origin/main, regenerate applied specs and run `clm check`; \
             exit 1 with what is left to resolve",
        ),
        (
            "autosync",
            "Push a snapshot of the spec work on a `spec/*` branch to origin as \
             `autosync/<branch>`",
        ),
    ] {
        let out = clm(&work, &[name, "--help"]);
        assert_eq!(out.status.code(), Some(0), "{name}: {out:?}");
        assert!(out.stderr.is_empty(), "{name}: {out:?}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.first(), Some(&about), "{name}: {stdout}");
        let usage = lines.iter().find(|l| l.starts_with("Usage:"));
        assert_eq!(
            usage,
            Some(&format!("Usage: clm {name} [OPTIONS]").as_str()),
            "{name}: {stdout}"
        );
        assert!(
            lines.iter().any(|l| l.trim_start().starts_with("--root")),
            "{name}: {stdout}"
        );
    }
}

#[test]
fn help_and_usage_errors_print_no_outcome_line() {
    // req:sync/outcome-line@cac8dfb4
    let (_dir, work) = conflicting();
    let outcomes = ["rebase ok", "rebase incomplete", "rebase failed"];
    let out = clm(&work, &["rebase", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage: clm rebase"), "{stdout}");
    assert!(
        !stdout.lines().any(|l| outcomes.contains(&l)),
        "an outcome line in the help: {stdout}"
    );
    let out = clm(&work, &["rebase", "--no-such-option"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn sync_prints_what_is_left_and_exits_1_then_0() {
    // req:sync/rebase@3ba762e9 req:sync/outcome-line@cac8dfb4
    let (_dir, work) = conflicting();
    write(&work, "src/lib.rs", "edited\n");
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "uncommitted src/lib.rs\nrebase incomplete\n"
    );
    git(&work, &["checkout", "-q", "--", "src/lib.rs"]);

    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "conflict src/lib.rs\nrebase incomplete\n"
    );

    write(&work, "src/lib.rs", "both\n");
    git(&work, &["add", "src/lib.rs"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "rebase ok\n",
        "{out:?}"
    );
}

#[test]
fn sync_refusing_exits_1_and_git_failing_exits_2() {
    // req:sync/rebase@3ba762e9 req:sync/outcome-line@cac8dfb4
    let (dir, work) = conflicting();
    git(&work, &["checkout", "-q", "main"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("main"),
        "{out:?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "rebase incomplete\n",
        "{out:?}"
    );

    // An `origin` that isn't there: the fetch fails.
    git(&work, &["checkout", "-q", "pr"]);
    let gone = dir.path().join("gone");
    git(
        &work,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "rebase failed\n",
        "{out:?}"
    );

    let elsewhere = dir.path().join("not-a-repo");
    fs::create_dir_all(&elsewhere).unwrap();
    assert_eq!(clm(&elsewhere, &["rebase"]).status.code(), Some(2));
}

#[test]
fn a_refused_sync_keeps_its_output() {
    // req:sync/rebase@789d48ec
    let (_dir, work) = conflicting();
    git(&work, &["checkout", "-q", "main"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("clm rebase: "), "{out:?}");
    assert!(stderr.contains("`main`"), "the reason names main: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().last(), Some("rebase incomplete"), "{out:?}");

    git(&work, &["checkout", "-q", "--detach", "HEAD"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("clm rebase: "), "{out:?}");
    assert!(stderr.contains("detached"), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "rebase incomplete\n");
}

/// `git <args>` in `dir`, whatever its status: its stdout.
fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// What a refusal must leave as it was: the refs, the status, both diffs and git's own
/// rebase state.
fn untouched(work: &Path) -> [String; 5] {
    let state = |dir: &str| {
        let path = work.join(git_out(work, &["rev-parse", "--git-path", dir]).trim());
        path.exists()
    };
    [
        git_out(work, &["for-each-ref"]),
        git_out(work, &["status", "--porcelain"]),
        git_out(work, &["diff"]),
        git_out(work, &["diff", "--cached"]),
        format!("{} {}", state("rebase-merge"), state("rebase-apply")),
    ]
}

/// Runs `clm rebase` in `work`, asserting it refused naming `continue` and `abort` and left
/// everything as it was.
fn refused_naming(work: &Path, continue_: &str, abort: &str) {
    let before = untouched(work);
    let out = clm(work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("clm rebase: "), "{out:?}");
    assert!(stderr.contains("in progress"), "{out:?}");
    assert!(
        stderr.contains(continue_) && stderr.contains(abort),
        "{out:?}"
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "rebase incomplete\n");
    assert_eq!(untouched(work), before);
}

#[test]
fn a_rebase_started_by_hand_is_refused() {
    // req:sync/rebase@789d48ec
    let (_dir, work) = conflicting();
    git(&work, &["fetch", "-q", "origin"]);
    git_out(&work, &["rebase", "origin/main"]);
    assert!(git_out(&work, &["status"]).contains("rebase in progress"));
    refused_naming(&work, "git rebase --continue", "git rebase --abort");
}

#[test]
fn a_hand_rebase_after_its_own_finished_with_git_is_refused() {
    // req:sync/rebase@789d48ec
    let (dir, work) = conflicting();
    let out = clm(&work, &["rebase"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "conflict src/lib.rs\nrebase incomplete\n"
    );
    write(&work, "src/lib.rs", "both\n");
    git(&work, &["add", "src/lib.rs"]);
    git_out(&work, &["rebase", "--continue"]);
    assert!(!git_out(&work, &["status"]).contains("rebase in progress"));

    // `main` moves again, and a rebase by hand onto it stops.
    let other = dir.path().join("other");
    git(
        dir.path(),
        &[
            "clone",
            "-q",
            dir.path().join("origin").to_str().unwrap(),
            "other",
        ],
    );
    write(&other, "src/lib.rs", "theirs again\n");
    git(
        &other,
        &[
            "-c",
            "user.name=o",
            "-c",
            "user.email=o@o",
            "commit",
            "-qam",
            "again",
        ],
    );
    git(&other, &["push", "-q", "origin", "HEAD:main"]);
    write(&work, "src/lib.rs", "mine again\n");
    commit(&work, "again");
    git(&work, &["fetch", "-q", "origin"]);
    git_out(&work, &["rebase", "origin/main"]);
    assert!(git_out(&work, &["status"]).contains("rebase in progress"));
    refused_naming(&work, "git rebase --continue", "git rebase --abort");
}

#[test]
fn a_git_am_in_progress_is_refused() {
    // req:sync/rebase@789d48ec
    let (dir, work) = conflicting();
    // `main`'s commit, which doesn't apply on `pr`'s `src/lib.rs`.
    let patch = dir.path().join("p.patch");
    fs::write(
        &patch,
        git_out(&work, &["format-patch", "-1", "--stdout", "main"]),
    )
    .unwrap();
    git_out(&work, &["am", patch.to_str().unwrap()]);
    refused_naming(&work, "git am --continue", "git am --abort");
}

/// `conflicting()`'s work clone on a new branch `spec/x` with an uncommitted edit to
/// `specs/SPEC.md`, whose `origin` is `url` when given.
fn spec_branch_with_an_edit(url: Option<&str>) -> (TempDir, PathBuf) {
    let (dir, work) = conflicting();
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    write(&work, "specs/SPEC.md", "edited\n");
    if let Some(url) = url {
        git(&work, &["remote", "set-url", "origin", url]);
    }
    (dir, work)
}

#[test]
fn autosync_that_pushes_prints_nothing() {
    // req:sync/autosync@462cfce4
    let (dir, work) = spec_branch_with_an_edit(None);
    let out = clm(&work, &["autosync"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");
    let origin = dir.path().join("origin");
    let pushed = Command::new("git")
        .args(["rev-parse", "--verify", "-q", "refs/heads/autosync/spec/x"])
        .current_dir(&origin)
        .output()
        .unwrap();
    assert!(pushed.status.success(), "nothing pushed: {pushed:?}");
}

/// Puts `dir` first on this process's `PATH` and takes `GIT_SSH_COMMAND` out of its
/// environment; each test that calls it runs in a process of its own.
fn path_first_without_ssh_command(dir: &Path) {
    let path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: nextest runs each test in its own process, and this one starts no thread
    // before it.
    unsafe {
        std::env::set_var("PATH", format!("{}:{path}", dir.display()));
        std::env::remove_var("GIT_SSH_COMMAND");
    }
}

#[test]
fn autosync_lets_ssh_neither_ask_nor_wait() {
    // req:sync/autosync@462cfce4
    let (dir, work) = spec_branch_with_an_edit(Some("fakehost:repo.git"));
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let log = dir.path().join("ssh.log");
    exec::write_exec(
        &bin.join("ssh"),
        &format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", log.display()),
    );
    path_first_without_ssh_command(&bin);
    // The process's own `GIT_SSH_COMMAND` isn't the environment the call is given.
    let ran = dir.path().join("process-ssh-ran");
    let process_ssh = dir.path().join("process-ssh");
    exec::write_exec(
        &process_ssh,
        &format!("#!/bin/sh\ntouch '{}'\nexit 1\n", ran.display()),
    );
    // SAFETY: as in `path_first_without_ssh_command`.
    unsafe { std::env::set_var("GIT_SSH_COMMAND", &process_ssh) };

    let out = clm(&work, &["autosync"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(!ran.exists(), "the process's ssh command ran");
    let args = fs::read_to_string(&log).expect("ssh never ran");
    assert!(args.contains("-o BatchMode=yes"), "{args}");
    assert!(args.contains("-o ConnectTimeout=10"), "{args}");
}

#[test]
fn autosync_hands_git_the_ssh_command_it_is_given() {
    // req:sync/autosync@462cfce4
    let (dir, work) = spec_branch_with_an_edit(Some("fakehost:repo.git"));
    let ran = dir.path().join("ran");
    let ssh = dir.path().join("my-ssh");
    exec::write_exec(
        &ssh,
        &format!("#!/bin/sh\ntouch '{}'\nexit 1\n", ran.display()),
    );
    path_first_without_ssh_command(&dir.path().join("no-such-dir"));
    let command = ssh.to_string_lossy().into_owned();
    let env = |key: &str| (key == "GIT_SSH_COMMAND").then(|| command.clone());

    let out = clm_with(&work, &["autosync"], &env);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).starts_with("clm autosync: "),
        "{out:?}"
    );
    assert!(ran.exists(), "the given ssh command never ran: {out:?}");
}

#[test]
fn autosync_exits_0_off_a_spec_branch() {
    // req:sync/autosync@462cfce4
    let (_dir, work) = conflicting();
    let out = clm(&work, &["autosync"]);
    assert_eq!(
        (out.status.code(), out.stdout.len()),
        (Some(0), 0),
        "{out:?}"
    );
}

/// A work clone of a bare `origin`, on branch `impl/a`, whose node `a` owns `src/a/x.rs` and
/// whose `clm.toml` holds `clm_toml`, all on `origin/main`.
fn node_a(clm_toml: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let (origin, work) = (dir.path().join("origin"), dir.path().join("work"));
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &["clone", "-q", origin.to_str().unwrap(), "work"],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(&work, "clm.toml", clm_toml);
    write(
        &work,
        "specs/SPEC.md",
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    );
    write(
        &work,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n\
         ## Contract\nc\n",
    );
    write(&work, "src/a/x.rs", "a\n");
    commit(&work, "base");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", "impl/a"]);
    (dir, work)
}

/// `clm rebase` in `dir`, with no user settings (`repository_settings` reads only the
/// repository's) and `TMPDIR` an empty directory of its own, which it asserts is empty again
/// afterwards; its stdout.
fn rebase_without_user_settings(dir: &Path) -> (Option<i32>, String) {
    let tmp = TempDir::new().unwrap();
    let env = |key: &str| (key == "TMPDIR").then(|| tmp.path().to_string_lossy().into_owned());
    let out = clm_with(dir, &["rebase"], &env);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let left: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
    assert!(left.is_empty(), "TMPDIR holds {left:?}: {stdout}");
    (
        out.status.code(),
        stdout.replace(&*tmp.path().to_string_lossy(), "<TMPDIR>"),
    )
}

#[test]
fn a_skipped_proposals_warning_is_written_as_clm_rebase() {
    // req:sync/warnings-name-the-command@92bb9d3e
    let (dir, work) = node_a("");
    let ledger = dir.path().join("ledger");
    git(dir.path(), &["init", "-q", ledger.to_str().unwrap()]);
    git(&ledger, &["config", "user.name", "l"]);
    git(&ledger, &["config", "user.email", "l@l"]);
    let record = serde_json::json!({
        "id": "p9",
        "kind": "proposed",
        "writer": "alice/web",
        "at": "2026-10-28T10:00:00.000Z",
        "slug": "Not Kebab",
        "title": "Bad",
        "body": "x",
        "nodes": ["a"],
    });
    write(&ledger, "records.jsonl", &format!("{record}\n"));
    commit(&ledger, "records");
    let origin = dir.path().join("origin");
    git(
        &ledger,
        &[
            "push",
            "-q",
            origin.to_str().unwrap(),
            "HEAD:refs/heads/clm/ledger/alice/web",
        ],
    );

    let out = clm(&work, &["rebase"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let skipped: Vec<_> = stderr
        .lines()
        .filter(|l| l.contains("skipped proposal"))
        .collect();
    assert_eq!(skipped.len(), 1, "{out:?}");
    assert!(
        skipped[0].starts_with("clm rebase: skipped proposal ") && skipped[0].contains("p9"),
        "{out:?}"
    );
}

/// `clm rebase` on `spec/x` of [`node_a`], `violation` committed on `main` when given, where
/// a `post-checkout` hook leaves, in the temporary worktree, a directory its owner can't
/// write, holding a file: its status, stdout, stderr, and the `sync-*` directory it made.
fn rebase_leaving_a_worktree(violation: bool) -> (Option<i32>, String, String, PathBuf) {
    let (dir, work) = node_a("");
    if violation {
        write(
            &work,
            "src/a/t.rs",
            concat!("// req", ":a/missing\n#[test]\nfn t() {}\n"),
        );
        commit(&work, "a violation");
        git(&work, &["push", "-q", "origin", "HEAD:main"]);
    }
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    let hooks = work.join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    exec::write_exec(
        &hooks.join("post-checkout"),
        "#!/bin/sh\ncase \"$PWD\" in */sync-*) mkdir -p stuck && : > stuck/f && chmod a-w stuck \
         ;; esac\n",
    );
    let tmp = dir.path().join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    let env = |key: &str| (key == "TMPDIR").then(|| tmp.to_string_lossy().into_owned());
    let out = clm_with(&work, &["rebase"], &env);
    let made: Vec<PathBuf> = fs::read_dir(&tmp)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let [made] = &made[..] else {
        panic!("{made:?}: {out:?}");
    };
    // Writable again, so the test's directory can go.
    Command::new("chmod")
        .args(["-R", "u+w"])
        .arg(made)
        .status()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        made.clone(),
    )
}

#[test]
fn a_worktree_left_behind_keeps_the_outcome() {
    // req:sync/judged-by-check@04b187e7
    let (code, stdout, stderr, made) = rebase_leaving_a_worktree(false);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert_eq!(stdout.lines().last(), Some("rebase ok"), "{stdout}");
    assert!(
        stderr.contains(&format!(
            "clm rebase: couldn't remove the temporary worktree {}: ",
            made.display()
        )),
        "{stderr}"
    );

    let (code, stdout, stderr, _) = rebase_leaving_a_worktree(true);
    assert_eq!(code, Some(1), "{stdout}{stderr}");
    assert!(stdout.contains("check/dangling-markers"), "{stdout}");
    assert_eq!(stdout.lines().last(), Some("rebase incomplete"), "{stdout}");
    assert!(
        stderr.contains("couldn't remove the temporary worktree"),
        "{stderr}"
    );
}

#[test]
fn sync_leaves_no_temporary_worktree() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    write(
        &work,
        "src/a/t.rs",
        concat!("// req", ":a/missing\n#[test]\nfn t() {}\n"),
    );
    commit(&work, "a violation");

    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("check/dangling-markers"), "{stdout}");
    let list = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(&work)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&list.stdout).lines().count(), 1);
}

#[test]
fn sync_ignores_an_uncommitted_settings_edit() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    git(&work, &["checkout", "-q", "main"]);
    write(&work, "notes.md", "main\n");
    commit(&work, "main moves");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "spec/x"]);
    write(&work, "clm.toml", "no_such_key = 1\n");

    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!((code, stdout.as_str()), (Some(0), "rebase ok\n"));
    assert_eq!(
        fs::read_to_string(work.join("clm.toml")).unwrap(),
        "no_such_key = 1\n"
    );
}

#[test]
fn sync_with_settings_that_cant_be_loaded() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("[component_size]\nspec_tokens = 0\n");
    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!(code, Some(1), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].contains("clm.toml: "), "{stdout}");
    assert!(lines[0].contains("component_size"), "{stdout}");
    assert!(lines[0].contains("check.health/health-report"), "{stdout}");
    assert_eq!(lines[1], "rebase incomplete");
}

#[test]
fn sync_fails_on_settings_main_brought_in_that_cant_be_loaded() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    write(&work, "notes.md", "mine\n");
    commit(&work, "work");
    // `main` gains a key this `clm` doesn't know after the branch was cut.
    git(&work, &["checkout", "-q", "main"]);
    write(&work, "clm.toml", "no_such_key = 1\n");
    commit(&work, "new key");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "spec/x"]);

    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!(code, Some(1), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines[0].contains("clm.toml: ") && lines[0].contains("no_such_key"),
        "{stdout}"
    );
    // Named by its path in the repository, no part of it under the temporary directory.
    assert!(!lines[0].contains("<TMPDIR>"), "{stdout}");
    assert_eq!(lines.last(), Some(&"rebase incomplete"), "{stdout}");
    assert_eq!(
        fs::read_to_string(work.join("clm.toml")).unwrap(),
        "no_such_key = 1\n",
        "the branch is rebased"
    );
}

#[test]
fn sync_files_a_reconsider_item_at_the_repository_threshold() {
    // req:sync/file-reconsider-item@1f67b16b
    let (_dir, work) = node_a("[health]\nreconsider = 2\n");
    write(
        &work,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n\
         ## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n\
         #### Scenario: s\n- ok\n",
    );
    let graph = clm_model::load(&work).0;
    let version = graph.requirement("a/r").unwrap().version().to_string();
    for n in 1..=2 {
        write(
            &work,
            &format!("backlog/friction-a-r-{n}.md"),
            &format!(
                "---\ntitle: \"Hard {n}\"\nkind: friction\ncomponents: [a]\n\
                 requirement: a/r@{version}\n---\n\nHard.\n"
            ),
        );
    }
    commit(&work, "two flags");

    // Two flags reach the repository's threshold of 2, under the default of 3.
    assert_eq!(
        rebase_without_user_settings(&work),
        (
            Some(0),
            "filed backlog/reconsider-a-r.md\nrebase ok\n".into()
        ),
    );
}
