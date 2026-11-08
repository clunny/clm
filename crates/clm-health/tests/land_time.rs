mod common;

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod ledger_common;

use std::path::{Path, PathBuf};

use clm_ledger::{Body, LandKind, TimedStep};
use common::{section, settings, write};

fn step(name: &str, seconds: u64, selected_by: &[&str]) -> TimedStep {
    TimedStep {
        name: name.into(),
        seconds,
        selected_by: selected_by.iter().map(|s| s.to_string()).collect(),
    }
}

fn land(root: &Path, home: &Path, commit: &str, steps: Vec<TimedStep>) {
    let check_seconds = steps.iter().map(|s| s.seconds).sum();
    let body = Body::Landed {
        commit: commit.into(),
        branch: "impl/a".into(),
        kind: LandKind::Code,
        review: false,
        fingerprint: "f".into(),
        code_key: None,
        changes: vec![],
        check_seconds: Some(check_seconds),
        steps,
    };
    clm_ledger::append(root, body, &ledger_common::env(home, &[])).unwrap();
}

fn clone() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = ledger_common::clone_with_origin(dir.path(), "a", "A");
    write(&root, "specs/SPEC.md", common::ROOT);
    (dir, root)
}

fn report(root: &Path) -> String {
    let mut s = settings(12000, 40000);
    s.land_check_target_seconds = 60;
    clm_health::report(root, &s, 4).unwrap().to_string()
}

#[test]
fn scenario_slow_steps() {
    // req:check.health/land-time-signal@ba4f73a6
    let (dir, root) = clone();
    for c in ["c1", "c2", "c3"] {
        land(
            &root,
            dir.path(),
            c,
            vec![step("web", 100, &["web/src/main.ts"]), step("fmt", 1, &[])],
        );
    }
    land(&root, dir.path(), "c4", vec![step("fmt", 1, &[])]);
    let out = report(&root);
    assert_eq!(
        section(&out, "land-time"),
        [
            "health/land-time: 3 of 4 timed lands over 60s",
            "web/src/main.ts: health/land-time: web: 3 runs, median 100s, total 300s, selected by web/src/main.ts",
            "clm.toml: health/land-time: fmt: 4 runs, median 1s, total 4s, selected by no file",
        ],
        "{out}"
    );
    let order: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("health/"))
        .map(|l| l.split(':').next().unwrap())
        .collect();
    let at = |s: &str| order.iter().position(|x| *x == s).unwrap();
    assert!(at("health/churn") < at("health/land-time"));
    assert!(at("health/land-time") < at("health/many-scenarios"));
}

#[test]
fn only_the_five_costliest_steps_and_the_three_commonest_paths() {
    // req:check.health/land-time-signal@ba4f73a6
    let (dir, root) = clone();
    let steps = (1..=6)
        .map(|i| {
            let paths: &[&str] = if i == 6 { &["d", "c", "b", "a"] } else { &[] };
            step(&format!("s{i}"), i, paths)
        })
        .collect();
    land(&root, dir.path(), "c1", steps);
    let out = report(&root);
    let lines = section(&out, "land-time");
    assert_eq!(lines.len(), 6, "{out}");
    assert_eq!(
        lines[1],
        "a: health/land-time: s6: 1 runs, median 6s, total 6s, selected by a, b, c"
    );
    assert!(lines[5].contains(": s2:"), "{out}");
    assert!(!out.contains(": s1:"), "{out}");
}

#[test]
fn scenario_no_timings() {
    // req:check.health/land-time-signal@ba4f73a6
    let (_dir, root) = clone();
    let out = report(&root);
    assert_eq!(
        section(&out, "land-time"),
        ["health/land-time: 0 of 0 timed lands over 60s"],
        "{out}"
    );
}
