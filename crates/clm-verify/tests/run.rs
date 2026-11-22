//! The run of `[verify] check` on `origin/main`'s head and its record (verify/verify-run).

mod common;

use clm_ledger::Body;
use common::{Fixture, in_child, write_exec};

fn every_land() -> Fixture {
    Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ))
}

#[test]
fn a_passing_run() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let land = f.land(&[("crates/a/src/lib.rs", "x\n")], "two");
    let out = f.due();
    let short = &land[..7];
    assert!(
        out.line.starts_with(&format!("verify passed {short}")) && out.line.ends_with(" 1 lands"),
        "{}",
        out.line
    );
    assert_eq!(out.code, 0);
    assert_eq!(
        f.lines("check-pwd.log"),
        [f.verify_tree().display().to_string()]
    );
    assert_eq!(f.lines("check-head.log"), std::slice::from_ref(&land));
    assert_eq!(f.lines("check-selection.log"), ["set"]);
    let selection = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert!(
        selection.contains("file crates/a/src/lib.rs"),
        "{selection}"
    );
    assert!(selection.contains("node a\n"), "{selection}");
    let records = f.records();
    let Body::Verified {
        commit,
        passed,
        since,
        lands,
        culprit,
        components,
        run,
        ..
    } = records.last().unwrap()
    else {
        panic!("{records:?}");
    };
    assert_eq!(commit, &land);
    assert!(passed);
    assert_eq!(since.as_deref(), Some(green.as_str()));
    assert_eq!(lands, &std::slice::from_ref(&land));
    assert_eq!((culprit, components, run), (&None, &vec![], &None));
    // A passing run keeps no log.
    assert!(
        !f.work()
            .join(".clm/verify-logs")
            .join(format!("{short}.log"))
            .exists()
    );
}

#[test]
fn a_run_with_no_green_record_selects_nothing() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    f.land(&[("a.txt", "1\n")], "one");
    assert_eq!(f.due().code, 0);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
}

#[test]
fn a_failing_run() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let land = f.land(&[("crates/b/src/lib.rs", "x\n")], "two");
    f.write("check-fails", "");
    let out = f.due();
    assert_eq!(
        out.line,
        format!("verify failed {0} culprit {0} components b", &land[..7])
    );
    assert_eq!(out.code, 1);
    let log = f
        .work()
        .join(format!(".clm/verify-logs/{}.log", &land[..7]));
    assert!(log.exists());
    assert!(
        out.notes
            .iter()
            .any(|n| n.contains(&log.display().to_string())),
        "{:?}",
        out.notes
    );
    let records = f.records();
    let Body::Verified {
        passed,
        culprit,
        components,
        ..
    } = records.last().unwrap()
    else {
        panic!("{records:?}");
    };
    assert!(!passed);
    assert_eq!(culprit.as_deref(), Some(land.as_str()));
    assert_eq!(components, &["b".to_string()]);
}

#[test]
fn a_red_runs_report() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let land = f.land(&[("crates/b/src/lib.rs", "x\n")], "two");
    f.write("check-prints", "");
    f.write("check-fails", "");
    let out = f.due();
    assert_eq!(out.code, 1);
    assert!(out.line.starts_with("verify failed"), "{}", out.line);
    let check = f.path("bin/check").display().to_string();
    let at = out
        .notes
        .iter()
        .position(|n| n == &format!("last 40 lines of `{check}`:"))
        .unwrap_or_else(|| panic!("{:?}", out.notes));
    // The last 40 of the 1000 lines the check prints, then the log's path.
    let tail: Vec<String> = (961..=1000).map(|n| n.to_string()).collect();
    let after = &out.notes[at + 1..];
    assert_eq!(after.len(), 41, "{after:?}");
    assert_eq!(&after[..40], tail.as_slice());
    let log = f
        .work()
        .join(format!(".clm/verify-logs/{}.log", &land[..7]));
    assert!(after[40].contains(&log.display().to_string()), "{after:?}");
    // The check marks no step.
    assert!(!out.notes.iter().any(|n| n.starts_with("the step open")));
}

#[test]
fn a_red_runs_report_names_the_open_step() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("crates/b/src/lib.rs", "x\n")], "two");
    f.write("check-group", "unit");
    f.write("check-prints", "");
    f.write("check-fails", "");
    let out = f.due();
    let step = out
        .notes
        .iter()
        .position(|n| {
            n.starts_with("the step open when it ended is `unit`, lines 1 to 1001 of its output")
        })
        .unwrap_or_else(|| panic!("{:?}", out.notes));
    let first = out
        .notes
        .iter()
        .position(|n| n.starts_with("last 40 lines of"))
        .unwrap();
    // In order: the tail, the step, the log's path.
    assert!(
        first < step && step + 2 == out.notes.len(),
        "{:?}",
        out.notes
    );
    assert!(
        out.notes[step + 1].contains("verify-logs"),
        "{:?}",
        out.notes
    );
}

