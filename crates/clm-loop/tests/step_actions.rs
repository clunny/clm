//! `clm build step` and `launched` (`step-actions`). The `clm build` dispatch and the capability
//! test that drains its queue through it are in `crates/clm`.

mod common;

use std::fs;

use clm_loop::{RunState, launched, step};
use common::{Fixture, line};

#[test]
fn an_unreported_launch() {
    // req:orchestrator.loop/step-actions@c4f86a80
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let first = step(&fx.options()).unwrap();
    let launch = line(&first, "launch build:a build sonnet ").to_string();
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
    // req:orchestrator.loop/step-actions@c4f86a80
    let fx = Fixture::new();
    fx.set("next", "a\n");
    step(&fx.options()).unwrap();
    assert_eq!(fx.file().runs[0].state, RunState::Planned);
    launched(&fx.options(), "build:a", "t-17").unwrap();
    let run = fx.file().runs[0].clone();
    assert_eq!(run.state, RunState::Running);
    assert_eq!(run.task_id.as_deref(), Some("t-17"));
    // A run the loop doesn't list is refused.
    assert!(launched(&fx.options(), "build:zz", "t-1").is_err());
}

#[test]
fn the_launcher_names_the_run() {
    // req:orchestrator.loop/step-actions@c4f86a80
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
    // req:orchestrator.loop/step-actions@c4f86a80
    let fx = Fixture::new();
    assert_eq!(step(&fx.options()).unwrap(), ["wait"]);
    assert!(fx.file().lease.is_some(), "every command renews the lease");
}
