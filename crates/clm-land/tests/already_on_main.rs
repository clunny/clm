//! A land of a branch whose head `main` already holds lands nothing (land/already-on-main):
//! the branch's own recorded land is reported, and any other head `main` holds is refused.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use clm_ledger::Body;
use common::{Fixture, refused};

fn wait_for(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < deadline, "{what} never happened");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The `landed` records of `branch`.
fn landed_of(f: &Fixture, branch: &str) -> Vec<String> {
    f.landed()
        .into_iter()
        .filter_map(|b| match b {
            Body::Landed {
                commit, branch: b, ..
            } if b == branch => Some(commit),
            _ => None,
        })
        .collect()
}

/// The lines of the check's log that start a run.
fn starts(f: &Fixture) -> usize {
    f.lines("check-order.log")
        .iter()
        .filter(|l| l.starts_with("start "))
        .count()
}

/// The merges of `head` on `main`'s first-parent chain.
fn merges_of(f: &Fixture, head: &str) -> usize {
    f.git(
        &f.work(),
        &["log", "--first-parent", "--format=%P", "origin/main"],
    )
    .lines()
    .filter(|l| l.split_whitespace().nth(1) == Some(head))
    .count()
}

fn has_local(f: &Fixture, branch: &str) -> bool {
    !f.git(&f.work(), &["branch", "--list", branch]).is_empty()
}

