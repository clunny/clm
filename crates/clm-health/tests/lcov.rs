mod common;

use common::{repo, report_with, section, settings, spec, write};

fn with_lcov(lcov: &str) -> clm_health::Settings {
    clm_health::Settings {
        lcov: Some(lcov.into()),
        ..settings(12000, 40000)
    }
}

#[test]
fn scenario_a_run_of_unexercised_lines() {
    // req:check.health/unexercised-code@2c85f1a2
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", &["x"]));
    let absolute = d.join("crates/a/src/lib.rs");
    write(
        d,
        "cov.info",
        &format!(
            "TN:\nSF:{}\nFN:3,f\nDA:3,1\nDA:4,0\nDA:5,0\nDA:6,2\nDA:9,0\nLH:2\nLF:5\nend_of_record\nSF:vendor/x.rs\nDA:1,0\nend_of_record\n",
            absolute.display()
        ),
    );
    let out = report_with(d, &with_lcov("cov.info"));
    assert_eq!(
        section(&out, "unexercised"),
        [
            "health/unexercised: 3 lines",
            "crates/a/src/lib.rs:4-5: health/unexercised: 2 lines no test runs",
            "crates/a/src/lib.rs:9: health/unexercised: 1 lines no test runs",
        ],
        "{out}"
    );
}

#[test]
fn scenario_no_file() {
    // req:check.health/unexercised-code@2c85f1a2
    let dir = repo();
    let out = report_with(dir.path(), &with_lcov("cov.info"));
    assert_eq!(
        section(&out, "unexercised"),
        ["health/unexercised: cov.info not found"],
        "{out}"
    );
}

#[test]
fn no_lcov_file_configured() {
    // req:check.health/unexercised-code@2c85f1a2
    let dir = repo();
    let out = report_with(dir.path(), &settings(12000, 40000));
    assert_eq!(
        section(&out, "unexercised"),
        ["health/unexercised: no lcov file configured"],
        "{out}"
    );
}
