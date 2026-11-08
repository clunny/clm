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
    // req:verify/verify-run@005554c5
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
    // req:verify/verify-run@005554c5
    let f = every_land();
    f.land(&[("a.txt", "1\n")], "one");
    assert_eq!(f.due().code, 0);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
}

#[test]
fn a_failing_run() {
    // req:verify/verify-run@005554c5
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
    // req:verify/verify-run@005554c5
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
fn one_at_a_time() {
    // req:verify/verify-run@005554c5
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