/// The tickets in `root`'s land lock queue.
fn tickets(root: &Path) -> usize {
    std::fs::read_dir(root.join(".clm/land.queue"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.contains("-ordinary-p") || name.contains("-priority-p")
                })
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn a_branch_landed_before() {
    // req:land/already-on-main@5d3c9a80
    let f = Fixture::new("");
    let head = f.branch("impl/a", &[("a2.txt", "x\n")]);
    let env = f.env(&[]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    let m = f.remote("main");
    assert_eq!(out.lines[0], format!("landed {m} code"));
    // The local branch made again at the head it landed.
    f.git(&f.work(), &["branch", "impl/a", &head]);
    let again = clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    assert_eq!(again.lines[0], format!("landed {m} code"), "{again:?}");
    assert_eq!(f.remote("main"), m);
    assert_eq!(starts(&f), 1, "{:?}", f.lines("check-order.log"));
    assert_eq!(landed_of(&f, "impl/a"), vec![m]);
    assert!(!has_local(&f, "impl/a"));
}

#[test]
fn a_light_branch_landed_before() {
    // req:land/already-on-main@5d3c9a80
    // The word the record gives, `light` included.
    let f = Fixture::new("");
    f.write("gate-out", "light\n");
    let head = f.branch("impl/a", &[("a2.txt", "x\n")]);
    let env = f.env(&[]);
    clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    let m = f.remote("main");
    // The gate would read the newer `main` otherwise: the record's word stands.
    f.write("gate-out", "code\n");
    f.git(&f.work(), &["branch", "impl/a", &head]);
    let again = clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    assert_eq!(again.lines[0], format!("landed {m} light"), "{again:?}");
}

#[test]
fn landed_while_it_checked() {
    // req:land/already-on-main@5d3c9a80
    // req:land/push-turn@a2816e96
    // The first clone's check waits until the second clone has landed `impl/a`; the land that
    // then takes its push turn finds that land's merge recorded.
    let f = Fixture::new("");
    let head = f.branch("impl/a", &[("a2.txt", "x\n")]);
    let second = f.clone_as("second", "Second");
    f.write("hold", "");
    let env = f.env(&[]);
    let first = std::thread::scope(|s| {
        let first = s.spawn(|| clm_land::land(&f.work(), Some("impl/a"), &env));
        wait_for("the first check", || starts(&f) == 1);
        // The second clone's check doesn't wait.
        std::fs::remove_file(f.path("hold")).unwrap();
        let out = clm_land::land(&second, Some("impl/a"), &env).unwrap();
        assert!(out.lines[0].starts_with("landed "), "{out:?}");
        f.write("release", "");
        first.join().unwrap()
    });
    let first = first.unwrap();
    let m = f.remote("main");
    assert_eq!(f.parents(&f.path("origin.git"), &m)[1], head);
    assert_eq!(first.lines[0], format!("landed {m} code"), "{first:?}");
    assert_eq!(landed_of(&f, "impl/a"), vec![m.clone()]);
    assert_eq!(merges_of(&f, &head), 1);
    assert!(
        !first
            .warnings
            .iter()
            .any(|w| w.contains("kept `impl/a` on origin")),
        "{:?}",
        first.warnings
    );
}

#[test]
fn a_head_main_holds_another_way() {
    // req:land/already-on-main@5d3c9a80
    let f = Fixture::new("");
    let work = f.work();
    f.branch("impl/b", &[("b.txt", "b\n")]);
    // `impl/c` holds `impl/b`.
    f.git(&work, &["checkout", "-q", "-b", "impl/c", "impl/b"]);
    f.commit_files(&[("c.txt", "c\n")], "impl/c");
    f.git(&work, &["push", "-q", "origin", "impl/c"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let env = f.env(&[]);
    clm_land::land(&work, Some("impl/c"), &env).unwrap();
    let m = f.remote("main");
    let t = f.advance_main("later.txt", "later\n");
    f.git(&work, &["fetch", "-q", "origin"]);
    f.git(&work, &["branch", "fix-x", &t]);
    let records = f.records().len();
    let b = refused(clm_land::land(&work, Some("impl/b"), &env));
    assert!(
        b.contains(&format!("`impl/b` is already on main, in {m}")),
        "{b}"
    );
    let x = refused(clm_land::land(&work, Some("fix-x"), &env));
    assert!(
        x.contains(&format!("`fix-x` is already on main, in {t}")),
        "{x}"
    );
    assert_eq!(f.remote("main"), t);
    assert!(has_local(&f, "impl/b"));
    assert!(has_local(&f, "fix-x"));
    assert_eq!(f.records().len(), records);
}

#[test]
fn a_waiting_land_main_already_holds() {
    // req:land/already-on-main@5d3c9a80
    let f = Fixture::new("");
    f.branch("impl/a", &[("a2.txt", "x\n")]);
    let b_head = f.branch("impl/b", &[("b.txt", "x\n")]);
    // Only on `origin`, which the other land's cleanup then removes it from: the waiting land
    // asks about the head its first attempt resolved.
    f.git(&f.work(), &["branch", "-q", "-D", "impl/b"]);
    let second = f.clone_as("second", "Second");
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/land.lock")).unwrap();
    held.lock().unwrap();
    let env = f.env(&[]);
    let (a, b) = std::thread::scope(|s| {
        let a = s.spawn(|| clm_land::land(&f.work(), Some("impl/a"), &env));
        wait_for("impl/a's ticket", || tickets(&f.work()) == 1);
        let b = s.spawn(|| clm_land::land(&f.work(), Some("impl/b"), &env));
        wait_for("impl/b's ticket", || tickets(&f.work()) == 2);
        // `impl/b` lands meanwhile from the second clone.
        let out = clm_land::land(&second, Some("impl/b"), &env).unwrap();
        assert!(out.lines[0].starts_with("landed "), "{out:?}");
        drop(held);
        (a.join().unwrap(), b.join().unwrap())
    });
    let (a, b) = (a.unwrap(), b.unwrap());
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let m = f.rev("origin/main^");
    assert_eq!(f.parents(&f.work(), &m)[1], b_head);
    let tip = f.rev("origin/main");
    assert_eq!(a.lines[0], format!("landed {tip} code"), "{a:?}");
    assert_eq!(f.parents(&f.work(), &tip)[0], m);
    assert_eq!(b.lines[0], format!("landed {m} code"), "{b:?}");
    assert_eq!(merges_of(&f, &b_head), 1);
}

#[test]
fn a_branch_already_landed_under_a_newer_main() {
    // req:land/already-on-main@5d3c9a80
    let root = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";
    let node = "---\ntitle: A\nstatus: specd\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n\
                ## Contract\nc\n\n## Design\nd\n\n## Requirements\n";
    let f = Fixture::new("");
    f.commit_files(
        &[("specs/SPEC.md", root), ("specs/a/SPEC.md", node)],
        "components",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    let head = f.branch("impl/a", &[("crates/a/src/lib.rs", "a\n")]);
    let env = f.env(&[]);
    clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    let m = f.remote("main");
    // `main` gains an item that would refuse a new land of `impl/a` (`verify-stop`).
    let item = "---\ntitle: Slow tests broke\nkind: bug\ncomponents: [a]\n\
                verify_failures: [\"0123456789abcdef0123456789abcdef01234567\"]\n---\nBroke.\n";
    f.advance_main("backlog/a-broke.md", item);
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    f.git(&f.work(), &["branch", "impl/a", &head]);
    let again = clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    assert_eq!(again.lines[0], format!("landed {m} code"), "{again:?}");
}
