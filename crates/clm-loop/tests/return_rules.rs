//! `clm build returned` (`return-rules`).

mod common;

use clm_loop::{Kind, RunState, returned};
use common::{Fixture, line, run};

fn running(fx: &Fixture, name: &str, kind: Kind, task: &str) {
    fx.add_run(run(name, kind, "sonnet", RunState::Running, Some(task)));
}

#[test]
fn waiting() {
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    assert_eq!(fx.ran("run --finish a"), 1);
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.state, RunState::Returned);
    assert!(run.landed);
}

#[test]
fn no_closing_line_landed() {
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    assert_eq!(fx.ran("run --finish a"), 1);
}

#[test]
fn stopped_keeps_its_place() {
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    // req:orchestrator.loop/return-rules@69ce6fa7
    let fx = Fixture::new();
    assert!(returned(&fx.options(), "build:nope", None, "none", None, None).is_err());
}

#[test]
fn a_closing_line_decides_landed_not_the_finish() {
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    // req:orchestrator.loop/return-rules@69ce6fa7
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

    assert_eq!(fx.ran("run --finish a"), 0);
    assert!(
        !actions.iter().any(|a| a.starts_with("stop ")),
        "{actions:?}"
    );
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Running);
}

#[test]
fn a_report_of_a_commit_made_since_the_run_started_is_finished() {
    // req:orchestrator.loop/return-rules@69ce6fa7
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
    assert_eq!(fx.ran("run --finish a"), 1);
    assert!(fx.file().run("build:a").unwrap().landed);
}
