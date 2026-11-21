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
    // req:check.health/unexercised-code@3b36e1e7
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
    // req:check.health/unexercised-code@3b36e1e7
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
    // req:check.health/unexercised-code@3b36e1e7
    let dir = repo();
    let out = report_with(dir.path(), &settings(12000, 40000));
    assert_eq!(
        section(&out, "unexercised"),
        ["health/unexercised: no lcov file configured"],
        "{out}"
    );
}

#[test]
fn scenario_records_merged() {
    // req:check.health/unexercised-code@3b36e1e7
    let parent = tempfile::TempDir::new().unwrap();
    let r = parent.path().join("r");
    std::fs::create_dir(&r).unwrap();
    common::git(&r, &["init", "-q", "-b", "main"]);
    write(&r, "specs/SPEC.md", common::ROOT);
    write(&r, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", &["x"]));
    let lib = r.join("crates/a/src/lib.rs");
    write(
        &r,
        "cov.info",
        &format!(
            "DA:7,0\n\
             SF:{lib}\nDA:4,0\nDA:8,0,abc\nDA:11,18446744073709551615\nend_of_record\n\
             SF:{lib}\nDA:4,3\nDA:x,0\nDA:9,\nDA:+10,0\nDA:11,1\nDA:12,18446744073709551616\nend_of_record\n\
             SF:crates/a/src/b.rs\nDA:2,0\nend_of_record\n\
             SF:/elsewhere/crates/a/src/b.rs\nDA:3,0\nend_of_record\n\
             SF:r/crates/a/src/c.rs\nDA:1,0\nend_of_record\n",
            lib = lib.display()
        ),
    );
    // `clm health --root r` run from the directory holding `r`; the test runs in a process
    // of its own (`one-runner-for-the-rust-tests`).
    std::env::set_current_dir(parent.path()).unwrap();
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::health_command(
        ["--root", "r"],
        std::path::Path::new("/nonexistent"),
        Ok(with_lcov("cov.info")),
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    let out = String::from_utf8(stdout).unwrap();
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        section(&out, "unexercised"),
        [
            "health/unexercised: 3 lines",
            "crates/a/src/b.rs:2: health/unexercised: 1 lines no test runs",
            "crates/a/src/lib.rs:8: health/unexercised: 1 lines no test runs",
            "crates/a/src/lib.rs:10: health/unexercised: 1 lines no test runs",
        ],
        "{out}"
    );
}
