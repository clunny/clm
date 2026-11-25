//! A fold batch whose land fails gets one rebase run per approval (`failed-batch-land-rebased`).

mod common;

use std::fs;

use clm_loop::{Batch, Kind, RunState, approved, launched, returned, start, step, stored_plan};
use common::{Fixture, line, run};

/// The batch `b`, folded from drafts `x` and `y`, approved, with `land:b` printed.
fn approved_batch(fx: &Fixture) {
    clm_loop::update(&fx.work, |f| {
        f.batches.push(Batch {
            id: "b".to_string(),
            approved: false,
            drafts: vec!["x".to_string(), "y".to_string()],
            since: clm_git::now(),
            rebase: Default::default(),
        });
        Ok(())
    })
    .unwrap();
    let actions = approved(&fx.options(), "b").unwrap();
    assert_eq!(line(&actions, "run land:b "), "run land:b clm land spec/b");
}

fn says(actions: &[String]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|l| l.strip_prefix("say "))
        .map(|p| fs::read_to_string(p).unwrap())
        .collect()
}

fn queued(fx: &Fixture) -> bool {
    stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .iter()
        .any(|e| e.name == "b")
}

const MERGE_FAILED: &str = "clm land: the check of the merge failed";

/// The approved batch `b` whose first land failed, with `rebase:b` launched and reported
/// running; the actions of the failed land.
fn rebase_running(fx: &Fixture) -> Vec<String> {
    approved_batch(fx);
    let actions = returned(&fx.options(), "land:b", None, MERGE_FAILED, None, None).unwrap();
    launched(&fx.options(), "rebase:b", "t-r").unwrap();
    actions
}

#[test]
fn a_land_that_fails_after_main_moved() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce
    let fx = Fixture::new();
    let actions = rebase_running(&fx);
    let launch = line(&actions, &common::launch_on_build_model("rebase:b", "fold"));
    let prompt = fs::read_to_string(launch.rsplit(' ').next().unwrap()).unwrap();
    assert!(prompt.starts_with("loop run rebase:b\n"), "{prompt}");
    assert!(
        prompt.contains("git switch -c spec/b origin/spec/b"),
        "{prompt}"
    );
    assert!(prompt.contains("clm rebase"), "{prompt}");
    assert!(prompt.contains("clm branch push"), "{prompt}");
    assert!(prompt.contains("proposed b review"), "{prompt}");
    assert!(!prompt.contains("clm land"), "{prompt}");
    assert!(!prompt.contains("clm approve"), "{prompt}");
    assert!(says(&actions).is_empty(), "{actions:?}");
    assert!(
        !actions.iter().any(|l| l.starts_with("run land:b")),
        "{actions:?}"
    );
    assert!(!queued(&fx));
    assert!(fx.file().batches[0].approved, "the batch stays approved");
    // While the rebase runs, no land is printed, and the rebase isn't launched again.
    let again = step(&fx.options()).unwrap();
    assert!(
        !again
            .iter()
            .any(|l| l.starts_with("run land:b") || l.starts_with("launch rebase:b")),
        "{again:?}"
    );
}

#[test]
fn brought_up_to_date() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce
    let fx = Fixture::new();
    rebase_running(&fx);
    let actions = returned(
        &fx.options(),
        "rebase:b",
        None,
        "proposed b review https://example.com/pull/7",
        None,
        None,
    )
    .unwrap();
    assert_eq!(line(&actions, "run land:b "), "run land:b clm land spec/b");
    let batches = fx.file().batches;
    assert_eq!(batches.len(), 1, "{batches:?}");
    assert_eq!(batches[0].id, "b");
    assert!(batches[0].approved);
    assert!(!queued(&fx));
}

#[test]
fn a_bare_proposed_line_lands_again() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce: with or without a URL.
    let fx = Fixture::new();
    rebase_running(&fx);
    let actions = returned(
        &fx.options(),
        "rebase:b",
        None,
        "proposed b review",
        None,
        None,
    )
    .unwrap();
    line(&actions, "run land:b ");
    assert_eq!(fx.file().batches.len(), 1);
}

#[test]
fn a_fresh_approval_needed() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce
    let fx = Fixture::new();
    rebase_running(&fx);
    returned(
        &fx.options(),
        "rebase:b",
        None,
        "proposed b review",
        None,
        None,
    )
    .unwrap();
    let actions = returned(
        &fx.options(),
        "land:b",
        None,
        "clm land: gate/approval: not approved",
        None,
        None,
    )
    .unwrap();
    let said = says(&actions);
    assert_eq!(said.len(), 1, "{actions:?}");
    assert!(said[0].contains("fold batch b"), "{said:?}");
    assert!(said[0].contains("approve it again"), "{said:?}");
    assert!(
        !actions.iter().any(|l| l.starts_with("launch rebase:b")),
        "{actions:?}"
    );
    assert!(queued(&fx));
    assert!(!fx.file().batches[0].approved);
}

#[test]
fn a_rebase_that_cant_finish() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce
    let fx = Fixture::new();
    rebase_running(&fx);
    let actions = returned(
        &fx.options(),
        "rebase:b",
        None,
        "stopped: conflicts",
        None,
        None,
    )
    .unwrap();
    let said = says(&actions);
    assert_eq!(said.len(), 1, "{actions:?}");
    assert_eq!(
        said[0].trim_end(),
        "fold batch b couldn't be brought up to date with main (stopped: conflicts); it is back in your queue"
    );
    assert!(
        !actions.iter().any(|l| l.starts_with("run land:b")),
        "{actions:?}"
    );
    assert!(queued(&fx));
    assert!(!fx.file().batches[0].approved);
    assert_eq!(fx.file().batches.len(), 1, "no batch added");
}

