//! The line a violation prints as, which every crate that prints violations gives.

use std::path::PathBuf;

use clm_model::Violation;

// req:model/violation-line@ebe3aa0d
#[test]
fn one_violation() {
    let v = Violation {
        path: PathBuf::from("specs/a/SPEC.md"),
        rule: "check/x",
        message: "m".to_string(),
    };
    assert_eq!(v.to_string(), "specs/a/SPEC.md: check/x: m");
}
