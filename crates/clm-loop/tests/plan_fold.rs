//! Folding the drafts that need review into one spec branch (`plan-fold`).

mod common;

use std::fs;

use clm_git::Failure;
use clm_loop::{Batch, Kind, RunState, StopMode, approved, returned, step, stop, stored_plan};
use common::{Fixture, line, run};

/// Drafts `x` and `y` returned for review, in that order, and `build:a` still running.
fn two_drafts_for_review(fx: &Fixture) {
    for (name, task) in [("draft:x", "t-1"), ("draft:y", "t-2")] {
        let mut d = run(name, Kind::Draft, "opus", RunState::Returned, Some(task));
        d.items = vec![format!("item-{}", &name[6..])];
        fx.add_run(d);
    }
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-3"),
    ));
    clm_loop::update(&fx.work, |f| {
        // Recorded in the order they returned, y first.
        f.review = vec!["y".to_string(), "x".to_string()];
        Ok(())
    })
    .unwrap();
}

#[test]
fn fold_while_builds_run() {
    // req:orchestrator.loop/plan-fold@fbfc873a
    let fx = Fixture::new();
    two_drafts_for_review(&fx);
    let actions = step(&fx.options()).unwrap();
    let launch = line(&actions, &common::launch_on_build_model("fold:x", "fold"));
    let prompt = fs::read_to_string(launch.rsplit(' ').next().unwrap()).unwrap();
    assert!(prompt.starts_with("loop run fold:x\n"), "{prompt}");
    assert!(
        prompt.contains("--fold x y"),
        "in the order their drafts launched: {prompt}"
    );
    assert!(fx.file().review.is_empty(), "the fold took them");
    // Not launched twice.
    let again = step(&fx.options()).unwrap();
    assert_eq!(again, [launch.to_string()]);
    assert_eq!(fx.ran("draft --fold x y"), 1);
    // It takes no place: nothing but the fold's own entry changed the places the plan sees.
    let view = stored_plan(&fx.work).unwrap();
    assert!(view.running.iter().any(|r| r.name == "fold:x"));
}

#[test]
fn no_fold_while_a_draft_runs_but_one_while_draining() {
    // req:orchestrator.loop/plan-fold@fbfc873a
    let fx = Fixture::new();
    two_drafts_for_review(&fx);
    fx.add_run(run(
        "draft:z",
        Kind::Draft,
        "opus",
        RunState::Running,
        Some("t-9"),
    ));
    let actions = step(&fx.options()).unwrap();
    assert!(!actions.iter().any(|l| l.contains("fold:")), "{actions:?}");
    fx.change_run("draft:z", |r| r.state = RunState::Returned);
    stop(&fx.options(), StopMode::Drain).unwrap();
    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch fold:x fold ");
}

#[test]
fn a_batch_goes_to_the_human_and_is_landed_once_approved() {
    // req:orchestrator.loop/plan-fold@fbfc873a
    let fx = Fixture::new();
    two_drafts_for_review(&fx);
    step(&fx.options()).unwrap();
    returned(
        &fx.options(),
        "fold:x",
        None,
        "proposed b review https://example.com/pr/2",
        None,
        None,
    )
    .unwrap();
    let queue = stored_plan(&fx.work).unwrap().waiting_on_you;
    assert_eq!(queue.len(), 1, "{queue:?}");
    assert_eq!(queue[0].name, "b");
    assert_eq!(queue[0].action, "say \"approve b\"");

    let actions = approved(&fx.options(), "b").unwrap();
    assert_eq!(line(&actions, "run land:b "), "run land:b clm land spec/b");
    assert!(approved(&fx.options(), "nope").is_err());
    // The same line until it is reported.
    let again = step(&fx.options()).unwrap();
    line(&again, "run land:b ");

    // The land reports gate/approval: back in the human's queue, with nothing said.
    let refused = returned(
        &fx.options(),
        "land:b",
        None,
        "clm land: refused: gate/approval: needs a human",
        None,
        None,
    )
    .unwrap();
    assert!(
        !refused.iter().any(|l| l.starts_with("say ")),
        "{refused:?}"
    );
    assert_eq!(stored_plan(&fx.work).unwrap().waiting_on_you.len(), 1);
    assert!(!fx.file().batches[0].approved);

    // Approved again, it lands, and each folded draft is finished.
    approved(&fx.options(), "b").unwrap();
    returned(
        &fx.options(),
        "land:b",
        None,
        "landed 1a2b3c4 spec",
        None,
        None,
    )
    .unwrap();
    assert_eq!(fx.ran("draft --finish x"), 1);
    assert_eq!(fx.ran("draft --finish y"), 1);
    assert!(fx.file().batches.is_empty());
}

#[test]
fn a_fold_that_proposes_nothing_is_told_to_the_session() {
    // req:orchestrator.loop/plan-fold@fbfc873a
    let fx = Fixture::new();
    two_drafts_for_review(&fx);
    step(&fx.options()).unwrap();
    let actions = returned(
        &fx.options(),
        "fold:x",
        None,
        "stopped: conflicts",
        None,
        None,
    )
    .unwrap();
    let said = line(&actions, "say ").strip_prefix("say ").unwrap();
    let text = fs::read_to_string(said).unwrap();
    assert!(text.contains("fold x"), "{text}");
    assert!(text.contains("conflicts"), "{text}");
    assert!(fx.file().batches.is_empty());
    let review = fx.file().review;
    assert!(
        !review.contains(&"x".to_string()) && !review.contains(&"y".to_string()),
        "{review:?}"
    );

    // An approved batch whose land fails is said and goes back to the human.
    clm_loop::update(&fx.work, |f| {
        f.batches.push(Batch {
            id: "b".to_string(),
            approved: false,
            drafts: vec!["x".to_string(), "y".to_string()],
            since: clm_git::now(),
        });
        Ok(())
    })
    .unwrap();
    approved(&fx.options(), "b").unwrap();
    let actions = returned(
        &fx.options(),
        "land:b",
        None,
        "stopped: check fails",
        None,
        None,
    )
    .unwrap();
    let said = line(&actions, "say ").strip_prefix("say ").unwrap();
    let text = fs::read_to_string(said).unwrap();
    assert!(text.contains("batch b"), "{text}");
    let queue = stored_plan(&fx.work).unwrap().waiting_on_you;
    assert!(queue.iter().any(|e| e.name == "b"), "{queue:?}");
    assert!(!fx.file().batches[0].approved);
}

#[test]
fn approval_of_an_unknown_batch() {
    // req:orchestrator.loop/plan-fold@fbfc873a
    let fx = Fixture::new();
    two_drafts_for_review(&fx);
    let file = fx.loop_dir().join("loop");
    let before = fs::read(&file).unwrap();
    let Err(Failure::Refused(why)) = approved(&fx.options(), "z") else {
        panic!("an unknown batch is refused");
    };
    assert!(why.contains("batch z"), "{why}");
    assert_eq!(
        fs::read(&file).unwrap(),
        before,
        "the loop file is unchanged"
    );
    assert!(
        !fx.calls().iter().any(|c| c.contains("land")),
        "{:?}",
        fx.calls()
    );
}
