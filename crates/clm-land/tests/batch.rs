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

#[test]
fn a_land_that_conflicts_with_one_before_it() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.write("check-sleeps", "1");
    f.branch("impl/a", &[("a2.txt", "x\n")]);
    f.branch("impl/b", &[("a.txt", "b\n")]);
    f.branch("impl/c", &[("a.txt", "c\n")]);
    let held = hold(&f);
    let mut outs = land_all(&f, &["a", "b", "c"], held);
    let c = outs.pop().unwrap();
    let why = common::refused(c);
    assert!(why.contains("clm rebase"), "{why}");
    assert!(outs.iter().all(|o| o.is_ok()));
    assert_eq!(starts(&f), 1, "{:?}", f.lines("check-order.log"));
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    assert_eq!(
        subjects(&f)[..2],
        ["Merge impl/b into main", "Merge impl/a into main"]
    );
}

#[test]
fn a_land_the_gate_refuses_once_another_is_merged_before_it() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.write("check-sleeps", "1");
    f.branch("impl/a", &[("a_file.txt", "x\n")]);
    f.branch("spec/b", &[("ARCHITECTURE.md", "# b\n")]);
    // Passes except on `spec/b`'s merge commit when its first parent holds `a_file.txt`.
    let gate = f.path("bin/gate-b");
    common::write_exec(
        &gate,
        "#!/usr/bin/env bash\n\
         if [ \"$(git log -1 --format=%s)\" = \"Merge spec/b into main\" ] && \
         git cat-file -e HEAD^1:a_file.txt 2>/dev/null; then\n\
         echo 'gate/approval: needs review'; exit 1; fi\n\
         echo code\n",
    );
    let gate = gate.display().to_string();
    let env = f.env(&[("CLM_GATE", &gate)]);
    let held = hold(&f);
    let (a, b) = std::thread::scope(|s| {
        let a = s.spawn(|| clm_land::land(&f.work(), Some("impl/a"), &env));
        wait_for_tickets(&f.work(), 1);
        let b = s.spawn(|| clm_land::land(&f.work(), Some("spec/b"), &env));
        wait_for_tickets(&f.work(), 2);
        drop(held);
        (a.join().unwrap(), b.join().unwrap())
    });
    a.unwrap();
    let why = common::refused(b);
    assert!(why.contains("gate/approval"), "{why}");
    assert_eq!(starts(&f), 1, "{:?}", f.lines("check-order.log"));
}

#[test]
fn the_selection_covers_what_two_of_them_share() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.write("readers-out", "file crates/a/tests/r.rs\n");
    f.write("overlap-out", "file crates/c/tests/t.rs\n");
    f.branch("impl/a", &[("a_file.txt", "x\n")]);
    f.branch("impl/b", &[("b_file.txt", "x\n")]);
    f.mark_checked("origin/impl/a");
    f.mark_checked("origin/impl/b");
    let held = hold(&f);
    let outs = land_all(&f, &["a", "b"], held);
    assert!(outs.iter().all(|o| o.is_ok()), "{outs:?}");
    let copy = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert!(
        copy.lines().any(|l| l == "file crates/c/tests/t.rs"),
        "{copy}"
    );
    assert_eq!(starts(&f), 1, "{:?}", f.lines("check-order.log"));
}

#[test]
fn a_land_with_another_check_environment() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.write("check-sleeps", "1");
    f.branch("impl/a", &[("a_file.txt", "x\n")]);
    let text = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    f.branch(
        "impl/b",
        &[("clm.toml", &format!("{text}\n[env]\nA = \"2\"\n"))],
    );
    let held = hold(&f);
    let outs = land_all(&f, &["a", "b"], held);
    assert!(outs.iter().all(|o| o.is_ok()), "{outs:?}");
    assert_eq!(starts(&f), 2, "{:?}", f.lines("check-order.log"));
}