#[test]
fn a_new_approval_a_new_rebase() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce
    let fx = Fixture::new();
    rebase_running(&fx);
    returned(
        &fx.options(),
        "rebase:b",
        None,
        "proposed b review",
        None,
        None,
    )
    .unwrap();
    returned(
        &fx.options(),
        "land:b",
        None,
        "clm land: gate/approval: not approved",
        None,
        None,
    )
    .unwrap();
    approved(&fx.options(), "b").unwrap();
    let actions = returned(&fx.options(), "land:b", None, MERGE_FAILED, None, None).unwrap();
    line(&actions, &common::launch_on_build_model("rebase:b", "fold"));
}

#[test]
fn a_second_failure_before_the_next_approval_goes_back_to_the_human() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce: one rebase run per approval.
    let fx = Fixture::new();
    rebase_running(&fx);
    returned(
        &fx.options(),
        "rebase:b",
        None,
        "proposed b review",
        None,
        None,
    )
    .unwrap();
    let actions = returned(&fx.options(), "land:b", None, MERGE_FAILED, None, None).unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("launch rebase:b")),
        "{actions:?}"
    );
    let said = says(&actions);
    assert_eq!(said.len(), 1, "{actions:?}");
    assert!(
        said[0].contains("the land of fold batch b failed"),
        "{said:?}"
    );
    assert!(queued(&fx));
}

#[test]
fn a_rebase_rebuilt_goes_back_to_the_human() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce: a stop `stop-causes` answers
    // with a rebuild launches no second rebase run.
    let fx = Fixture::new();
    rebase_running(&fx);
    let actions = returned(
        &fx.options(),
        "rebase:b",
        None,
        "context window full",
        Some("context"),
        None,
    )
    .unwrap();
    assert!(
        !actions
            .iter()
            .any(|l| l.starts_with("launch rebase:b") || l.starts_with("run land:b")),
        "{actions:?}"
    );
    let said = says(&actions);
    assert_eq!(said.len(), 1, "{actions:?}");
    assert!(
        said[0].contains("fold batch b couldn't be brought up to date with main"),
        "{said:?}"
    );
    assert!(queued(&fx));
    let file = fx.file();
    let rebase = file.runs.iter().find(|r| r.name == "rebase:b").unwrap();
    assert_eq!(rebase.state, RunState::Returned);
    assert!(!rebase.orphan);
}

#[test]
fn a_rebase_orphan_goes_back_to_the_human() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce: an orphan `loop-start` drops
    // isn't launched again before the next approval.
    let fx = Fixture::new();
    rebase_running(&fx);
    fx.set_lease(20);
    let actions = start(&fx.options(), "opus", false).unwrap();
    assert!(
        !actions
            .iter()
            .any(|l| l.starts_with("launch rebase:b") || l.starts_with("run land:b")),
        "{actions:?}"
    );
    let said = says(&actions);
    assert!(
        said.iter()
            .any(|s| s.contains("fold batch b couldn't be brought up to date with main")),
        "{said:?}"
    );
    assert!(queued(&fx));
    assert!(!fx.file().runs.iter().any(|r| r.name == "rebase:b"));
    let again = step(&fx.options()).unwrap();
    assert!(
        !again.iter().any(|l| l.starts_with("launch rebase:b")),
        "{again:?}"
    );
}

#[test]
fn a_running_rebase_holds_back_no_fold() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce
    let fx = Fixture::new();
    rebase_running(&fx);
    let mut d = run(
        "draft:x",
        Kind::Draft,
        "opus",
        RunState::Returned,
        Some("t-1"),
    );
    d.items = vec!["item-x".to_string()];
    fx.add_run(d);
    clm_loop::update(&fx.work, |f| {
        f.review = vec!["x".to_string()];
        Ok(())
    })
    .unwrap();
    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch fold:x fold ");
}

#[test]
fn launched_while_draining_and_counted_when_printed() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce: also while a drain is
    // requested, and the one launch is counted when it is printed.
    let fx = Fixture::new();
    approved_batch(&fx);
    clm_loop::stop(&fx.options(), clm_loop::StopMode::Drain).unwrap();
    let actions = returned(&fx.options(), "land:b", None, MERGE_FAILED, None, None).unwrap();
    line(&actions, &common::launch_on_build_model("rebase:b", "fold"));
    assert_eq!(fx.file().batches[0].rebase, clm_loop::Rebase::Launched);
}

#[test]
fn a_proposal_for_another_batch_brings_nothing_up_to_date() {
    // req:orchestrator.loop/failed-batch-land-rebased@a017d9ce: only `proposed b review` lands
    // `b` again; any other line puts it back in the human's queue.
    let fx = Fixture::new();
    rebase_running(&fx);
    let actions = returned(
        &fx.options(),
        "rebase:b",
        None,
        "proposed c review",
        None,
        None,
    )
    .unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("run land:b")),
        "{actions:?}"
    );
    assert_eq!(says(&actions).len(), 1, "{actions:?}");
    assert!(queued(&fx));
    assert_eq!(fx.file().batches.len(), 1, "no batch added");
}
