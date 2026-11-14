//! A git query or a `gh` lookup that outlasts its budget blocks, and nothing is left running:
//! `guard_within` with budgets of about a second, where through the binary these waited out
//! the hook's real ones (`hook-budgets`). One test function, since it puts the hanging fakes
//! first on its process's `PATH`: `nextest` runs each test in a process of its own, and with
//! one function a plain `cargo test` shares that environment with no other test.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use clm_guard::{Budgets, Decision, guard_within};
use tempfile::TempDir;

fn executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Puts a directory holding `program`, a script, first on this process's `PATH`.
fn fake_first(dir: &Path, program: &str, script: &str, original: &str) {
    executable(&dir.join(program), script);
    let path = format!("{}:{original}", dir.display());
    // SAFETY: this is the only test function of its process, and it spawns no thread.
    unsafe { std::env::set_var("PATH", path) };
}

fn judge(cwd: &Path, command: &str, budgets: &Budgets) -> (Decision, Duration) {
    let event = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": cwd,
    })
    .to_string();
    let started = Instant::now();
    let decision = guard_within(&event, budgets);
    (decision, started.elapsed())
}

/// Whether no live process `pid` exists (a zombie counts as gone), waiting up to thirty
/// seconds for one that is exiting, since a loaded host may be slow to end it.
fn gone(pid: &str) -> bool {
    (0..300).any(|_| {
        let alive = Command::new("sh")
            .args(["-c", &format!("ps -o stat= -p {pid} | grep -qv Z")])
            .status()
            .unwrap()
            .success();
        if alive {
            std::thread::sleep(Duration::from_millis(100));
        }
        !alive
    })
}

fn all_gone(pids: &Path, count: usize) {
    let pids = fs::read_to_string(pids).unwrap();
    let pids: Vec<&str> = pids.split_whitespace().collect();
    assert_eq!(pids.len(), count, "{pids:?}");
    for pid in pids {
        assert!(
            gone(pid),
            "process {pid} still running after the guard returned"
        );
    }
}

#[test]
fn queries_and_lookups_that_outlast_their_budget_block() {
    // req:plugin.hooks.guard/push-guard@dabbe399
    // req:plugin.hooks.guard/release-guard@6e589859
    // req:plugin.hooks.guard/release-run-lookup@3024a9da
    let original = std::env::var("PATH").unwrap();
    let cwd = TempDir::new().unwrap();
    let one_second = Budgets {
        push: Duration::from_secs(1),
        lookup: Duration::from_secs(1),
    };

    // A push git doesn't answer: the fake `git` writes its process ID to `pids` and then
    // `exec`s its sleep, so the sleep has that ID.
    let git = TempDir::new().unwrap();
    let pids = git.path().join("pids");
    fake_first(
        git.path(),
        "git",
        &format!(
            "#!/bin/sh\necho $$ >> '{}'\nexec sleep 60\n",
            pids.display()
        ),
        &original,
    );
    let (decision, took) = judge(cwd.path(), "git push", &one_second);
    let Decision::Block(why) = decision else {
        panic!("not blocked: {decision:?}")
    };
    // Well under the fake's 60 seconds, and wide enough for a loaded host.
    assert!(took.as_secs() < 30, "{took:?}");
    assert!(
        why.contains("guard-blocks-push-to-main")
            && why.contains("didn't answer in time")
            && why.contains("HEAD:<branch>"),
        "{why}"
    );
    all_gone(&pids, 1);
    // After a `cd` the guard couldn't follow, the reason for a query that ran out of time
    // doesn't change.
    let (decision, _) = judge(cwd.path(), "cd nowhere && git push", &one_second);
    let Decision::Block(why) = decision else {
        panic!("not blocked: {decision:?}")
    };
    assert!(
        why.contains("didn't answer in time") && !why.contains("couldn't follow"),
        "{why}"
    );
    // An explicit branch needs no query.
    fs::remove_file(&pids).unwrap();
    let (decision, _) = judge(cwd.path(), "git push origin impl/x/b", &one_second);
    assert_eq!(decision, Decision::Allow);
    assert!(!pids.exists(), "the fake git ran");

    // The push queries of a command share one budget: each takes 0.4 seconds and answers
    // nothing, so a segment's three fit in 2 seconds and two segments' six don't. The fifth
    // query is killed at 2 seconds, and the sixth is never started; on a loaded host each
    // query takes longer, so fewer start.
    let calls = git.path().join("calls");
    executable(
        &git.path().join("git"),
        &format!(
            "#!/bin/sh\necho \"$*\" >> '{}'\nsleep 0.4\nexit 1\n",
            calls.display()
        ),
    );
    let two_seconds = Budgets {
        push: Duration::from_secs(2),
        ..one_second
    };
    let (decision, took) = judge(cwd.path(), "git push; git push", &two_seconds);
    let Decision::Block(why) = decision else {
        panic!("not blocked: {decision:?}")
    };
    assert!(why.contains("didn't answer in time"), "{why}");
    assert!(took.as_secs() < 30, "{took:?}");
    let calls = fs::read_to_string(&calls).unwrap();
    // Spawn overhead may delay the fifth past the deadline; a sixth never starts.
    assert!((1..=5).contains(&calls.lines().count()), "{calls}");

    // The lookups of a command share one budget: `repo view` and `api` each answer in 0.6
    // seconds, so the two don't fit in 1, and fit in 10 even on a loaded host.
    let slow = TempDir::new().unwrap();
    fake_first(
        slow.path(),
        "gh",
        "#!/bin/sh\nsleep 0.6\nif [ \"$1\" = repo ]; then \
         echo '{\"url\":\"https://github.com/o/r\"}'; exit 0; fi\n\
         echo '{\"name\":\"CI\",\"path\":\".github/workflows/ci.yml\"}'\n",
        &original,
    );
    let (decision, _) = judge(cwd.path(), "gh run rerun 42", &one_second);
    assert!(matches!(decision, Decision::Block(_)), "{decision:?}");
    let two_lookups = Budgets {
        lookup: Duration::from_secs(10),
        ..one_second
    };
    let (decision, _) = judge(cwd.path(), "gh run rerun 42", &two_lookups);
    assert_eq!(decision, Decision::Allow);

    // A lookup that never answers: one never answers `repo view`, one answers it but never
    // `api`. Each writes its process ID to `pids` and then `exec`s its sleep.
    for answers_repo_view in [false, true] {
        let gh = TempDir::new().unwrap();
        let pids = gh.path().join("pids");
        let view = if answers_repo_view {
            "if [ \"$1\" = repo ]; then echo '{\"url\":\"https://github.com/o/r\"}'; exit 0; fi\n"
        } else {
            ""
        };
        fake_first(
            gh.path(),
            "gh",
            &format!(
                "#!/bin/sh\n{view}echo $$ >> '{}'\nexec sleep 60\n",
                pids.display()
            ),
            &original,
        );
        for command in ["gh run rerun 42", "gh run rerun --job 900"] {
            let (decision, took) = judge(cwd.path(), command, &one_second);
            let Decision::Block(why) = decision else {
                panic!("{command}: not blocked: {decision:?}")
            };
            assert!(why.contains("humans-cut-releases"), "{command}: {why}");
            assert!(took.as_secs() < 30, "{command}: {took:?}");
        }
        all_gone(&pids, 2);
    }
}
