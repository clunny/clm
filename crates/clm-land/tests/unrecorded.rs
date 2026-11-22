//! A land pushed and not recorded is finished by `clm land` of its branch
//! (land/unrecorded-land-finished).

mod common;

use std::time::{Duration, Instant};

use clm_ledger::Body;
use common::Fixture;

/// Merges each of `branches` onto `main` as `clm land` pushes lands taken together does, each
/// its own merge commit, with `Checked: <key>` naming the code key of the last when there are
/// several, pushes them to `origin`, and returns the commits and the key. No record is made.
fn pushed(f: &Fixture, branches: &[&str]) -> (Vec<String>, String) {
    let work = f.work();
    let build = |lines: Option<&str>| -> Vec<(String, String)> {
        let mut prev = f.rev("origin/main");
        let mut made = Vec::new();
        for b in branches {
            let head = f.rev(&format!("origin/{b}"));
            let tree = f
                .git(&work, &["merge-tree", "--write-tree", &prev, &head])
                .lines()
                .next()
                .unwrap()
                .to_string();
            let message = match lines {
                Some(key) => format!("Merge {b} into main\n\nChecked: {key}"),
                None => format!("Merge {b} into main"),
            };
            let commit = f.git(
                &work,
                &[
                    "commit-tree",
                    &tree,
                    "-p",
                    &prev,
                    "-p",
                    &head,
                    "-m",
                    &message,
                ],
            );
            made.push((commit.clone(), tree));
            prev = commit;
        }
        made
    };
    let plain = build(None);
    let key = clm_ledger::code_key(&work, &plain.last().unwrap().0).unwrap();
    let made = if branches.len() > 1 {
        build(Some(&key))
    } else {
        plain
    };
    let tip = made.last().unwrap().0.clone();
    f.git(
        &work,
        &["push", "-q", "origin", &format!("{tip}:refs/heads/main")],
    );
    f.git(&work, &["fetch", "-q", "origin"]);
    (made.into_iter().map(|(c, _)| c).collect(), key)
}

fn landed_for(f: &Fixture) -> Vec<(String, String, Option<u64>, Option<String>)> {
    f.landed()
        .into_iter()
        .filter_map(|b| match b {
            Body::Landed {
                commit,
                branch,
                check_seconds,
                code_key,
                ..
            } => Some((commit, branch, check_seconds, code_key)),
            _ => None,
        })
        .collect()
}

#[test]
fn lands_taken_together_pushed_and_not_recorded() {
    // req:land/unrecorded-land-finished@6700f3ee
    let f = Fixture::new("");
    for n in ["a", "b", "c"] {
        f.branch(&format!("impl/{n}"), &[(&format!("{n}.txt"), "x\n")]);
    }
    let (commits, key) = pushed(&f, &["impl/a", "impl/b", "impl/c"]);
    f.mark_checked(&commits[2]);
    let main = f.remote("main");
    let out = clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert_eq!(out.lines[0], format!("landed {} code", commits[1]));
    assert_eq!(f.remote("main"), main);
    let landed = landed_for(&f);
    assert_eq!(landed.len(), 3, "{landed:?}");
    for (commit, _, seconds, code_key) in &landed {
        assert!(commits.contains(commit));
        assert_eq!(*seconds, None);
        assert_eq!(code_key.as_deref(), Some(key.as_str()));
    }
    let findings = clm_ledger::verify(&f.work(), "origin/main", None, false).unwrap();
    assert!(findings.is_empty(), "{findings:?}");
    // No check ran: the land was only finished.
    assert!(f.lines("check-order.log").is_empty());
}

#[test]
fn a_land_alone_pushed_and_not_recorded() {
    // req:land/unrecorded-land-finished@6700f3ee
    let f = Fixture::new("");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let (commits, _) = pushed(&f, &["impl/a"]);
    f.mark_checked(&commits[0]);
    let main = f.remote("main");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(out.lines[0], format!("landed {} code", commits[0]));
    assert_eq!(f.remote("main"), main);
    let findings = clm_ledger::verify(&f.work(), "origin/main", None, false).unwrap();
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn a_light_land_pushed_and_not_recorded() {
    // req:land/unrecorded-land-finished@3c6b2dbc
    // Finished, it is recorded as its own land would have recorded it: light.
    let f = Fixture::new("");
    f.write("gate-out", "light\n");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let (commits, _) = pushed(&f, &["impl/a"]);
    f.mark_checked(&commits[0]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(out.lines[0], format!("landed {} light", commits[0]));
    match &f.landed()[..] {
        [
            Body::Landed {
                kind,
                light,
                review,
                ..
            },
        ] => {
            assert_eq!(*kind, clm_ledger::LandKind::Code);
            assert!(*light);
            assert!(!*review);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_land_that_pushed_is_still_running() {
    // req:land/unrecorded-land-finished@6700f3ee
    let f = Fixture::new("");
    for n in ["a", "b"] {
        f.branch(&format!("impl/{n}"), &[(&format!("{n}.txt"), "x\n")]);
    }
    let (commits, _) = pushed(&f, &["impl/a", "impl/b"]);
    f.mark_checked(&commits[1]);
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/land.lock")).unwrap();
    held.lock().unwrap();
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let land = s.spawn(|| clm_land::land(&f.work(), Some("impl/b"), &env));
        // It waits for the lock, naming it, and appends nothing meanwhile.
        let deadline = Instant::now() + Duration::from_secs(30);
        while !std::fs::read_dir(f.work().join(".clm/land.queue"))
            .map(|d| {
                d.filter_map(Result::ok)
                    .any(|e| e.file_name().to_string_lossy().contains("-ordinary-"))
            })
            .unwrap_or(false)
        {
            assert!(Instant::now() < deadline, "it never queued");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(300));
        assert!(landed_for(&f).is_empty());
        drop(held);
        land.join().unwrap().unwrap();
    });
    assert_eq!(landed_for(&f).len(), 2);
}

#[test]
fn a_merge_no_land_made() {
    // req:land/unrecorded-land-finished@6700f3ee
    let f = Fixture::new("");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let (commits, _) = pushed(&f, &["impl/a"]);
    // No `checked` record names its code key.
    let main = f.remote("main");
    let why = common::refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("ledger/unrecorded-land"), "{why}");
    assert!(why.contains(&commits[0]), "{why}");
    assert_eq!(f.remote("main"), main);
    assert!(landed_for(&f).is_empty());
}

#[test]
fn an_unapproved_branch_pushed_by_hand() {
    // req:land/unrecorded-land-finished@6700f3ee
    let f = Fixture::new("");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let (commits, _) = pushed(&f, &["impl/a"]);
    f.mark_checked(&commits[0]);
    // The gate holds it for review.
    f.write("needs-approval", "");
    let main = f.remote("main");
    let why = common::refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("ledger/unrecorded-land"), "{why}");
    assert_eq!(f.remote("main"), main);
    assert!(landed_for(&f).is_empty());
}
