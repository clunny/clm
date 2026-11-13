//! What `clm build returned` does with a run that stopped, by its cause (`stop-causes`).

mod common;

use std::fs;

use clm_loop::{Kind, RunState, cleared, returned, step, stored_plan};
use common::{Fixture, line, rfc3339, run};

fn running(fx: &Fixture, name: &str, task: &str) {
    fx.add_run(run(
        name,
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some(task),
    ));
}

fn stop(fx: &Fixture, name: &str, result: &str, cause: &str, until: Option<&str>) -> Vec<String> {
    returned(&fx.options(), name, result, Some(cause), until).unwrap()
}

#[test]
fn a_limit() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    running(&fx, "build:a", "t-1");
    let until = rfc3339(clm_git::now() + 3600);
    let first = stop(&fx, "build:a", "none", "limit", Some(&until));
    assert_eq!(first, ["wait"], "no action for the stopped run");
    assert_eq!(fx.ran("run --finish a"), 0, "it keeps its place");
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Stopped);
    assert!(fx.file().conditions.limit_until.is_some());

    // The limit lifts.
    fx.change_run("build:a", |r| {
        r.resume.as_mut().unwrap().after = Some(clm_git::now() - 1);
    });
    let second = step(&fx.options()).unwrap();
    let resume = line(&second, "resume build:a t-1 ");
    let message = fs::read_to_string(resume.rsplit(' ').next().unwrap()).unwrap();
    assert!(message.contains("usage limit"), "{message}");
    assert!(message.contains("own worktree"), "{message}");
    assert!(message.contains("review or check subagent"), "{message}");
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Running);
}

#[test]
fn a_limit_resumes_each_time_it_recurs() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    running(&fx, "build:a", "t-1");
    for _ in 0..3 {
        let until = rfc3339(clm_git::now() - 60);
        let actions = stop(&fx, "build:a", "none", "limit", Some(&until));
        line(&actions, "resume build:a t-1 ");
    }
    assert_eq!(fx.file().run("build:a").unwrap().stops["limit"], 3);
    assert_eq!(fx.ran("run --finish a"), 0, "a limit never rebuilds");
}

#[test]
fn a_full_disk_cleared() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    running(&fx, "build:a", "t-1");
    stop(&fx, "build:a", "none", "disk", None);
    let actions = step(&fx.options()).unwrap();
    assert!(
        !actions.iter().any(|l| l.starts_with("resume ")),
        "{actions:?}"
    );
    let names: Vec<String> = stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert!(names.contains(&"disk".to_string()), "{names:?}");
    let after = cleared(&fx.options(), "disk").unwrap();
    line(&after, "resume build:a t-1 ");
    assert!(cleared(&fx.options(), "the moon").is_err());
}

#[test]
fn an_outage_resumes_at_once_the_first_time() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    fx.session("opus");
    running(&fx, "build:a", "t-1");
    let first = stop(&fx, "build:a", "stopped: network", "outage", None);
    line(&first, "resume build:a t-1 ");
    assert_eq!(fx.ran("run --finish a"), 0);
    // The second stop by the same cause rebuilds.
    let second = stop(&fx, "build:a", "stopped: network", "outage", None);
    assert_eq!(fx.ran("run --finish a"), 1);
    line(&second, "launch build:a build opus ");
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!(run.round, 2);
}

#[test]
fn a_second_work_stop() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    fx.session("opus");
    let mut a = run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    );
    a.stops.insert("work".to_string(), 1);
    fx.add_run(a);
    let actions = stop(&fx, "build:a", "stopped: check fails", "work", None);
    assert_eq!(fx.ran("run --finish a"), 1);
    line(&actions, "launch build:a build opus ");
    let run = fx.file().run("build:a").unwrap().clone();
    assert_eq!((run.round, run.stops["work"]), (2, 2));
    assert_eq!(run.rebuilt_for.as_deref(), Some("work"));
}

