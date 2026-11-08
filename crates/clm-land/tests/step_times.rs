//! The times a land keeps on its `landed` record (land/step-times).

mod common;

use clm_ledger::{Body, TimedStep};
use common::Fixture;

/// The `check_seconds` and `steps` of the only `landed` record.
fn timing(f: &Fixture) -> (Option<u64>, Vec<TimedStep>) {
    match f.landed().into_iter().next() {
        Some(Body::Landed {
            check_seconds,
            steps,
            ..
        }) => (check_seconds, steps),
        other => panic!("{other:?}"),
    }
}

#[test]
fn two_steps() {
    // req:land/step-times@d247dda8
    let f = Fixture::new("");
    f.write(
        "check-prints",
        "echo ::group::a\necho ::clm-selected::x/y.rs\necho '::clm-selected::x/y.rs  '\nsleep 1\n\
         echo ::endgroup::\necho ::clm-selected::ignored\necho ::group::b\n",
    );
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let (seconds, steps) = timing(&f);
    assert!(seconds.unwrap() >= 1, "{seconds:?}");
    assert_eq!(steps.len(), 2, "{steps:?}");
    assert_eq!(steps[0].name, "a");
    assert!(steps[0].seconds >= 1, "{steps:?}");
    assert_eq!(steps[0].selected_by, ["x/y.rs"]);
    assert_eq!(steps[1].name, "b");
    assert!(steps[1].selected_by.is_empty());
}

#[test]
fn no_steps_marked() {
    // req:land/step-times@d247dda8
    let f = Fixture::new("");
    f.write("check-prints", "echo ok\n");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let (seconds, steps) = timing(&f);
    assert!(seconds.is_some());
    assert!(steps.is_empty());
}

#[test]
fn code_already_checked() {
    // req:land/step-times@d247dda8
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert!(f.lines("check.log").is_empty());
    assert_eq!(timing(&f), (None, vec![]));
}
