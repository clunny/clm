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
    // req:gate/rules-version@373963b2
    assert_eq!(RULES, 6);
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
    // req:gate/rules-version@373963b2
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
fn scenario_a_sentence_of_behaviour_that_holds_a_phrase_under_each_version() {
    // req:gate/rules-version@373963b2
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    let a = |statement: &str| {
        format!("{HEAD}\n### Requirement: x\n{statement}\n\n#### Scenario: s\n- ok\n")
    };
    let one = "The command SHALL print the list.";
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &a(&format!(
            "{one} The directory SHALL be deleted once nothing else is left in it."
        )),
    );
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "s"]);
    write(d, "specs/a/SPEC.md", &a(one));
    commit(d, "drop the second sentence");
    let v3 = evaluate_by(d, "main", "s", false, 3).unwrap();
    let now = evaluate_by(d, "main", "s", false, RULES).unwrap();
    assert_eq!((v3.kind, v3.review), (Kind::Spec, false));
    assert_eq!((now.kind, now.review), (Kind::Spec, true));
    assert!(rules(&v3).is_empty(), "{:?}", v3.violations);
    assert_eq!(rules(&now), ["gate/approval"]);
    for out in [&v3, &now] {
        let lines: Vec<String> = out.cuts.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            ["specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2"]
        );
    }
    // versions 4 and 5 count it as today's version does
    for version in [4, 5] {
        assert!(
            evaluate_by(d, "main", "s", false, version).unwrap().review,
            "{version}"
        );
    }
    // under versions 2 and 3, a sentence holding a phrase that is counted as before still
    // needs review when lost: one holding no phrase
    git(d, &["checkout", "-q", "-b", "t", "main"]);
    write(
        d,
        "specs/a/SPEC.md",
        &a("The directory SHALL be deleted once nothing else is left in it."),
    );
    commit(d, "drop the first sentence");
    for version in [2, 3] {
        assert!(
            evaluate_by(d, "main", "t", false, version).unwrap().review,
            "{version}"
        );
    }
    // and under version 3 a count that falls only as it counted then needs review, though
    // today's version lists no cut
    git(d, &["checkout", "-q", "-b", "u", "main"]);
    write(
        d,
        "specs/a/SPEC.md",
        &a(
            "The directory SHALL be deleted once nothing else is left in it. It SHALL sit in place.",
        ),
    );
    commit(d, "trade a stating sentence for two holding a phrase");
    let v3 = evaluate_by(d, "main", "u", false, 3).unwrap();
    let now = evaluate_by(d, "main", "u", false, RULES).unwrap();
    assert!(now.cuts.is_empty(), "{:?}", now.cuts);
    assert_eq!((v3.review, now.review), (true, false));
}

#[test]
fn scenario_a_placement_phrase_across_a_line_under_each_version() {
    // req:gate/rules-version@373963b2
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    let a = |statement: &str| {
        format!("{HEAD}\n### Requirement: x\n{statement}\n\n#### Scenario: s\n- ok\n")
    };
    let one = "The command SHALL print the list.";
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/a/SPEC.md",
        &a(&format!("{one} It SHALL exit 1 when the list is empty.")),
    );
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "s"]);
    write(
        d,
        "specs/a/SPEC.md",
        &a(&format!(
            "{one} Every test of a's requirements SHALL\nsit in a's own files."
        )),
    );
    commit(d, "trade a sentence of behaviour for a wrapped placing one");
    let v5 = evaluate_by(d, "main", "s", false, 5).unwrap();
    let now = evaluate_by(d, "main", "s", false, RULES).unwrap();
    assert_eq!((v5.kind, v5.review), (Kind::Spec, false));
    assert_eq!((now.kind, now.review), (Kind::Spec, true));
    assert!(rules(&v5).is_empty(), "{:?}", v5.violations);
    assert_eq!(rules(&now), ["gate/approval"]);
    for out in [&v5, &now] {
        let lines: Vec<String> = out.cuts.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            ["specs/a/SPEC.md: gate/cut: a/x states 1 sentences where it stated 2"]
        );
    }
    // the lower versions matched a phrase only where one space separates its words too
    for version in [1, 2, 3, 4] {
        assert!(
            !evaluate_by(d, "main", "s", false, version).unwrap().review,
            "{version}"
        );
    }
}

#[test]
fn scenario_a_subdirectory_root_under_each_version() {
    // req:gate/rules-version@373963b2
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    git(top, &["init", "-q", "-b", "main"]);
    write(top, "tool/specs/SPEC.md", ROOT);
    write(top, "tool/specs/a/SPEC.md", &format!("{HEAD}{X}{Y}"));
    write(top, "tool/ARCHITECTURE.md", "invariants\n");
    commit(top, "base");
    git(top, &["checkout", "-q", "-b", "s"]);
    write(top, "tool/ARCHITECTURE.md", "invariants, more\n");
    commit(top, "architecture");
    let tool = top.join("tool");
    let v4 = evaluate_by(&tool, "main", "s", false, 4).unwrap();
    let now = evaluate_by(&tool, "main", "s", false, RULES).unwrap();
    assert_eq!((v4.kind, v4.review), (Kind::Code, false));
    assert_eq!((now.kind, now.review), (Kind::Spec, true));
    assert!(rules(&v4).is_empty(), "{:?}", v4.violations);
    assert_eq!(rules(&now), ["gate/approval"]);
}

#[test]
fn specs_that_dont_load_need_no_review_under_version_1() {
    // req:gate/rules-version@373963b2
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
    // req:gate/rules-version@373963b2
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
    // req:gate/rules-version@373963b2
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
