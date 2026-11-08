//! The clone's check lock goes to the runs waiting for it in an order CLM sets
//! (land/lock-queue).

mod common;

use common::Fixture;
use std::path::Path;
use std::time::{Duration, Instant};

/// A check that logs `start <directory>` and then `end <directory>`, to a log outside every
/// worktree.
fn logging_check(f: &Fixture) {
    let log = f.path("order.log");
    let script = format!(
        "#!/bin/sh\necho \"start $(pwd -P)\" >> '{log}'\nsleep 1\necho \"end $(pwd -P)\" >> '{log}'\n",
        log = log.display()
    );
    common::write_exec(&f.path("bin/check"), &script);
}

/// Holds `.clm/check.lock` as an older `clm` would, with no ticket.
fn hold(f: &Fixture) -> std::fs::File {
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/check.lock")).unwrap();
    held.lock().unwrap();
    held
}

/// The tickets in the check lock's queue.
fn tickets(root: &Path) -> usize {
    std::fs::read_dir(root.join(".clm/check.queue"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.ends_with("-ordinary") || name.ends_with("-priority")
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

fn starts(f: &Fixture) -> Vec<String> {
    f.lines("order.log")
        .into_iter()
        .filter_map(|l| l.strip_prefix("start ").map(str::to_string))
        .collect()
}

#[test]
fn turns_in_arrival_order() {
    // req:land/lock-queue@ea656239
    let f = Fixture::new("");
    logging_check(&f);
    let names = ["wa", "wb", "wc"];
    for n in names {
        f.branch(&format!("impl/{n}"), &[(&format!("{n}.txt"), "x\n")]);
    }
    let dirs: Vec<_> = names
        .iter()
        .map(|n| f.linked(n, &format!("impl/{n}")))
        .collect();
    let held = hold(&f);
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let mut runs = Vec::new();
        for (i, dir) in dirs.iter().enumerate() {
            let env = &env;
            runs.push(s.spawn(move || clm_land::check_only(dir, None, env)));
            wait_for_tickets(&f.work(), i + 1);
        }
        drop(held);
        for run in runs {
            assert_eq!(
                run.join().unwrap().unwrap().lines.last().unwrap(),
                "checks pass"
            );
        }
    });
    let want: Vec<_> = dirs
        .iter()
        .map(|d| std::fs::canonicalize(d).unwrap().display().to_string())
        .collect();
    assert_eq!(starts(&f), want);
    assert_eq!(tickets(&f.work()), 0);
}

#[test]
fn a_checked_land_goes_first() {
    // req:land/lock-queue@ea656239
    let f = Fixture::new("");
    logging_check(&f);
    f.write("readers-out", "file crates/a/tests/r.rs\n");
    f.branch("impl/wa", &[("wa.txt", "x\n")]);
    f.branch("spec/x", &[("ARCHITECTURE.md", "# x\n")]);
    let only = f.linked("wa", "impl/wa");
    f.mark_checked("origin/main");
    let held = hold(&f);
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let waiting = s.spawn(|| clm_land::check_only(&only, None, &env));
        wait_for_tickets(&f.work(), 1);
        let land = s.spawn(|| clm_land::land(&f.work(), Some("spec/x"), &env));
        wait_for_tickets(&f.work(), 2);
        drop(held);
        let landed = land.join().unwrap().unwrap();
        assert!(landed.lines[0].starts_with("landed "), "{:?}", landed.lines);
        waiting.join().unwrap().unwrap();
    });
    let starts = starts(&f);
    assert_eq!(starts.len(), 2, "{starts:?}");
    assert!(starts[0].contains("land-tree"), "{starts:?}");
    assert_eq!(tickets(&f.work()), 0);
}
