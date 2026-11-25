//! `clm build returned` (`return-rules`).

mod common;

use clm_loop::{Kind, RunState, returned};
use common::{Fixture, line, run};

fn running(fx: &Fixture, name: &str, kind: Kind, task: &str) {
    fx.add_run(run(name, kind, "sonnet", RunState::Running, Some(task)));
}

#[test]
fn waiting() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    let actions = returned(
        &fx.options(),
        "build:a",
        None,
        "waiting: clm land impl/a",
        None,
        None,
    )
    .unwrap();
    assert_eq!(actions, ["wait"]);
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Running);
    assert!(
        !fx.calls().iter().any(|c| c.starts_with("run --finish")),
        "nothing is freed"
    );
}

#[test]
fn landed_with_a_task_left_running() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-17");
    fx.set("finish-a", "landed impl/a\n");
    let actions = returned(
        &fx.options(),
        "build:a",
        None,
        "landed 1a2b3c4",
        Some("running"),
        None,
    )
    .unwrap();
    assert_eq!(actions[0], "stop build:a t-17");
    assert_eq!(fx.ran("run --finish a --returned"), 1);
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
}

#[test]
fn no_closing_line_landed() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    fx.set("finish-a", "landed impl/a\n");
    returned(
        &fx.options(),
        "build:a",
        None,
        "none",
        Some("context"),
        None,
    )
    .unwrap();
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
    assert!(run.stops.is_empty(), "a landed run records no stop");
}

#[test]
fn no_closing_line_not_landed() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    returned(
        &fx.options(),
        "build:a",
        None,
        "none",
        Some("context"),
        None,
    )
    .unwrap();
    let run = fx.file().run("build:a").unwrap().clone();
    assert!(!run.landed);
    assert_eq!(run.stops.get("context"), Some(&1));
    // A first `context` stop finishes the run and rebuilds it.
    assert_eq!(fx.ran("run --finish a --returned"), 1);
}

#[test]
fn stopped_keeps_its_place() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    returned(
        &fx.options(),
        "build:a",
        None,
        "none",
        Some("stopped"),
        None,
    )
    .unwrap();
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Stopped);
    assert!(
        !fx.calls().iter().any(|c| c.starts_with("run --finish")),
        "{:?}",
        fx.calls()
    );
}

#[test]
fn a_draft_for_review_joins_the_fold_list() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "draft:d1", Kind::Draft, "t-1");
    // Another draft still runs, so the fold list waits.
    running(&fx, "draft:d9", Kind::Draft, "t-9");
    fx.set("finish-d1", "left slug-x: needs the human\n");
    returned(
        &fx.options(),
        "draft:d1",
        None,
        "proposed d1 review https://example.com/pr/1",
        None,
        None,
    )
    .unwrap();
    assert_eq!(fx.ran("draft --finish d1"), 1);
    let file = fx.file();
    assert_eq!(file.review, ["d1"]);
    assert_eq!(file.drafted, ["slug-x"]);
}

#[test]
fn a_build_finish_passes_on_what_it_left() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    fx.set(
        "finish-a",
        "landed impl/a\nleft /w/agent-1: delete the directory by hand\n",
    );
    let actions = returned(&fx.options(), "build:a", None, "landed abc", None, None).unwrap();
    let say = line(&actions, "say ");
    let text = std::fs::read_to_string(say.strip_prefix("say ").unwrap()).unwrap();
    assert_eq!(text, "left /w/agent-1: delete the directory by hand\n");
}

#[test]
fn an_unknown_run_is_refused() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    let before = fx.file();
    let Err(clm_git::Failure::Refused(why)) =
        returned(&fx.options(), "build:nope", None, "none", None, None)
    else {
        panic!("a run the loop doesn't list is refused");
    };
    assert!(why.contains("build:nope"), "{why}");
    assert!(
        !fx.calls().iter().any(|c| c.starts_with("run --finish")),
        "{:?}",
        fx.calls()
    );
    unchanged_but_for_the_lease(before, fx.file());
}

