//! A draft the loop printed or left an orphan whose items have all left `origin/main`
//! (`draft-of-gone-items-dropped`).

mod common;

use clm_loop::{Kind, RunState, start, step};
use common::{Fixture, line, run};

/// Deletes `backlog/<slug>.md` from `origin/main`, as a build's `Resolves:` commit does.
fn resolve(fx: &Fixture, slug: &str) {
    let git = |args: &[&str]| clm_git::git(&fx.work, args).unwrap();
    git(&["rm", "-q", &format!("backlog/{slug}.md")]);
    git(&["commit", "-q", "-m", &format!("Resolves: backlog/{slug}")]);
    git(&["push", "-q", "origin", "HEAD:main"]);
}

/// Pushes `spec/<id>` to `origin`, as a draft's session does.
fn push_spec(fx: &Fixture, id: &str) {
    let git = |args: &[&str]| clm_git::git(&fx.work, args).unwrap();
    git(&["fetch", "-q", "origin"]);
    let commit = git(&[
        "commit-tree",
        "-p",
        "origin/main",
        "-m",
        "spec",
        "origin/main^{tree}",
    ]);
    git(&[
        "push",
        "-q",
        "origin",
        &format!("{commit}:refs/heads/spec/{id}"),
    ]);
}

/// A draft run of `items`, named `draft:<id>`.
fn draft(id: &str, items: &[&str], state: RunState, task: Option<&str>) -> clm_loop::Run {
    let mut r = run(&format!("draft:{id}"), Kind::Draft, "opus", state, task);
    r.items = items.iter().map(|s| s.to_string()).collect();
    r.prompt = Some("/tmp/prompt.md".to_string());
    r
}

#[test]
fn a_printed_draft_whose_only_item_a_build_resolved() {
    // req:orchestrator.loop/draft-of-gone-items-dropped@4dafc4af
    let fx = Fixture::new();
    fx.land_file("backlog/b.md", "---\nkind: bug\n---\nb\n");
    fx.set("triage", "group bug:b [b] P3 0 b\n");
    let first = step(&fx.options()).unwrap();
    line(&first, "launch draft:d1 ");
    fx.set("triage", "");
    fx.set("finish-d1", "freed d1\nleft /w/agent-d1: still on disk\n");
    resolve(&fx, "b");
    let second = step(&fx.options()).unwrap();
    assert!(!second.iter().any(|l| l.contains("draft:d1")), "{second:?}");
    let calls = fx.calls();
    let finish = calls.iter().position(|c| c == "draft --finish d1");
    let wrap_up = calls.iter().rposition(|c| c == "wrap-up");
    assert!(finish.is_some(), "{calls:?}");
    assert!(
        finish < wrap_up,
        "finished before the housekeeping: {calls:?}"
    );
    let file = fx.file();
    assert!(file.runs.iter().all(|r| r.name != "draft:d1"));
    // Its items aren't recorded as left by a draft, and its `left <path>` line is said.
    assert!(file.drafted.is_empty(), "{:?}", file.drafted);
    let said = line(&second, "say ")
        .strip_prefix("say ")
        .unwrap()
        .to_string();
    let text = std::fs::read_to_string(said).unwrap();
    assert!(text.contains("left /w/agent-d1: still on disk"), "{text}");
}

#[test]
fn one_item_still_open() {
    // req:orchestrator.loop/draft-of-gone-items-dropped@4dafc4af
    let fx = Fixture::new();
    fx.land_file("backlog/b.md", "---\nkind: bug\n---\nb\n");
    fx.land_file("backlog/c.md", "---\nkind: bug\n---\nc\n");
    fx.set("triage", "group bug:b [b] P3 0 b c\n");
    let first = step(&fx.options()).unwrap();
    let launch = line(&first, "launch draft:d1 ").to_string();
    fx.set("triage", "");
    resolve(&fx, "b");
    let second = step(&fx.options()).unwrap();
    assert_eq!(second, [launch]);
    assert_eq!(fx.ran("draft --finish d1"), 0);
}

#[test]
fn an_orphan_with_a_pushed_spec_branch() {
    // req:orchestrator.loop/draft-of-gone-items-dropped@4dafc4af
    let fx = Fixture::new();
    fx.add_run(draft("20261119-x", &["b"], RunState::Running, Some("t-1")));
    push_spec(&fx, "20261119-x");
    fx.set_lease(20);
    let actions = start(&fx.options(), "opus", false).unwrap();
    assert_eq!(fx.ran("draft --resume 20261119-x"), 1);
    line(&actions, "launch draft:20261119-x ");
    assert_eq!(fx.ran("draft --finish 20261119-x"), 0);
}

#[test]
fn an_orphan_with_nothing_pushed_is_dropped() {
    // req:orchestrator.loop/draft-of-gone-items-dropped@4dafc4af
    let fx = Fixture::new();
    fx.add_run(draft("20261119-x", &["b"], RunState::Running, Some("t-1")));
    fx.set_lease(20);
    let actions = start(&fx.options(), "opus", false).unwrap();
    assert!(
        !actions.iter().any(|l| l.contains("draft:20261119-x")),
        "{actions:?}"
    );
    assert_eq!(fx.ran("draft --resume 20261119-x"), 0);
    assert_eq!(fx.ran("draft --finish 20261119-x"), 1);
    let file = fx.file();
    assert!(file.runs.iter().all(|r| r.name != "draft:20261119-x"));
    // Its task stays kept under its name (`launched-refuses-an-earlier-task`).
    assert_eq!(
        file.kept_tasks.get("draft:20261119-x"),
        Some(&vec!["t-1".to_string()])
    );
}

#[test]
fn drafts_kept_as_before() {
    // req:orchestrator.loop/draft-of-gone-items-dropped@4dafc4af
    let fx = Fixture::new();
    // A running draft with a live session, and a planned draft that names no item.
    fx.add_run(draft("r", &["b"], RunState::Running, Some("t-1")));
    fx.add_run(draft("n", &[], RunState::Planned, None));
    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch draft:n ");
    assert_eq!(fx.ran("draft --finish r"), 0);
    assert_eq!(fx.ran("draft --finish n"), 0);
    let file = fx.file();
    assert!(file.runs.iter().any(|r| r.name == "draft:r"));
    assert!(file.runs.iter().any(|r| r.name == "draft:n"));
}
