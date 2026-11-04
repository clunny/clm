mod common;

use common::{git, repo, report, report_with, section, settings, spec, write};

/// The signals in the order of the Contract's table.
const SIGNALS: [&str; 18] = [
    "check",
    "pending",
    "oversized",
    "lapsed-claim",
    "stalled-change",
    "unanswered",
    "uncited-decision",
    "unserved",
    "history-words",
    "design-heavy",
    "outlier-size",
    "unexercised",
    "workaround",
    "rewritten",
    "churn",
    "many-scenarios",
    "many-exceptions",
    "reconsider",
];

#[test]
fn every_signal_prints_its_summary_in_the_table_order() {
    // req:check.health/health-report@a5dd24b1
    let dir = repo();
    let out = report(dir.path());
    let summaries: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("health/"))
        .map(|l| l["health/".len()..].split(':').next().unwrap())
        .collect();
    assert_eq!(summaries, SIGNALS, "{out}");
    assert!(out.contains("health/check: 0 violations\n"), "{out}");
}

#[test]
fn model_violations_are_findings_under_check() {
    // req:check.health/health-report@a5dd24b1
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[\"src/**\"]", &["x"]));
    write(d, "src/t.rs", &format!("// {}:a/ghost@00000000\n", "req"));
    git(d, &["add", "-A"]);
    let out = report(d);
    let check = section(&out, "check");
    assert_eq!(check[0], "health/check: 1 violations", "{out}");
    assert!(
        check[1].starts_with("src/t.rs:1: health/check: check/dangling-markers: "),
        "{out}"
    );
}

#[test]
fn a_missing_root_spec_is_refused() {
    // req:check.health/health-report@a5dd24b1
    let dir = tempfile::TempDir::new().unwrap();
    git(dir.path(), &["init", "-q"]);
    let e = clm_health::report(dir.path(), &settings(1, 1), 4).unwrap_err();
    assert_eq!(e.exit_code(), 1);
    assert!(e.to_string().contains("specs/SPEC.md"), "{e}");
}

#[test]
fn scenario_pending_and_oversized() {
    // req:check.health/state-signals@1f6b7a1b
    // req:check.health/says-component@15cdb4fa
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[]", &["x"]));
    write(d, "specs/b/SPEC.md", &spec("[\"src/b/**\"]", &[]));
    write(d, "src/b/code.txt", &"x".repeat(8000));
    git(d, &["add", "-A"]);
    let out = report_with(d, &settings(12000, 1000));
    assert!(
        out.contains(
            "specs/a/SPEC.md: health/pending: 1 pending (1 unbuilt, 0 behind, 0 obsolete)\n"
        ),
        "{out}"
    );
    let oversized = section(&out, "oversized");
    assert_eq!(oversized.len(), 2, "{out}");
    assert!(
        oversized[1].starts_with("specs/b/SPEC.md: health/oversized: spec ")
            && oversized[1].ends_with("/12000, code 2000/1000"),
        "{out}"
    );
    assert_eq!(section(&out, "pending")[0], "health/pending: 1 components");
    assert!(
        section(&out, "pending")[1].starts_with("specs/a/SPEC.md: "),
        "{out}"
    );
}

#[test]
fn a_violation_the_model_reports_is_a_finding_under_check() {
    // req:check.health/health-report@a5dd24b1
    let dir = repo();
    write(dir.path(), "specs/a/SPEC.md", "no frontmatter\n");
    let out = report(dir.path());
    assert!(
        out.contains("specs/a/SPEC.md: health/check: model/parse-node-specs: "),
        "{out}"
    );
}

#[test]
fn a_git_failure_is_exit_2() {
    // req:check.health/health-report@a5dd24b1
    let dir = tempfile::TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", common::ROOT);
    let e = clm_health::report(dir.path(), &settings(1, 1), 4).unwrap_err();
    assert_eq!(e.exit_code(), 2, "{e}");
}

#[test]
fn scenario_the_budgets_by_their_name() {
    // req:check.health/component-size-field@7cb5ae48
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec("[]", &["x"]));
    git(d, &["add", "-A"]);
    let mut s = settings(12000, 40000);
    s.component_size = clm_check::Budgets {
        spec_tokens: 1,
        code_tokens: 40000,
    };
    let out = report_with(d, &s);
    assert!(
        section(&out, "oversized")
            .iter()
            .any(|l| l.starts_with("specs/a/SPEC.md: ")),
        "{out}"
    );
}