#[test]
fn a_closing_line_decides_landed_not_the_finish() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    fx.set("finish-a", "landed impl/a\n");
    returned(
        &fx.options(),
        "build:a",
        None,
        "stopped: check fails",
        None,
        None,
    )
    .unwrap();
    assert!(!fx.file().run("build:a").unwrap().landed);
    running(&fx, "draft:d2", Kind::Draft, "t-2");
    running(&fx, "draft:d9", Kind::Draft, "t-9");
    returned(
        &fx.options(),
        "draft:d2",
        None,
        "proposed d2 review",
        None,
        None,
    )
    .unwrap();
    assert_eq!(fx.file().review, ["d2"]);
}

#[test]
fn a_late_report_of_a_run_the_loop_finished() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    fx.land_file("c", "c\n");
    let c = clm_git::git(&fx.work, &["rev-parse", "origin/main"]).unwrap();
    running(&fx, "build:a", Kind::Build, "t-2");
    // The run started after the commit was made.
    fx.change_run("build:a", |r| r.started = clm_git::now() + 300);

    let actions = returned(
        &fx.options(),
        "build:a",
        None,
        &format!("landed {c}"),
        Some("running"),
        None,
    )
    .unwrap();

    assert!(
        !fx.calls().iter().any(|c| c.starts_with("run --finish")),
        "{:?}",
        fx.calls()
    );
    assert!(
        !actions.iter().any(|a| a.starts_with("stop ")),
        "{actions:?}"
    );
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Running);
}

#[test]
fn a_report_of_a_commit_made_since_the_run_started_is_finished() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    fx.land_file("c", "c\n");
    let c = clm_git::git(&fx.work, &["rev-parse", "origin/main"]).unwrap();
    running(&fx, "build:a", Kind::Build, "t-2");
    fx.change_run("build:a", |r| r.started = clm_git::now() - 600);
    fx.set("finish-a", "landed impl/a\n");

    let actions = returned(
        &fx.options(),
        "build:a",
        None,
        &format!("landed {c}"),
        Some("running"),
        None,
    )
    .unwrap();

    assert_eq!(actions[0], "stop build:a t-2");
    assert_eq!(fx.ran("run --finish a --returned"), 1);
    assert!(fx.file().run("build:a").unwrap().landed);
}

/// A commit on `origin/main` made after `build:a` started, as a build's `landed` line names one.
fn a_commit_since_the_run_started(fx: &Fixture) -> String {
    fx.land_file("c", "c\n");
    let c = clm_git::git(&fx.work, &["rev-parse", "origin/main"]).unwrap();
    fx.change_run("build:a", |r| r.started = clm_git::now() - 600);
    c.trim().to_string()
}

#[test]
fn a_return_finishes_as_returned() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    let sha = a_commit_since_the_run_started(&fx);
    fx.set("finish-a", "landed impl/a\n");

    returned(
        &fx.options(),
        "build:a",
        None,
        &format!("landed {sha}"),
        None,
        None,
    )
    .unwrap();

    assert_eq!(fx.ran("run --finish a --returned"), 1);
    assert_eq!(fx.ran("run --finish a"), 0, "{:?}", fx.calls());
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
}

/// The `build:a` with task `t-1` that a step finished as returned and landed once its claim had
/// left `origin`, its finish printing a `left` line (`claim-gone-ends-a-build`); the commit its
/// report names.
fn finished_by_the_loop(fx: &Fixture) -> String {
    running(fx, "build:a", Kind::Build, "t-1");
    let sha = a_commit_since_the_run_started(fx);
    fx.drop_claim("a");
    fx.set(
        "finish-a",
        "landed impl/a\nleft /w/agent-1: busy; delete the directory by hand, then run git worktree prune\n",
    );
    clm_loop::step(&fx.options()).unwrap();
    assert_eq!(fx.ran("run --finish a"), 1, "the step's finish passes none");
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
    sha
}