#[test]
fn a_first_context_stop_rebuilds() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    fx.session("opus");
    running(&fx, "build:a", "t-1");
    let actions = stop(&fx, "build:a", "none", "context", None);
    line(&actions, "launch build:a build opus ");
}

#[test]
fn an_undelivered_resume_rebuilds() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    fx.session("opus");
    running(&fx, "build:a", "t-1");
    // A work stop is resumed; the session then reports the resume couldn't be delivered.
    stop(&fx, "build:a", "stopped: x", "work", None);
    assert!(fx.file().run("build:a").unwrap().resumed);
    let actions = stop(&fx, "build:a", "none", "tool", None);
    assert_eq!(fx.ran("run --finish a"), 1);
    line(&actions, "launch build:a build opus ");
    assert_eq!(fx.file().run("build:a").unwrap().stops["tool"], 1);
}

#[test]
fn blocked_and_running_relaunch_nothing() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    running(&fx, "build:a", "t-1");
    running(&fx, "build:b", "t-2");
    let blocked = stop(&fx, "build:a", "stopped: needs the human", "blocked", None);
    assert!(
        !blocked.iter().any(|l| l.starts_with("launch ")),
        "{blocked:?}"
    );
    assert_eq!(fx.file().run("build:a").unwrap().state, RunState::Returned);
    let background = stop(&fx, "build:b", "none", "running", None);
    assert!(
        !background.iter().any(|l| l.starts_with("launch ")),
        "{background:?}"
    );
    assert_eq!(fx.file().run("build:b").unwrap().state, RunState::Running);
}

#[test]
fn a_recurring_limit_resumes_one_run_at_a_time() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    for (name, task) in [("build:a", "t-1"), ("build:b", "t-2")] {
        running(&fx, name, task);
    }
    let until = rfc3339(clm_git::now() + 3600);
    stop(&fx, "build:a", "none", "limit", Some(&until));
    stop(&fx, "build:b", "none", "limit", Some(&until));
    assert!(
        fx.file().conditions.serial,
        "a second run stopped while it holds"
    );
    fx.set("next", "c\n");
    for name in ["build:a", "build:b"] {
        fx.change_run(name, |r| {
            r.resume.as_mut().unwrap().after = Some(clm_git::now() - 1)
        });
    }
    let first = step(&fx.options()).unwrap();
    assert_eq!(
        first.iter().filter(|l| l.starts_with("resume ")).count(),
        1,
        "{first:?}"
    );
    line(&first, "resume build:a t-1 ");
    assert!(!first.iter().any(|l| l.starts_with("launch ")), "{first:?}");
    // While the first runs, nothing else resumes or launches.
    let again = step(&fx.options()).unwrap();
    assert!(
        !again
            .iter()
            .any(|l| l.starts_with("resume ") || l.starts_with("launch ")),
        "{again:?}"
    );
    fx.set("finish-a", "landed impl/a\n");
    let after = returned(&fx.options(), "build:a", "landed 1a2b3c4", None, None).unwrap();
    line(&after, "resume build:b t-2 ");
    assert!(!after.iter().any(|l| l.starts_with("launch ")), "{after:?}");
    fx.set("finish-b", "landed impl/b\n");
    let last = returned(&fx.options(), "build:b", "landed 5d6e7f8", None, None).unwrap();
    line(&last, "launch build:c ");
}

#[test]
fn an_unknown_cause_or_until_is_refused() {
    // req:orchestrator.loop/stop-causes@ceb78f17
    let fx = Fixture::new();
    running(&fx, "build:a", "t-1");
    assert!(returned(&fx.options(), "build:a", "none", Some("weather"), None).is_err());
    assert!(
        returned(
            &fx.options(),
            "build:a",
            "none",
            Some("limit"),
            Some("soon")
        )
        .is_err()
    );
    assert!(
        returned(
            &fx.options(),
            "build:a",
            "none",
            Some("work"),
            Some(&rfc3339(1))
        )
        .is_err()
    );
}
