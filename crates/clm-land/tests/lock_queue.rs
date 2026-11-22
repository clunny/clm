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
                    name.contains("-ordinary-p") || name.contains("-priority-p")
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
    // req:land/lock-queue@32c5dce4
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
    // req:land/lock-queue@32c5dce4
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

/// Puts `a`, a component with an unbuilt requirement from an applied change of priority
/// `P1`, on `origin/main`, so a land of `impl/a` carries `P1` (`orchestrator/land-priority`).
fn a_is_p1(f: &Fixture) {
    let node = |reqs: &[&str]| {
        let mut s = String::from(
            "---\ntitle: T\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n\
             ## Contract\nc\n\n## Design\nd\n\n## Requirements\n",
        );
        for r in reqs {
            s.push_str(&format!(
                "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
            ));
        }
        s
    };
    f.commit_files(
        &[
            ("ARCHITECTURE.md", "# Arch\n"),
            ("specs/SPEC.md", &node(&[])),
            ("specs/a/SPEC.md", &node(&["p"])),
            (
                "changes/x/change.md",
                "---\nstatus: applied\npriority: P1\n---\n",
            ),
            (
                "changes/x/deltas/a.md",
                "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: p\n\
                 It SHALL.\n\n#### Scenario: s\n- ok\n\n",
            ),
        ],
        "a is P1",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
}

#[test]
fn a_p1_land() {
    // req:land/lock-queue@32c5dce4
    let f = Fixture::new("");
    a_is_p1(&f);
    logging_check(&f);
    f.branch("impl/wa", &[("wa.txt", "x\n")]);
    f.branch("impl/a", &[("a.txt", "changed\n")]);
    let only = f.linked("wa", "impl/wa");
    let held = hold(&f);
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let waiting = s.spawn(|| clm_land::check_only(&only, None, &env));
        wait_for_tickets(&f.work(), 1);
        let land = s.spawn(|| clm_land::land(&f.work(), Some("impl/a"), &env));
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

/// `clm land <args>` as the call, in the fixture's main worktree, on a thread of its own so a
/// run that never takes its turn fails the test rather than hanging it: its exit status,
/// stdout and stderr.
fn call(f: &Fixture, args: &[&str]) -> (u8, String, String) {
    let (work, env) = (f.work(), f.env(&[]));
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (sent, got) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = clm_land::land_command(&args, &work, &env, &mut out, &mut err);
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        let _ = sent.send((code, text(&out), text(&err)));
    });
    got.recv_timeout(Duration::from_secs(60))
        .expect("`clm land` never finished")
}

/// Holds the lock file `.clm/<name>` as another process would, releasing it after a second.
fn hold_for_a_second(f: &Fixture, name: &str) -> std::thread::JoinHandle<()> {
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm").join(name)).unwrap();
    held.lock().unwrap();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        drop(held);
    })
}

#[test]
fn the_waiting_lines() {
    // req:land/lock-queue@32c5dce4
    let f = Fixture::new("");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let home = std::fs::canonicalize(f.work()).unwrap().join(".clm");
    let releaser = hold_for_a_second(&f, "land.lock");
    let (code, _, stderr) = call(&f, &["impl/a"]);
    releaser.join().unwrap();
    assert_eq!(code, 0, "{stderr}");
    let line = format!(
        "waiting for another land's check on {}: 1 ahead",
        home.join("land").display()
    );
    assert!(stderr.lines().any(|l| l == line), "{stderr}");
    let releaser = hold_for_a_second(&f, "check.lock");
    let (code, stdout, stderr) = call(&f, &["--check-only"]);
    releaser.join().unwrap();
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.ends_with("checks pass\n"), "{stdout}");
    let line = format!(
        "waiting for another check on {}: 1 ahead",
        home.join("check.lock").display()
    );
    assert!(stderr.lines().any(|l| l == line), "{stderr}");
}

#[test]
fn a_ticket_it_cant_open() {
    // req:land/lock-queue@32c5dce4
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new("");
    let queue = f.work().join(".clm/check.queue");
    std::fs::create_dir_all(&queue).unwrap();
    // A waiting ticket whose run has ended, which no one may open.
    let ticket = queue.join("00000000000000000000-ordinary-p5");
    std::fs::write(&ticket, "").unwrap();
    std::fs::set_permissions(&ticket, std::fs::Permissions::from_mode(0o000)).unwrap();
    let releaser = hold_for_a_second(&f, "check.lock");
    let (code, stdout, stderr) = call(&f, &["--check-only"]);
    releaser.join().unwrap();
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.ends_with("checks pass\n"), "{stdout}");
    let waits: Vec<&str> = stderr.lines().filter(|l| l.contains(" ahead")).collect();
    assert_eq!(waits.len(), 1, "{stderr}");
    assert!(waits[0].ends_with(": 1 ahead"), "{stderr}");
    assert!(ticket.exists());
    std::fs::set_permissions(&ticket, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn an_unwritable_queue() {
    // req:land/lock-queue@32c5dce4
    let f = Fixture::new("");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let old = f.remote("main");
    // A file where the check lock's queue would be.
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    std::fs::write(f.work().join(".clm/check.queue"), "").unwrap();
    let releaser = hold_for_a_second(&f, "check.lock");
    let (code, _, stderr) = call(&f, &["--check-only"]);
    releaser.join().unwrap();
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains(".clm/check.queue"), "{stderr}");
    assert!(f.lines("check.log").is_empty());
    // And where the land lock's would be.
    std::fs::remove_file(f.work().join(".clm/check.queue")).unwrap();
    std::fs::write(f.work().join(".clm/land.queue"), "").unwrap();
    let releaser = hold_for_a_second(&f, "land.lock");
    let (code, _, stderr) = call(&f, &["impl/a"]);
    releaser.join().unwrap();
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains(".clm/land.queue"), "{stderr}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.landed().is_empty());
    assert!(f.checked().is_empty());
}

#[test]
fn a_priority_that_cant_be_worked_out() {
    // req:land/lock-queue@32c5dce4
    // The fixture's `origin/main` holds no specs that load.
    let f = Fixture::new("");
    f.branch("impl/a", &[("a.txt", "x\n")]);
    let releaser = hold_for_a_second(&f, "land.lock");
    let (code, stdout, stderr) = call(&f, &["impl/a"]);
    releaser.join().unwrap();
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.starts_with("landed "), "{stdout}");
    let warning = "clm land: warning: land priority not worked out, queueing at P5: ";
    let line = stderr
        .lines()
        .find(|l| l.starts_with(warning))
        .expect(&stderr);
    assert!(line.len() > warning.len(), "{stderr}");
}
