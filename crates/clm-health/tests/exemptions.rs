//! The `exemption` signal: what each component's exemptions still cover.

mod common;

use common::{git, repo, report, section, write};

/// Component `a`, owning `tests/a/**`, exempt from `undeclared-imports` there until `backlog/r`.
fn component(root: &std::path::Path) {
    write(
        root,
        "specs/a/SPEC.md",
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"tests/a/**\"]\n\
exemptions:\n  - rule: undeclared-imports\n    files: [\"tests/a/**\"]\n    until: backlog/r\n---\n\n## Requirements\n",
    );
}

#[test]
fn scenario_exemption_being_paid_down() {
    // req:check.health/exemptions-signal@f1b32bd2
    let dir = repo();
    let d = dir.path();
    component(d);
    write(d, "tests/a/x.py", "x = 1\n");
    write(d, "tests/a/y.py", "y = 1\n");
    write(
        d,
        "backlog/r.md",
        "---\ntitle: R\nkind: bug\ncomponents: [a]\n---\n\nr\n",
    );
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "exemption"),
        [
            "health/exemption: 1 exemptions",
            "specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 2 files",
        ],
        "{out}"
    );
}

#[test]
fn scenario_work_done() {
    // req:check.health/exemptions-signal@f1b32bd2
    let dir = repo();
    let d = dir.path();
    component(d);
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "exemption"),
        [
            "health/exemption: 1 exemptions",
            "specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 0 files; item gone; matches nothing",
        ],
        "{out}"
    );
}

#[test]
fn each_suffix_follows_its_own_condition() {
    // req:check.health/exemptions-signal@f1b32bd2
    let dir = repo();
    let d = dir.path();
    component(d);
    // The item is gone but a file is named; then the item exists and no file is.
    write(d, "tests/a/x.py", "x\n");
    git(d, &["add", "-A"]);
    assert_eq!(
        section(&report(d), "exemption")[1],
        "specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 1 files; item gone"
    );
    git(d, &["rm", "-q", "-f", "tests/a/x.py"]);
    write(
        d,
        "backlog/r.md",
        "---\ntitle: R\nkind: bug\ncomponents: [a]\n---\n\nr\n",
    );
    git(d, &["add", "-A"]);
    assert_eq!(
        section(&report(d), "exemption")[1],
        "specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 0 files; matches nothing"
    );
}

#[test]
fn components_in_id_order_with_each_rule_word_and_only_tracked_files() {
    // req:check.health/exemptions-signal@f1b32bd2
    let dir = repo();
    let d = dir.path();
    let spec = |dir: &str, rules: &[&str]| {
        let mut s = format!(
            "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"{dir}/**\"]\nexemptions:\n"
        );
        for r in rules {
            s.push_str(&format!(
                "  - rule: {r}\n    files: [\"t/**\"]\n    until: backlog/r\n"
            ));
        }
        s + "---\n\n## Requirements\n"
    };
    write(
        d,
        "specs/b/SPEC.md",
        &spec("u", &["tests-cite-their-owner"]),
    );
    write(
        d,
        "specs/a/SPEC.md",
        &spec("t", &["skipped-tests", "undeclared-imports"]),
    );
    write(
        d,
        "backlog/r.md",
        "---\ntitle: R\nkind: bug\ncomponents: [a]\n---\n\nr\n",
    );
    write(d, "t/x.py", "x\n");
    git(d, &["add", "-A"]);
    write(d, "t/untracked.py", "x\n");
    assert_eq!(
        section(&report(d), "exemption"),
        [
            "health/exemption: 3 exemptions",
            "specs/a/SPEC.md: health/exemption: a: skipped-tests until backlog/r: 1 files",
            "specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 1 files",
            "specs/b/SPEC.md: health/exemption: b: tests-cite-their-owner until backlog/r: 0 files; matches nothing",
        ]
    );
}

#[test]
fn only_files_the_component_owns_count_and_none_is_a_zero_summary() {
    // req:check.health/exemptions-signal@f1b32bd2
    let dir = repo();
    let d = dir.path();
    // `b` owns `tests/b/**`; a's exemption glob is wider but a owns only `tests/a/**`.
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"tests/a/**\"]\n\
exemptions:\n  - rule: skipped-tests\n    files: [\"tests/**/*.py\"]\n    until: backlog/r\n---\n\n## Requirements\n",
    );
    write(
        d,
        "specs/b/SPEC.md",
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"tests/b/**\"]\n---\n\n## Requirements\n",
    );
    write(d, "tests/a/x.py", "x\n");
    write(d, "tests/b/y.py", "y\n");
    write(d, "tests/a/n.txt", "n\n");
    write(
        d,
        "backlog/r.md",
        "---\ntitle: R\nkind: bug\ncomponents: [a]\n---\n\nr\n",
    );
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "exemption")[1],
        "specs/a/SPEC.md: health/exemption: a: skipped-tests until backlog/r: 1 files",
        "{out}"
    );
    let none = repo();
    assert_eq!(
        section(&report(none.path()), "exemption"),
        ["health/exemption: 0 exemptions"]
    );
}