#[test]
fn a_red_run_answered_from_the_ledger_prints_no_report() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let land = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&land, false);
    let out = f.run(&clm_verify::Request::Commit(land));
    assert_eq!(out.code, 1);
    assert!(out.notes.iter().all(|n| !n.starts_with("last 40 lines")));
}

#[test]
fn a_land_waits_for_the_slow_check() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    f.land(&[("a.txt", "1\n")], "one");
    f.write("sleep", "2");
    // A land's `[land] check` goes through the same runner, so under the same lock.
    let landing = format!("echo land >> '{}'", f.path("check-order.log").display());
    std::thread::scope(|s| {
        let verifying = s.spawn(|| f.due());
        for _ in 0..200 {
            if f.lines("check-order.log") == ["start"] {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert_eq!(f.lines("check-order.log"), ["start"]);
        let run = clm_land::run_check_as(
            "clm land",
            &landing,
            &f.work(),
            None,
            &clm_land::CheckEnv::default(),
            None,
        )
        .unwrap();
        assert!(run.passed);
        assert_eq!(verifying.join().unwrap().code, 0);
    });
    assert_eq!(f.lines("check-order.log"), ["start", "end", "land"]);
}

#[test]
fn two_runs_never_move_the_tree_under_each_other() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let old = f.land(&[("a.txt", "1\n")], "old");
    let new = f.land(&[("a.txt", "2\n")], "new");
    f.write("sleep", "1");
    std::thread::scope(|s| {
        let one = s.spawn(|| f.run(&clm_verify::Request::Commit(old.clone())));
        let two = s.spawn(|| f.run(&clm_verify::Request::Commit(new.clone())));
        assert_eq!(one.join().unwrap().code, 0);
        assert_eq!(two.join().unwrap().code, 0);
    });
    // Each run's tree is the same at its end as at its start.
    let trees = f.lines("check-trees.log");
    assert_eq!(trees.len(), 4, "{trees:?}");
    for pair in trees.chunks(2) {
        let start = pair[0].strip_prefix("start ").expect("start");
        assert_eq!(pair[1], format!("end {start}"), "{trees:?}");
    }
}

#[test]
fn a_selection_that_cannot_be_made_runs_every_test() {
    // req:verify/verify-run@a6220530
    if let Some(stderr) = in_child("a_selection_that_cannot_be_made_runs_every_test") {
        assert!(
            stderr
                .lines()
                .any(|l| l.starts_with("clm affected failed: ")
                    && l.ends_with("; the check runs every test")),
            "{stderr}"
        );
        return;
    }
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    // A spec that doesn't load: `clm affected` fails, and the check runs without a selection.
    f.land(&[("specs/b/SPEC.md", "not a spec\n")], "break b");
    let out = f.due();
    assert_eq!(out.code, 0, "{}", out.line);
    assert!(out.line.starts_with("verify passed"), "{}", out.line);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
    assert!(matches!(
        f.records().last(),
        Some(Body::Verified { passed: true, .. })
    ));
}

/// Asserts that the check last ran in the verify tree on `origin/main`'s head, that the
/// outcome passed over one land, and that git lists the tree as a worktree.
fn ran_in_a_listed_tree(f: &Fixture, out: &clm_verify::Outcome) {
    let head = f.git(&f.work(), &["rev-parse", "origin/main"]);
    assert_eq!(out.line, format!("verify passed {} 1 lands", &head[..7]));
    assert_eq!(out.code, 0);
    assert_eq!(f.lines("check-head.log").last(), Some(&head));
    let tree = f.verify_tree().display().to_string();
    assert_eq!(f.lines("check-pwd.log").last(), Some(&tree));
    let listed = f.git(&f.work(), &["worktree", "list", "--porcelain"]);
    assert!(
        listed.lines().any(|l| l == format!("worktree {tree}")),
        "{listed}"
    );
}

#[test]
fn a_broken_tree_is_remade() {
    // req:verify/verify-run@a6220530
    // A tree from an earlier run whose `.git` file at its top was removed.
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    assert_eq!(f.due().code, 0);
    std::fs::remove_file(f.verify_tree().join(".git")).unwrap();
    f.land(&[("a.txt", "3\n")], "three");
    let out = f.due();
    ran_in_a_listed_tree(&f, &out);
    // A regular file where the tree goes.
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let tree = f.verify_tree();
    std::fs::create_dir_all(tree.parent().unwrap()).unwrap();
    std::fs::write(&tree, "not a tree\n").unwrap();
    f.land(&[("a.txt", "2\n")], "two");
    let out = f.due();
    ran_in_a_listed_tree(&f, &out);
}

