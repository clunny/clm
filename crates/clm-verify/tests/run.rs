//! The run of `[verify] check` on `origin/main`'s head and its record (verify/verify-run).

mod common;

use clm_ledger::Body;
use common::Fixture;

fn every_land() -> Fixture {
    Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ))
}

#[test]
fn a_passing_run() {
    // req:verify/verify-run@7308e44a
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
    // req:verify/verify-run@7308e44a
    let f = every_land();
    f.land(&[("a.txt", "1\n")], "one");
    assert_eq!(f.due().code, 0);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
}

#[test]
fn a_failing_run() {
    // req:verify/verify-run@7308e44a
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
fn a_land_waits_for_the_slow_check() {
    // req:verify/verify-run@7308e44a
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
        let run = clm_land::run_check(
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
    // req:verify/verify-run@7308e44a
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
    // req:verify/verify-run@7308e44a
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    // A spec that doesn't load: `clm affected` fails, and the check runs without a selection.
    f.land(&[("specs/b/SPEC.md", "not a spec\n")], "break b");
    let out = f.due();
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
}

#[test]
fn one_at_a_time() {
    // req:verify/verify-run@7308e44a
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
    // req:verify/verify-run@7308e44a
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
        assert!(out.status.success() && said.contains("1 passed"), "{said}");
        return;
    }
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&green, true);
    let first = f.land(&[("crates/a/src/lib.rs", "x\n")], "first");
    let head = f.land(&[("crates/b/src/lib.rs", "y\n")], "second");
    f.write("check-prints", "");
    f.write("fail-path", "crates/a/src/lib.rs");
    // Another process holds the check lock for a second, so the runner has a line to say.
    // The verify tree's own lock is held too, so its waiting line is said as well.
    let places = f.verify_tree().parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&places).unwrap();
    let locks: Vec<std::fs::File> = ["check.lock", "verify.lock"]
        .iter()
        .map(|name| {
            let lock = std::fs::File::create(places.join(name)).unwrap();
            lock.lock().unwrap();
            lock
        })
        .collect();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        drop(locks);
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