#[test]
fn a_ticket_that_names_no_branch_is_left_to_wait() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.write("check-sleeps", "1");
    f.branch("impl/a", &[("a_file.txt", "x\n")]);
    let held = hold(&f);
    // A ticket of an older `clm`: empty, locked while its run lives.
    let queue = f.work().join(".clm/land.queue");
    std::fs::create_dir_all(&queue).unwrap();
    std::fs::write(queue.join("next"), "1").unwrap();
    let older = queue.join("00000000000000000000-ordinary-p5");
    let file = std::fs::File::create(&older).unwrap();
    file.lock().unwrap();
    let env = f.env(&[]);
    let out = std::thread::scope(|s| {
        let land = s.spawn(|| clm_land::land(&f.work(), Some("impl/a"), &env));
        wait_for_tickets(&f.work(), 2);
        drop(held);
        // The older ticket's turn is first, and its run never takes the lock: it ends.
        std::thread::sleep(Duration::from_millis(500));
        drop(file);
        land.join().unwrap()
    });
    out.unwrap();
    assert_eq!(starts(&f), 1);
    assert_eq!(f.landed().len(), 1);
}

/// A `[land] check` that logs `start <tree>` and then, given `body` (a shell fragment run in
/// the checked tree), exits with its status.
fn check_with(f: &Fixture, body: &str) {
    let text = format!(
        "#!/bin/sh\necho \"start $(git rev-parse 'HEAD^{{tree}}')\" >> '{log}'\n{body}\n",
        log = f.path("check-order.log").display()
    );
    common::write_exec(&f.path("bin/check"), &text);
}

/// A `clm affected` that names `crates/<n>/tests/t.rs` for a branch that adds `<n>_file.txt`
/// to the tree, the file `--base` selects (`--overlap` and `--readers` print nothing).
fn affected_by_files(f: &Fixture, names: &[&str]) -> String {
    let mut body = String::from(
        "#!/bin/sh\ncase \" $* \" in *--overlap*|*--readers*|*--why*) exit 0;; esac\n",
    );
    for n in names {
        body.push_str(&format!(
            "[ -e {n}_file.txt ] && echo 'file crates/{n}/tests/t.rs'\n"
        ));
    }
    body.push_str("exit 0\n");
    let path = f.path("bin/affected-files");
    common::write_exec(&path, &body);
    path.display().to_string()
}

fn branches(f: &Fixture, names: &[&str]) {
    for n in names {
        f.branch(&format!("impl/{n}"), &[(&format!("{n}_file.txt"), "x\n")]);
    }
}

fn land_all_env(
    f: &Fixture,
    names: &[&str],
    env: &(impl Fn(&str) -> Option<String> + Sync + 'static),
) -> Vec<Result<clm_land::Output, clm_land::Error>> {
    let held = hold(f);
    std::thread::scope(|s| {
        let mut runs = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let work = f.work();
            let branch = format!("impl/{name}");
            runs.push(s.spawn(move || clm_land::land(&work, Some(&branch), env)));
            wait_for_tickets(&f.work(), i + 1);
        }
        drop(held);
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    })
}

