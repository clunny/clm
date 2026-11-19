//! The gate under each version of its rules (gate/rules-version).

use std::fs;
use std::path::Path;

use clm_gate::{Kind, RULES, evaluate_by};
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) -> String {
    clm_git::git(d, args).unwrap()
}

fn write(d: &Path, rel: &str, content: &str) {
    let path = d.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn commit(d: &Path, message: &str) {
    git(d, &["add", "-A"]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            message,
        ],
    );
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const HEAD: &str = "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n";
const Y: &str = "\n### Requirement: y\nY SHALL hold.\n\n#### Scenario: t\n- ok\n";
const X: &str = "\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: s\n- ok\n";

/// A repository whose `main` holds component `a` with `a/x` and `a/y`, and `pr` checked out
/// from it.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &format!("{HEAD}{X}{Y}"));
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

fn rules(out: &clm_gate::Outcome) -> Vec<&str> {
    out.violations.iter().map(|v| v.rule).collect()
}

#[test]
fn scenario_a_removal_under_each_version() {
    // req:gate/rules-version@aee5eff0
    assert_eq!(RULES, 3);
    let dir = repo();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &format!("{HEAD}{Y}"));
    commit(d, "remove a/x");
    let s = git(d, &["rev-parse", "HEAD"]);
    let old = evaluate_by(d, "main", &s, false, 1).unwrap();
    let now = evaluate_by(d, "main", &s, false, RULES).unwrap();
    assert_eq!((old.kind, old.review), (Kind::Spec, false));
    assert_eq!((now.kind, now.review), (Kind::Spec, true));
    assert!(rules(&old).is_empty(), "{:?}", old.violations);
    assert_eq!(rules(&now), ["gate/approval"]);
    for out in [&old, &now] {
        let lines: Vec<String> = out.cuts.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            ["specs/a/SPEC.md: gate/cut: requirement a/x is gone"]
        );
    }
    // `evaluate_at` is the current version
    let at = clm_gate::evaluate_at(d, "main", &s, false).unwrap();
    assert_eq!((at.kind, at.review), (now.kind, now.review));
}

#[test]
fn scenario_a_changed_outcome_under_each_version() {
    // req:gate/rules-version@aee5eff0
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    let first = |then: &str| {
        format!(
            "{HEAD}\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: first\n- GIVEN g\n- WHEN w\n- THEN {then}\n"
        )
    };
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &first("it exits 1"));
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "s"]);
    write(d, "specs/a/SPEC.md", &first("it lands"));
    commit(d, "reverse it");
    let v2 = evaluate_by(d, "main", "s", false, 2).unwrap();
    let now = evaluate_by(d, "main", "s", false, RULES).unwrap();
    assert_eq!((v2.kind, v2.review), (Kind::Spec, false));
    assert_eq!((now.kind, now.review), (Kind::Spec, true));
    assert!(rules(&v2).is_empty(), "{:?}", v2.violations);
    assert_eq!(rules(&now), ["gate/approval"]);
    for out in [&v2, &now] {
        let lines: Vec<String> = out.cuts.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            ["specs/a/SPEC.md: gate/cut: scenario \"first\" of a/x changes its outcome"]
        );
    }
    // version 1 too lists it and needs no review
    let v1 = evaluate_by(d, "main", "s", false, 1).unwrap();
    assert_eq!((v1.review, v1.cuts.len()), (false, 1));

    // under version 2 a changed outcome beside another cut still needs review
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/a/SPEC.md", &format!("{}{Y}", first("it exits 1")));
    commit(d, "add y");
    git(d, &["checkout", "-q", "-b", "t"]);
    write(d, "specs/a/SPEC.md", &first("it lands"));
    commit(d, "reverse it and remove y");
    let v2 = evaluate_by(d, "main", "t", false, 2).unwrap();
    let lines: Vec<String> = v2.cuts.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [
            "specs/a/SPEC.md: gate/cut: requirement a/y is gone",
            "specs/a/SPEC.md: gate/cut: scenario \"first\" of a/x changes its outcome",
        ]
    );
    assert!(v2.review);
}

#[test]
fn specs_that_dont_load_need_no_review_under_version_1() {
    // req:gate/rules-version@aee5eff0
    let dir = repo();
    let d = dir.path();
    // a requirement with no scenario doesn't load
    write(
        d,
        "specs/a/SPEC.md",
        &format!("{HEAD}{X}{Y}\n### Requirement: z\nZ SHALL hold.\n"),
    );
    commit(d, "broken");
    let old = evaluate_by(d, "main", "HEAD", false, 1).unwrap();
    let now = evaluate_by(d, "main", "HEAD", false, 2).unwrap();
    assert_eq!((old.review, now.review), (false, true));
    assert_eq!(old.cuts, now.cuts);
    assert_eq!(now.cuts.len(), 1);

    // an exemption it adds still needs review under version 1, read from the text
    let exempt = "owners: []\nexemptions:\n  - rule: skipped-tests\n    files: [\"src/a/**\"]\n    until: backlog/x\n";
    write(
        d,
        "specs/a/SPEC.md",
        &format!("{HEAD}{X}{Y}\n### Requirement: z\nZ SHALL hold.\n")
            .replace("owners: []\n", exempt),
    );
    write(
        d,
        "backlog/x.md",
        "---\nkind: deferred\ncomponents: [a]\n---\nEnds it.\n",
    );
    commit(d, "exempt");
    let old = evaluate_by(d, "main", "HEAD", false, 1).unwrap();
    assert!(old.review, "{:?}", old.violations);
}

#[test]
fn scenario_the_same_verdict_under_both() {
    // req:gate/rules-version@aee5eff0
    let dir = repo();
    let d = dir.path();
    write(d, "ARCHITECTURE.md", "invariants\n");
    commit(d, "architecture");
    for version in [1, RULES] {
        let out = evaluate_by(d, "main", "HEAD", false, version).unwrap();
        assert_eq!((out.kind, out.review), (Kind::Spec, true), "{version}");
        assert_eq!(rules(&out), ["gate/approval"]);
    }
}

#[test]
fn scenario_a_version_this_gate_doesnt_know() {
    // req:gate/rules-version@aee5eff0
    let dir = repo();
    let d = dir.path();
    for version in [RULES + 1, 0] {
        let e = evaluate_by(d, "main", "HEAD", false, version).unwrap_err();
        assert!(
            e.contains(&version.to_string()) && e.contains(&format!("RULES ({RULES})")),
            "{e}"
        );
    }
}
