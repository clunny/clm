//! `clm build step` and `launched` (`step-actions`). The `clm build` dispatch and the capability
//! test that drains its queue through it are in `crates/clm`.

mod common;

use std::fs;

use clm_git::Failure;
use clm_loop::{Kind, RunState, StopMode, launched, start, step, stop, stored_plan};
use common::{Fixture, line};

#[test]
fn an_unreported_launch() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let first = step(&fx.options()).unwrap();
    let launch = line(&first, &common::launch_on_build_model("build:a", "build")).to_string();
    // The next step prints the same line, and prepared nothing again.
    let second = step(&fx.options()).unwrap();
    assert_eq!(second, [launch]);
    let runs: Vec<String> = fx
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("run a "))
        .collect();
    assert_eq!(runs.len(), 1, "{runs:?}");
    // The build's final review runs on the session's model.
    assert!(runs[0].starts_with("run a --review-model "), "{runs:?}");
}

#[test]
fn launched_records_the_task() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    let fx = Fixture::new();
    fx.set("next", "a\n");
    step(&fx.options()).unwrap();
    assert_eq!(fx.file().runs[0].state, RunState::Planned);
    fx.add_run(common::run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Returned,
        Some("t-2"),
    ));
    // `launched` fetches nothing: an `origin` that can't be reached doesn't fail it.
    let missing = fx.tmp.path().join("no-such-origin");
    clm_git::git(
        &fx.work,
        &["remote", "set-url", "origin", missing.to_str().unwrap()],
    )
    .unwrap();
    launched(&fx.options(), "build:a", "t-17").unwrap();
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Running);
    assert_eq!(run.task_id.as_deref(), Some("t-17"));
    // A run the loop doesn't list, and one it lists as returned, are refused, whatever the task
    // ID, changing nothing.
    let loop_file = fx.loop_dir().join("loop");
    let before = fs::read(&loop_file).unwrap();
    for name in ["build:z", "build:b"] {
        let Err(Failure::Refused(why)) = launched(&fx.options(), name, "t-9") else {
            panic!("{name} wasn't refused");
        };
        assert!(why.contains(name), "{why}");
        assert_eq!(fs::read(&loop_file).unwrap(), before, "{name}");
    }
}

#[test]
fn the_launcher_names_the_run() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let actions = step(&fx.options()).unwrap();
    let path = line(&actions, "launch build:a ")
        .rsplit(' ')
        .next()
        .unwrap()
        .to_string();
    assert!(path.contains(".clm/prompts/loop/"), "{path}");
    let prompt = fs::read_to_string(&path).unwrap();
    assert_eq!(prompt, "loop run build:a\nlauncher for a\n");
}

#[test]
fn nothing_to_do_waits() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    let fx = Fixture::new();
    assert_eq!(step(&fx.options()).unwrap(), ["wait"]);
    assert!(fx.file().lease.is_some(), "every command renews the lease");
}

/// The fixture of "what the step does when a command fails": `a` and `b` ready, two places, and a
/// `clm run a` that exits 1.
fn two_ready_one_refused(fx: &Fixture) -> clm_loop::Options {
    let mut options = fx.options();
    options.max_builds = 2;
    fx.set("next", "a\nb\n");
    fx.set("run-exit-a", "1");
    fx.set("run-reason-a", "a has uncommitted changes\n");
    options
}

#[test]
fn what_the_step_does_when_a_command_fails() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    let fx = Fixture::new();
    let options = two_ready_one_refused(&fx);
    let actions = step(&options).unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("launch build:a ")),
        "{actions:?}"
    );
    line(&actions, "launch build:b ");

    // A `clm triage` that exits 1 fails the step with exit 1, naming it.
    fx.set("triage-exit", "1");
    let Err(Failure::Refused(why)) = step(&options) else {
        panic!("a refused clm triage didn't refuse the step");
    };
    assert!(why.contains("clm triage"), "{why}");
    fs::remove_file(fx.fake.join("triage-exit")).unwrap();

    // A `clm upgrade` that exits non-zero lists no migration.
    let migrations = |fx: &Fixture| -> Vec<String> {
        stored_plan(&fx.work)
            .unwrap()
            .waiting_on_you
            .into_iter()
            .map(|e| e.name)
            .filter(|n| n == "20261107-m")
            .collect()
    };
    fx.set("upgrade", "20261107-m required\n");
    step(&options).unwrap();
    assert_eq!(migrations(&fx), ["20261107-m"], "the fake lists it");
    fx.set("upgrade-exit", "1");
    step(&options).unwrap();
    assert!(migrations(&fx).is_empty());
}

#[test]
fn a_skipped_launch_is_named_on_stderr() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    // The step's stderr is this process's, so a child process runs the step and this reads it.
    if std::env::var_os("CLM_LOOP_SKIPPED_CHILD").is_some() {
        let fx = Fixture::new();
        let options = two_ready_one_refused(&fx);
        fx.set("triage", "group bug:x [x] P1 0 x1\n");
        fx.set("draft-exit", "1");
        step(&options).unwrap();
        return;
    }
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_skipped_launch_is_named_on_stderr",
            "--nocapture",
        ])
        .env("CLM_LOOP_SKIPPED_CHILD", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        stderr.contains("clm build: skipped build:a: a has uncommitted changes"),
        "{stderr}"
    );
    assert!(
        stderr.contains("clm build: skipped draft:bug:x: draft refused"),
        "{stderr}"
    );
}

#[test]
fn a_loop_that_ended() {
    // req:orchestrator.loop/step-actions@5e3bb1b7
    let fx = Fixture::new();
    start(&fx.options(), "opus", false).unwrap();
    stop(&fx.options(), StopMode::Drain).unwrap();
    let last = step(&fx.options()).unwrap();
    assert_eq!(last.last().map(String::as_str), Some("end"), "{last:?}");
    assert!(last[0].starts_with("say "), "{last:?}");
    // The step that printed `end` ended the lease: a `start` at once takes the loop.
    fx.set("next", "a\n");
    let first = start(&fx.options(), "opus", false).unwrap();
    line(&first, "launch build:a ");
}