/// The merge commits `main` holds for these branches, newest first.
fn on_main(f: &Fixture) -> Vec<String> {
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    subjects(f)
        .into_iter()
        .filter_map(|s| {
            s.strip_prefix("Merge ")?
                .strip_suffix(" into main")
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn the_failed_step_points_at_one_land() {
    // req:land/failed-batch@b38c0772
    let f = Fixture::new("");
    check_with(
        &f,
        "if [ -e c_file.txt ]; then echo '::group::unit'; echo '::clm-selected::crates/c/tests/t.rs'; exit 1; fi",
    );
    branches(&f, &["a", "b", "c"]);
    let affected = affected_by_files(&f, &["c"]);
    let env = f.env(&[("CLM_AFFECTED", &affected)]);
    let mut outs = land_all_env(&f, &["a", "b", "c"], &env);
    let c = common::refused(outs.pop().unwrap());
    assert!(c.contains(&f.check_command()), "{c}");
    assert!(outs.iter().all(|o| o.is_ok()), "{outs:?}");
    assert_eq!(on_main(&f)[..2], ["impl/b", "impl/a"]);
    assert!(!on_main(&f).contains(&"impl/c".to_string()));
    // Together, then the two together again, then `impl/c` alone.
    assert_eq!(starts(&f), 3, "{:?}", f.lines("check-order.log"));
}

#[test]
fn no_step_says_who() {
    // req:land/failed-batch@b38c0772
    let f = Fixture::new("");
    check_with(&f, "[ ! -e c_file.txt ]");
    branches(&f, &["a", "b", "c"]);
    let mut outs = land_all(&f, &["a", "b", "c"], hold(&f));
    // `main` is unchanged by the first run: it is `a`'s own, alone, that lands.
    let c = common::refused(outs.pop().unwrap());
    assert!(c.contains(&f.check_command()), "{c}");
    let a = outs.remove(0).unwrap();
    assert!(
        a.warnings
            .iter()
            .any(|w| w.contains("lands taken together failed")),
        "{:?}",
        a.warnings
    );
    assert!(outs.pop().unwrap().is_ok());
    assert_eq!(on_main(&f)[..2], ["impl/b", "impl/a"]);
    assert_eq!(starts(&f), 4, "{:?}", f.lines("check-order.log"));
}

#[test]
fn the_holders_own_branch_breaks_the_check() {
    // req:land/failed-batch@b38c0772
    let f = Fixture::new("");
    check_with(&f, "[ ! -e a_file.txt ]");
    branches(&f, &["a", "b", "c"]);
    let mut outs = land_all(&f, &["a", "b", "c"], hold(&f));
    assert!(outs.pop().unwrap().is_ok());
    assert!(outs.pop().unwrap().is_ok());
    let a = common::refused(outs.pop().unwrap());
    assert!(a.contains(&f.check_command()), "{a}");
    assert_eq!(on_main(&f)[..2], ["impl/c", "impl/b"]);
    assert_eq!(starts(&f), 4, "{:?}", f.lines("check-order.log"));
}

#[test]
fn the_failed_step_names_a_path_of_no_land() {
    // req:land/failed-batch@b38c0772
    let f = Fixture::new("");
    check_with(
        &f,
        "if [ -e c_file.txt ]; then echo '::group::docs'; echo '::clm-selected::docs/x.md'; exit 1; fi",
    );
    branches(&f, &["a", "b", "c"]);
    let affected = affected_by_files(&f, &["c"]);
    let env = f.env(&[("CLM_AFFECTED", &affected)]);
    let mut outs = land_all_env(&f, &["a", "b", "c"], &env);
    assert!(common::refused(outs.pop().unwrap()).contains(&f.check_command()));
    assert!(outs.iter().all(|o| o.is_ok()), "{outs:?}");
    // Together, and then each of the three alone.
    assert_eq!(starts(&f), 4, "{:?}", f.lines("check-order.log"));
}

#[test]
fn more_than_four_suspects_are_halved() {
    // req:land/failed-batch@b38c0772
    let f = Fixture::new("");
    check_with(&f, "[ ! -e f_file.txt ]");
    let names = ["a", "b", "c", "d", "e", "f"];
    branches(&f, &names);
    let mut outs = land_all(&f, &names, hold(&f));
    assert!(common::refused(outs.pop().unwrap()).contains(&f.check_command()));
    assert!(outs.iter().all(|o| o.is_ok()), "{outs:?}");
    assert_eq!(starts(&f), 6, "{:?}", f.lines("check-order.log"));
    let main = on_main(&f);
    assert_eq!(
        main[..5],
        ["impl/e", "impl/d", "impl/c", "impl/b", "impl/a"]
    );
    // `a`, `b` and `c` landed on one check: their records share a key.
    let keys: Vec<_> = f
        .landed()
        .into_iter()
        .filter_map(|b| match b {
            Body::Landed {
                branch, code_key, ..
            } if ["impl/a", "impl/b", "impl/c"].contains(&branch.as_str()) => code_key,
            _ => None,
        })
        .collect();
    assert_eq!(keys.len(), 3);
    assert!(keys.windows(2).all(|w| w[0] == w[1]), "{keys:?}");
}

#[test]
fn a_land_alone_lands_as_before() {
    // req:land/batched-lands@dbe8f4c4
    let f = Fixture::new("");
    f.branch("impl/a", &[("a_file.txt", "x\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(landed_lines(&out).starts_with("landed "));
    let landed = f.landed();
    assert_eq!(landed.len(), 1);
    let Body::Landed {
        check_seconds,
        recheck,
        ..
    } = &landed[0]
    else {
        panic!()
    };
    assert!(check_seconds.is_some());
    assert_eq!(*recheck, Some(Recheck::Selected));
    assert_eq!(starts(&f), 1);
    assert_eq!(f.parents(&f.work(), "origin/main").len(), 2);
}
