//! Running the slow tests after lands (`plan-verify`).

mod common;

use clm_loop::{Kind, RunState, StopMode, returned, step, stop, stored_plan};
use common::{Fixture, line, run};

fn queue(fx: &Fixture) -> Vec<(String, String)> {
    stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .into_iter()
        .map(|e| (e.name, e.action))
        .collect()
}

#[test]
fn idle_and_due() {
    // req:orchestrator.loop/plan-verify@2e8f46fd
    let fx = Fixture::new();
    fx.set("verify-due", "verify due work-done\n");
    let actions = step(&fx.options()).unwrap();
    assert_eq!(
        line(&actions, "run verify "),
        "run verify clm verify --due --event work-done"
    );
    assert_eq!(fx.ran("verify --due --dry-run --event work-done"), 1);
    // The same line until the run is reported.
    let again = step(&fx.options()).unwrap();
    line(&again, "run verify ");
    assert_eq!(fx.ran("verify --due --dry-run --event work-done"), 1);
}

#[test]
fn not_due_runs_nothing() {
    // req:orchestrator.loop/plan-verify@2e8f46fd
    let fx = Fixture::new();
    // The fake exits 3, as `clm verify --due --dry-run` does when nothing is due.
    let actions = step(&fx.options()).unwrap();
    assert_eq!(actions, ["wait"]);
    // A start-of-stdout other than `verify due` runs nothing either.
    fx.set("verify-due", "verify not-due no trigger fired\n");
    assert_eq!(step(&fx.options()).unwrap(), ["wait"]);
}

#[test]
fn after_a_landed_run_while_others_still_go() {
    // req:orchestrator.loop/plan-verify@2e8f46fd
    let fx = Fixture::new();
    fx.set("verify-due", "verify due change-done\n");
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.add_run(run(
        "build:b",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-2"),
    ));
    // Nothing landed yet and the loop isn't idle: no check at all.
    step(&fx.options()).unwrap();
    assert_eq!(fx.ran("verify --due --dry-run"), 0);
    fx.set("finish-a", "landed impl/a\n");
    let actions = returned(&fx.options(), "build:a", "landed 1a2b3c4", None, None).unwrap();
    assert_eq!(line(&actions, "run verify "), "run verify clm verify --due");
    assert_eq!(fx.ran("verify --due --dry-run"), 1, "no event: not idle");
}

#[test]
fn also_while_draining() {
    // req:orchestrator.loop/plan-verify@2e8f46fd
    let fx = Fixture::new();
    fx.set("verify-due", "verify due work-done\n");
    stop(&fx.options(), StopMode::Drain).unwrap();
    let actions = step(&fx.options()).unwrap();
    line(&actions, "run verify ");
}

#[test]
fn a_red_result_in_sight() {
    // req:orchestrator.loop/plan-verify@2e8f46fd
    let fx = Fixture::new();
    fx.set("verify-due", "verify due work-done\n");
    step(&fx.options()).unwrap();
    returned(
        &fx.options(),
        "verify",
        "verify failed 1a2b3c4 culprit 9f8e7d6 components a",
        None,
        None,
    )
    .unwrap();
    // Not drafted, and the human's queue holds it.
    assert_eq!(
        queue(&fx),
        [("verify".to_string(), "take in with /clm:triage".to_string())]
    );
    // An item on main that names its commit takes it in.
    fx.land_file(
        "backlog/slow-test-broke.md",
        "---\ntitle: a slow test broke\nkind: bug\ncomponents: [a]\nverify_failures: [1a2b3c4000000000000000000000000000000000]\n---\n\nIt broke.\n",
    );
    step(&fx.options()).unwrap();
    assert_eq!(queue(&fx), []);
}