#[test]
fn a_return_after_the_loop_finished_the_build() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    let sha = finished_by_the_loop(&fx);
    let before = fx.file();

    let actions = returned(
        &fx.options(),
        "build:a",
        Some("t-1"),
        &format!("landed {sha}"),
        Some("running"),
        None,
    )
    .unwrap();

    assert_eq!(fx.ran("run --finish a --returned"), 1);
    assert!(
        !actions
            .iter()
            .any(|a| a.starts_with("stop ") || a.starts_with("resume ")),
        "{actions:?}"
    );
    let say = line(&actions, "say ");
    let text = std::fs::read_to_string(say.strip_prefix("say ").unwrap()).unwrap();
    assert!(
        text.contains("left /w/agent-1: busy; delete the directory by hand"),
        "{text}"
    );
    let after = unchanged_but_for_the_lease(before, fx.file());
    let run = after.run("build:a").unwrap();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
    assert!(run.stops.is_empty());
}

/// The text of each `say` among `actions`.
fn says(actions: &[String]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|a| a.strip_prefix("say "))
        .map(|f| std::fs::read_to_string(f).unwrap())
        .collect()
}

#[test]
fn a_worktree_the_returns_finish_keeps() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "build:a", Kind::Build, "t-1");
    let sha = a_commit_since_the_run_started(&fx);
    fx.drop_claim("a");
    fx.set(
        "finish-a",
        "landed impl/a\nkept /w/a: a running process holds its lock\n",
    );
    fx.set("finish-a-returned", "kept /w/a: uncommitted changes\n");

    // The step's finish passes no `--returned`: what it keeps, the return's finish removes.
    let step = clm_loop::step(&fx.options()).unwrap();
    assert_eq!(fx.ran("run --finish a"), 1, "{:?}", fx.calls());
    assert!(!says(&step).iter().any(|s| s.contains("kept")), "{step:?}");

    let back = returned(
        &fx.options(),
        "build:a",
        Some("t-1"),
        &format!("landed {sha}"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(fx.ran("run --finish a --returned"), 1);
    assert_eq!(says(&back), ["kept /w/a: uncommitted changes\n"]);

    // A running build's first return says what its `--returned` finish kept.
    running(&fx, "build:b", Kind::Build, "t-2");
    fx.change_run("build:b", |r| r.started = clm_git::now() - 600);
    fx.set(
        "finish-b-returned",
        "landed impl/b\nkept /w/b: uncommitted changes\n",
    );
    let first = returned(
        &fx.options(),
        "build:b",
        Some("t-2"),
        &format!("landed {sha}"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(says(&first), ["kept /w/b: uncommitted changes\n"]);

    // So does a draft's return, whose `clm draft --finish` keeps a worktree.
    running(&fx, "draft:d", Kind::Draft, "t-3");
    running(&fx, "draft:e", Kind::Draft, "t-4");
    fx.set("finish-d", "kept /w/d: uncommitted changes\n");
    let draft = returned(
        &fx.options(),
        "draft:d",
        Some("t-3"),
        "proposed d review",
        None,
        None,
    )
    .unwrap();
    assert_eq!(says(&draft), ["kept /w/d: uncommitted changes\n"]);
}

#[test]
fn every_draft_finish_says_what_it_kept() {
    // req:orchestrator.loop/return-rules@3c0219a4: the drafts a landed fold batch held, and a
    // draft whose relaunch is refused, are finished with `clm draft --finish`, whose `kept`
    // lines are said.
    let fx = Fixture::new();
    clm_loop::update(&fx.work, |f| {
        f.batches.push(clm_loop::Batch {
            id: "b".to_string(),
            approved: true,
            drafts: vec!["x".to_string()],
            since: clm_git::now(),
            rebase: Default::default(),
        });
        Ok(())
    })
    .unwrap();
    let mut land = run("land:b", Kind::Fold, "sonnet", RunState::Running, None);
    land.kind = None;
    land.command = Some("clm land spec/b".to_string());
    fx.add_run(land);
    fx.set("finish-x", "kept /w/x: uncommitted changes\n");
    let landed = returned(&fx.options(), "land:b", None, "landed 1a2b3c4", None, None).unwrap();
    assert_eq!(fx.ran("draft --finish x"), 1);
    assert_eq!(says(&landed), ["kept /w/x: uncommitted changes\n"]);

    fx.add_run(run(
        "draft:d",
        Kind::Draft,
        "opus",
        RunState::Running,
        Some("t-1"),
    ));
    fx.set_lease(20);
    fx.set("resume-exit-d", "1");
    fx.set("resume-reason-d", "nothing to resume\n");
    fx.set("finish-d", "kept /w/d: uncommitted changes\n");
    let started = clm_loop::start(&fx.options(), "opus", false).unwrap();
    assert!(
        says(&started).contains(&"kept /w/d: uncommitted changes\n".to_string()),
        "{started:?}"
    );
}

#[test]
fn a_drafts_return_reported_twice() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    running(&fx, "draft:d", Kind::Draft, "t-1");
    // Another draft still runs, so no fold is due.
    running(&fx, "draft:e", Kind::Draft, "t-2");
    let report = || {
        returned(
            &fx.options(),
            "draft:d",
            None,
            "proposed d review",
            None,
            None,
        )
        .unwrap()
    };
    report();
    // A flag the repeated report mustn't clear: it changes nothing but the lease.
    fx.change_run("draft:d", |r| r.resumed = true);
    let before = fx.file();
    report();
    assert_eq!(fx.ran("draft --finish d"), 1, "{:?}", fx.calls());
    let after = unchanged_but_for_the_lease(before, fx.file());
    assert_eq!(after.review, ["d"]);
}

#[test]
fn a_landed_run_in_the_background_with_no_task() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Planned,
        None,
    ));
    let sha = a_commit_since_the_run_started(&fx);
    // A resume the run owes: the refusal mustn't clear it.
    fx.change_run("build:a", |r| r.resumed = true);
    let before = fx.file();

    let Err(clm_git::Failure::Refused(why)) = returned(
        &fx.options(),
        "build:a",
        None,
        &format!("landed {sha}"),
        Some("running"),
        None,
    ) else {
        panic!("a landed run with no task is refused");
    };

    assert!(why.contains("clm build launched"), "{why}");
    assert_eq!(fx.finishes("a"), 0, "{:?}", fx.calls());
    unchanged_but_for_the_lease(before, fx.file());

    // Whatever the run's kind: a `land:` run with no task is refused alike.
    let mut land = run("land:b", Kind::Fold, "sonnet", RunState::Running, None);
    land.kind = None;
    land.command = Some("clm land spec/b".to_string());
    fx.add_run(land);
    let before = fx.file();
    let refused = returned(
        &fx.options(),
        "land:b",
        None,
        "landed 1a2b3c4",
        Some("running"),
        None,
    );
    assert!(
        matches!(&refused, Err(clm_git::Failure::Refused(why)) if why.contains("clm build launched")),
        "{refused:?}"
    );
    unchanged_but_for_the_lease(before, fx.file());
}

/// `after`, once checked equal to `before` but for the lease and the plan the actions were
/// computed from.
fn unchanged_but_for_the_lease(
    mut before: clm_loop::LoopFile,
    mut after: clm_loop::LoopFile,
) -> clm_loop::LoopFile {
    for file in [&mut before, &mut after] {
        file.lease = None;
        file.plan = None;
    }
    assert_eq!(after, before, "nothing but the lease changes");
    after
}

#[test]
fn still_waiting_after_the_loop_finished_the_build() {
    // req:orchestrator.loop/return-rules@3c0219a4
    let fx = Fixture::new();
    finished_by_the_loop(&fx);
    let before = fx.file();

    let actions = returned(
        &fx.options(),
        "build:a",
        Some("t-1"),
        "waiting: clm land impl/a",
        None,
        None,
    )
    .unwrap();

    assert_eq!(fx.ran("run --finish a --returned"), 0);
    assert_eq!(fx.ran("run --finish a"), 1, "only the step's");
    assert!(
        !actions.iter().any(|a| a.starts_with("say ")),
        "{actions:?}"
    );
    let after = unchanged_but_for_the_lease(before, fx.file());
    let run = after.run("build:a").unwrap();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
    assert!(run.stops.is_empty());
}
