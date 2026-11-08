//! Whether a trigger has fired since the last verified commit (verify/verify-due).

mod common;

use common::Fixture;

const CHANGE: &str = "---\nstatus: applied\n---\nintent\n";

fn local(triggers: &str, every: &str) -> Fixture {
    Fixture::new(Some(&format!(
        "mode = \"local\"\ntriggers = [{triggers}]\n{every}"
    )))
}

#[test]
fn a_finished_change_is_due() {
    // req:verify/verify-due@702ae35c
    let f = local("\"change-done\"", "");
    f.land(&[("a.txt", "1\n")], "one");
    let green = f.land(&[("a.txt", "2\n")], "two");
    f.verified(&green, true);
    f.land(&[("a.txt", "3\n")], "three");
    f.land(&[("changes/archive/x/change.md", CHANGE)], "archive x");
    let before = f.records().len();
    let out = f.dry(false);
    assert_eq!((out.line.as_str(), out.code), ("verify due change-done", 0));
    assert_eq!(f.records().len(), before);
    assert!(f.lines("check-pwd.log").is_empty());
}

#[test]
fn nothing_finished() {
    // req:verify/verify-due@702ae35c
    let f = local("\"change-done\"", "");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    f.land(&[("changes/x/change.md", CHANGE)], "a live change, not an archived one");
    let before = f.records().len();
    let out = f.due();
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert_eq!(f.records().len(), before);
    assert!(f.lines("check-pwd.log").is_empty());
}

#[test]
fn a_dry_run_that_finds_nothing_due_exits_3() {
    // req:verify/verify-due@702ae35c
    let f = local("\"change-done\"", "");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    let out = f.dry(false);
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert_eq!(out.code, 3);
}

#[test]
fn work_done_only_when_asked() {
    // req:verify/verify-due@702ae35c
    let f = local("\"work-done\"", "");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    let first = f.due();
    assert!(first.line.starts_with("verify not-due"), "{}", first.line);
    let second = f.dry(true);
    assert_eq!(second.line, "verify due work-done");
    assert_eq!(second.code, 0);
}

#[test]
fn a_trigger_the_project_did_not_choose_does_not_fire() {
    // req:verify/verify-due@702ae35c
    let f = local("\"change-done\"", "");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    let out = f.dry(true);
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
}

#[test]
fn backstop_by_lands() {
    // req:verify/verify-due@702ae35c
    let f = local("", "every = { lands = 3 }");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    f.land(&[("a.txt", "3\n")], "three");
    assert!(f.dry(false).line.starts_with("verify not-due"));
    f.land(&[("a.txt", "4\n")], "four");
    assert_eq!(f.dry(false).line, "verify due every.lands");
}

#[test]
fn backstop_by_days() {
    // req:verify/verify-due@702ae35c
    let f = local("", "every = { days = 2 }");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "recent");
    assert!(f.dry(false).line.starts_with("verify not-due"));
    // A land committed three days ago, the oldest unverified.
    let old = f.git(&f.work(), &["log", "-1", "--format=%H"]);
    let date = clm_git::now() - 3 * 24 * 60 * 60;
    f.git(&f.work(), &["reset", "-q", "--hard", &green]);
    std::fs::write(f.work().join("a.txt"), "old\n").unwrap();
    f.git(&f.work(), &["add", "-A"]);
    let stamp = format!("{date} +0000");
    let out = std::process::Command::new("git")
        .args(["commit", "-q", "-m", "old"])
        .env("GIT_COMMITTER_DATE", &stamp)
        .env("GIT_AUTHOR_DATE", &stamp)
        .current_dir(f.work())
        .output()
        .unwrap();
    assert!(out.status.success());
    f.git(&f.work(), &["push", "-q", "-f", "origin", "main"]);
    assert_ne!(old, f.back(0));
    assert_eq!(f.dry(false).line, "verify due every.days");
}

#[test]
fn red_stays_due() {
    // req:verify/verify-due@702ae35c
    let f = local("", "");
    let red = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&red, false);
    f.land(&[("a.txt", "2\n")], "two");
    assert_eq!(f.dry(false).line, "verify due red-again");
}

#[test]
fn queue_drained_waits_for_the_last_pending_requirement() {
    // req:verify/verify-due@702ae35c
    let f = local("\"queue-drained\"", "");
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    // Component `a` holds a requirement no test cites.
    assert!(f.dry(false).line.starts_with("verify not-due"));
    let cited = format!("// {}{}\n", concat!("req", ":a/thing@"), hash_of_thing(&f));
    f.land(&[("crates/a/src/lib.rs", &cited)], "built");
    assert_eq!(f.dry(false).line, "verify due queue-drained");
}

/// The current version of `a/thing`, as `clm hashes` gives it.
fn hash_of_thing(f: &Fixture) -> String {
    let (graph, _) = clm_model::load(&f.work());
    let node = graph.nodes.get("a").unwrap();
    let r = node
        .requirements
        .iter()
        .find(|r| r.id == "a/thing")
        .expect("requirement");
    r.version().to_string()
}

#[test]
fn nothing_landed() {
    // req:verify/verify-due@702ae35c
    let f = local("\"work-done\"", "");
    let head = f.back(0);
    f.verified(&head, true);
    let out = f.run(&clm_verify::Request::Due {
        work_done: true,
        dry_run: false,
    });
    let short = f.git(&f.work(), &["rev-parse", "--short", &head]);
    assert_eq!(out.line, format!("verify not-due nothing landed since {short}"));
    assert_eq!(out.code, 0);
}

#[test]
fn off() {
    // req:verify/verify-due@702ae35c
    let f = Fixture::new(None);
    let requests = [
        clm_verify::Request::Due {
            work_done: false,
            dry_run: false,
        },
        clm_verify::Request::Import,
        clm_verify::Request::Commit(f.back(0)),
        clm_verify::Request::Bisect(f.back(0)),
    ];
    for request in requests {
        let out = f.run(&request);
        assert_eq!(out.line, "verify off: no [verify] in clm.toml");
        assert_eq!(out.code, 0);
    }
}
