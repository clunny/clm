//! Lands waiting for the land lock land together on one check (land/batched-lands,
//! land/failed-batch).

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use clm_ledger::{Body, Recheck};
use common::Fixture;

/// Holds `.clm/land.lock` as another process would, with no ticket.
fn hold(f: &Fixture) -> std::fs::File {
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/land.lock")).unwrap();
    held.lock().unwrap();
    held
}

/// The tickets in the land lock's queue.
fn tickets(root: &Path) -> usize {
    std::fs::read_dir(root.join(".clm/land.queue"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    (name.contains("-ordinary-p") || name.contains("-priority-p"))
                        && !name.ends_with(".alone")
                })
                .count()
        })
        .unwrap_or(0)
}

fn wait_for_tickets(root: &Path, n: usize) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while tickets(root) < n {
        assert!(Instant::now() < deadline, "{n} tickets never queued");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The lines of the check's log that start a run.
fn starts(f: &Fixture) -> usize {
    f.lines("check-order.log")
        .iter()
        .filter(|l| l.starts_with("start "))
        .count()
}

/// Lands `names` (branches `impl/<name>`) with the land lock held outside, each started once
/// the one before has its ticket, then releases the lock. Returns what each `clm land`
/// returned, in the order given.
fn land_all(
    f: &Fixture,
    names: &[&str],
    held: std::fs::File,
) -> Vec<Result<clm_land::Output, clm_land::Error>> {
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let mut runs = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let (env, work) = (&env, f.work());
            let branch = format!("impl/{name}");
            runs.push(s.spawn(move || clm_land::land(&work, Some(&branch), env)));
            wait_for_tickets(&f.work(), i + 1);
        }
        drop(held);
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    })
}

/// `main`'s first-parent subjects, newest first.
fn subjects(f: &Fixture) -> Vec<String> {
    f.git(
        &f.work(),
        &["log", "--first-parent", "--format=%s", "origin/main"],
    )
    .lines()
    .map(str::to_owned)
    .collect()
}

fn landed_lines(out: &clm_land::Output) -> String {
    out.lines
        .iter()
        .find(|l| l.starts_with("landed "))
        .cloned()
        .unwrap_or_default()
}

#[test]
fn three_waiting_lands_one_check() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.write("check-sleeps", "1");
    for n in ["a", "b", "c"] {
        f.branch(&format!("impl/{n}"), &[(&format!("{n}.txt"), "x\n")]);
    }
    let held = hold(&f);
    let outs = land_all(&f, &["a", "b", "c"], held);
    let outs: Vec<_> = outs.into_iter().map(|o| o.unwrap()).collect();
    assert_eq!(starts(&f), 1, "{:?}", f.lines("check-order.log"));
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let subjects = subjects(&f);
    assert_eq!(
        subjects[..3],
        [
            "Merge impl/c into main",
            "Merge impl/b into main",
            "Merge impl/a into main"
        ]
    );
    // Each `clm land` printed `landed` with its own merge commit.
    let chain: Vec<String> = f
        .git(
            &f.work(),
            &["log", "--first-parent", "--format=%H", "-3", "origin/main"],
        )
        .lines()
        .map(str::to_owned)
        .collect();
    for (out, commit) in outs.iter().zip(chain.iter().rev()) {
        assert_eq!(landed_lines(out), format!("landed {commit} code"));
    }
    // One record each, one key, `selected`, and the time only on the first.
    let landed = f.landed();
    assert_eq!(landed.len(), 3);
    let mut keys = Vec::new();
    for (i, body) in landed.iter().enumerate() {
        let Body::Landed {
            commit,
            code_key,
            recheck,
            check_seconds,
            steps,
            ..
        } = body
        else {
            panic!("{body:?}")
        };
        assert!(chain.contains(commit));
        assert_eq!(*recheck, Some(Recheck::Selected));
        keys.push(code_key.clone().unwrap());
        let first = commit == &chain[2];
        assert_eq!(check_seconds.is_some(), first, "record {i}");
        assert!(first || steps.is_empty());
    }
    assert!(keys.windows(2).all(|w| w[0] == w[1]), "{keys:?}");
    let findings = clm_ledger::verify(&f.work(), "origin/main", None, false).unwrap();
    assert!(findings.is_empty(), "{findings:?}");
}
