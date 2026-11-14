//! One land at a time between the fetch of `main` its push relies on and that push
//! (land/push-turn), and a land that waited for a lock merges with the `main` it finds then
//! (land/land-target-lock, land/overlap-recheck).

mod common;

use common::Fixture;
use std::path::Path;
use std::time::{Duration, Instant};

/// A `clm check` that logs `start` and sleeps, so that lands with nothing else to check
/// overlap in it.
fn slow_clm_check(f: &Fixture, passes: bool) {
    let log = f.path("clm-check-order.log");
    let script = format!(
        "#!/bin/sh\necho start >> '{}'\nsleep 2\n{}\n",
        log.display(),
        if passes { "exit 0" } else { "exit 1" }
    );
    common::write_exec(&f.path("bin/clm-check"), &script);
}

fn wait_for(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done() {
        assert!(Instant::now() < deadline, "{what} never happened");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The tickets of the push turn's queue.
fn tickets(root: &Path) -> usize {
    std::fs::read_dir(root.join(".clm/push.queue"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().contains("-ordinary-p"))
                .count()
        })
        .unwrap_or(0)
}

fn moved(out: &clm_land::Output) -> bool {
    out.warnings.iter().any(|w| w.contains("main moved"))
}

#[test]
fn two_lands_with_nothing_to_check() {
    // req:land/push-turn@379d1699
    let f = Fixture::new("");
    slow_clm_check(&f, true);
    f.branch("spec/a", &[("specs/a/SPEC.md", "# a\n")]);
    f.branch("spec/b", &[("specs/b/SPEC.md", "# b\n")]);
    f.mark_checked("origin/main");
    let (a, b) = (f.linked("wa", "spec/a"), f.linked("wb", "spec/b"));
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let one = s.spawn(|| clm_land::land(&a, None, &env));
        let two = s.spawn(|| clm_land::land(&b, None, &env));
        for out in [one.join().unwrap().unwrap(), two.join().unwrap().unwrap()] {
            assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
            assert!(!moved(&out), "{:?}", out.warnings);
        }
    });
    // One of them waited: its ticket's number was issued.
    assert!(f.work().join(".clm/push.queue/next").exists());
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn turns_in_the_order_asked() {
    // req:land/push-turn@379d1699
    let f = Fixture::new("");
    slow_clm_check(&f, true);
    for name in ["a", "b", "c"] {
        f.branch(
            &format!("spec/{name}"),
            &[(&format!("specs/{name}/SPEC.md"), "# x\n")],
        );
    }
    f.mark_checked("origin/main");
    let trees: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|n| f.linked(&format!("w{n}"), &format!("spec/{n}")))
        .collect();
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let first = s.spawn(|| clm_land::land(&trees[0], None, &env));
        wait_for("the first land's check", || {
            !f.lines("clm-check-order.log").is_empty()
        });
        let second = s.spawn(|| clm_land::land(&trees[1], None, &env));
        wait_for("the second land's ticket", || tickets(&f.work()) >= 1);
        let third = s.spawn(|| clm_land::land(&trees[2], None, &env));
        for run in [first, second, third] {
            let out = run.join().unwrap().unwrap();
            assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
            assert!(!moved(&out), "{:?}", out.warnings);
        }
    });
    let log = f.git(
        &f.path("origin.git"),
        &["log", "--first-parent", "--merges", "--format=%s", "main"],
    );
    let order: Vec<_> = log.lines().rev().collect();
    assert_eq!(
        order,
        [
            "Merge spec/a into main",
            "Merge spec/b into main",
            "Merge spec/c into main"
        ],
        "{log}"
    );
}

#[test]
fn released_when_the_attempt_fails() {
    // req:land/push-turn@379d1699
    let f = Fixture::new("");
    f.branch("spec/a", &[("specs/a/SPEC.md", "# a\n")]);
    f.branch("spec/b", &[("specs/b/SPEC.md", "# b\n")]);
    f.mark_checked("origin/main");
    let env = f.env(&[]);
    slow_clm_check(&f, false);
    let failed = clm_land::land(&f.work(), Some("spec/a"), &env);
    assert!(failed.is_err());
    slow_clm_check(&f, true);
    let out = clm_land::land(&f.work(), Some("spec/b"), &env).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    // Nobody waited, so the second land took the turn at once.
    assert!(!f.work().join(".clm/push.queue/next").exists());
    assert_eq!(tickets(&f.work()), 0);
}

/// A land of `spec/x`'s kind, whose check logs the files of the tree it runs in.
fn tree_logging_check(f: &Fixture) {
    let log = f.path("tree.log");
    let script = format!(
        "#!/bin/sh\necho \"run $(git ls-files | tr '\\n' ' ')\" >> '{}'\nsleep 2\n",
        log.display()
    );
    common::write_exec(&f.path("bin/check"), &script);
}

#[test]
fn a_land_that_waited_checks_the_new_main() {
    // req:land/land-target-lock@2dadb710
    let f = Fixture::new("");
    tree_logging_check(&f);
    f.branch("impl/a", &[("a-file.txt", "a\n")]);
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.mark_checked("origin/main");
    f.write("readers-out", "file crates/x/tests/t.rs\n");
    let (a, x) = (f.linked("wa", "impl/a"), f.linked("wx", "spec/x"));
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let first = s.spawn(|| clm_land::land(&a, None, &env));
        wait_for("impl/a's check", || !f.lines("tree.log").is_empty());
        let second = s.spawn(|| clm_land::land(&x, None, &env));
        let first = first.join().unwrap().unwrap();
        let second = second.join().unwrap().unwrap();
        assert!(first.lines[0].starts_with("landed "), "{:?}", first.lines);
        assert!(second.lines[0].starts_with("landed "), "{:?}", second.lines);
        assert!(!moved(&second), "{:?}", second.warnings);
    });
    let runs = f.lines("tree.log");
    assert_eq!(runs.len(), 2, "{runs:?}");
    // The first is impl/a's own; the second is spec/x's, in a tree holding impl/a's file.
    assert!(!runs[0].contains("specs/x"), "{runs:?}");
    assert!(
        runs[1].contains("specs/x") && runs[1].contains("a-file.txt"),
        "{runs:?}"
    );
}

#[test]
fn a_land_with_nothing_to_check_doesnt_wait_for_a_check() {
    // req:land/push-turn@379d1699
    let f = Fixture::new("");
    f.write("hold", "");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.mark_checked("origin/main");
    let (a, x) = (f.linked("wa", "impl/a"), f.linked("wx", "spec/x"));
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let first = s.spawn(|| clm_land::land(&a, None, &env));
        wait_for("impl/a's check", || !f.lines("check-order.log").is_empty());
        // Nothing to check: it lands while the other land's check still runs.
        let out = clm_land::land(&x, None, &env).unwrap();
        assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
        assert!(f.lines("check-order.log").len() == 1);
        f.write("release", "");
        let out = first.join().unwrap().unwrap();
        assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    });
    assert_eq!(f.lines("check.log").len(), 1);
}