#[test]
fn a_record_kept_local() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let head = f.land(&[("a.txt", "2\n")], "two");
    write_exec(
        &f.path("origin.git/hooks/pre-receive"),
        "#!/bin/sh\nwhile read old new ref; do\n  case $ref in refs/heads/clm/ledger/*) exit 1;; esac\ndone\n",
    );
    let out = f.due();
    assert_eq!(out.line, format!("verify passed {} 1 lands", &head[..7]));
    assert_eq!(out.code, 0);
    assert!(
        out.notes
            .iter()
            .any(|n| n == "the record is committed locally; `clm reconnect` pushes it"),
        "{:?}",
        out.notes
    );
    // The clone's own ledger branch holds the record.
    let own = f.git(
        &f.work(),
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/clm/ledger/",
        ],
    );
    let text = f.git(&f.work(), &["show", &format!("{own}:records.jsonl")]);
    let fold = clm_ledger::fold_records(&[(own.as_str(), text.as_str())]);
    assert!(
        fold.records.iter().any(|r| matches!(
            &r.body,
            Body::Verified { commit, passed: true, .. } if *commit == head
        )),
        "{text}"
    );
}

#[test]
fn the_logs_warning_names_clm_verify() {
    // req:verify/verify-run@a6220530
    if let Some(stderr) = in_child("the_logs_warning_names_clm_verify") {
        assert!(
            stderr
                .lines()
                .any(|l| l.starts_with("clm verify: warning: can't write the check's log at ")),
            "{stderr}"
        );
        assert!(
            !stderr.lines().any(|l| l.starts_with("clm land:")),
            "{stderr}"
        );
        return;
    }
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let head = f.land(&[("a.txt", "2\n")], "two");
    let logs = f.work().join(".clm/verify-logs");
    std::fs::create_dir_all(logs.parent().unwrap()).unwrap();
    std::fs::write(&logs, "a file\n").unwrap();
    let out = f.due();
    assert_eq!(out.line, format!("verify passed {} 1 lands", &head[..7]));
    assert_eq!(out.code, 0);
}

#[test]
fn one_at_a_time() {
    // req:verify/verify-run@a6220530
    let f = every_land();
    f.land(&[("a.txt", "1\n")], "one");
    f.write("sleep", "1");
    std::thread::scope(|s| {
        let one = s.spawn(|| f.due());
        let two = s.spawn(|| f.due());
        one.join().unwrap();
        two.join().unwrap();
    });
    assert_eq!(f.lines("check-order.log"), ["start", "end", "start", "end"]);
}

/// Set in the child the scenario below re-runs itself in.
const CHILD: &str = "CLM_TEST_STDERR_FULL";

#[cfg(target_os = "linux")]
#[test]
fn the_verdict_outlives_a_failing_stderr() {
    // req:verify/verify-run@a6220530
    if std::env::var_os(CHILD).is_none() {
        // The scenario runs in a child whose stderr is `/dev/full`.
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "the_verdict_outlives_a_failing_stderr",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stderr(full)
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && said.contains("1 passed"),
            "{}: {said}",
            out.status
        );
        return;
    }
    // The child's stderr is `/dev/full`, so its panic message goes to stdout, where the
    // parent's assertion shows it.
    std::panic::set_hook(Box::new(|info| println!("child panicked: {info}")));
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&green, true);
    let first = f.land(&[("crates/a/src/lib.rs", "x\n")], "first");
    let head = f.land(&[("specs/b/SPEC.md", "not a spec\n")], "second");
    f.write("check-prints", "");
    f.write("fail-path", "crates/a/src/lib.rs");
    // Another process holds the check lock for a second, so the runner has a line to say.
    // The verify tree's own lock is held too, so its waiting line is said as well.
    let places = f.verify_tree().parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&places).unwrap();
    let [check, verify]: [std::fs::File; 2] = ["check.lock", "verify.lock"].map(|name| {
        let lock = std::fs::File::create(places.join(name)).unwrap();
        lock.lock().unwrap();
        lock
    });
    // `verify.lock` goes first, so the runner then waits on `check.lock` and says so.
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        drop(verify);
        std::thread::sleep(std::time::Duration::from_millis(700));
        drop(check);
    });
    let out = f.due();
    releaser.join().unwrap();
    assert_eq!(
        out.line,
        format!(
            "verify failed {} culprit {} components a",
            &head[..7],
            &first[..7]
        )
    );
    let records = f.records();
    let Body::Verified {
        commit,
        passed,
        culprit,
        components,
        ..
    } = records.last().unwrap()
    else {
        panic!("{records:?}");
    };
    assert_eq!(commit, &head);
    assert!(!passed);
    assert_eq!(culprit.as_deref(), Some(first.as_str()));
    assert_eq!(components, &["a".to_string()]);
}
