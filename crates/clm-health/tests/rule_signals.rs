//! The signals of the rule for a requirement: `outside-contract`, `test-placement`,
//! `design-exit` and `uncited-test`.

mod common;

use std::path::Path;

use common::{git, repo, report, section, settings, write};

/// Component `a`'s spec with `paths`, `contract`, `design` and one requirement per
/// `(slug, statement)`.
fn component(paths: &str, contract: &str, design: &str, requirements: &[(&str, &str)]) -> String {
    let mut s = format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n---\n\n## Contract\n{contract}\n\n\
## Design\n{design}\n\n## Requirements\n"
    );
    for (slug, statement) in requirements {
        s.push_str(&format!(
            "\n### Requirement: {slug}\n{statement}\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// What `clm health --root <root>` prints, and the status it returns, as `clm-health`'s call.
fn call(root: &Path) -> (u8, String) {
    let args = vec!["--root".into(), root.as_os_str().to_owned()];
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_health::health_command(
        args,
        Path::new("/nonexistent"),
        Ok(settings(12000, 40000)),
        &|_| None,
        &mut stdout,
        &mut stderr,
    );
    (status, String::from_utf8(stdout).unwrap())
}

#[test]
fn scenario_a_name_the_contract_doesnt_hold() {
    // req:check.health/rule-signals@f7800f03
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &component(
            "[]",
            "The command prints what `select(graph)` returns.",
            "Plain.",
            &[(
                "x",
                "Its command SHALL print what `select(graph)` returns, as `walk_up(path)` finds \
it, in the file `clm.toml` names, with `walk_up(path)` again.\n\n```\n`fenced_only(x)`\n```",
            )],
        ),
    );
    let out = report(d);
    assert_eq!(
        section(&out, "outside-contract"),
        [
            "health/outside-contract: 1 names",
            "specs/a/SPEC.md: health/outside-contract: a/x names walk_up",
        ],
        "{out}"
    );
}

#[test]
fn code_names_are_paths_lists_or_camel_case() {
    // req:check.health/rule-signals@f7800f03
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &component(
            "[]",
            "Nothing named.",
            "Plain.",
            &[(
                "x",
                "It SHALL call `clm_gate::fingerprint_of` and return `RequirementSignals`, \
reading `human_burdens`, `origin/main`, `Settings` and `a.b(c)`.",
            )],
        ),
    );
    let out = report(d);
    assert_eq!(
        section(&out, "outside-contract"),
        [
            "health/outside-contract: 2 names",
            "specs/a/SPEC.md: health/outside-contract: a/x names clm_gate::fingerprint_of",
            "specs/a/SPEC.md: health/outside-contract: a/x names RequirementSignals",
        ],
        "{out}"
    );
}

#[test]
fn scenario_a_requirement_that_places_a_test() {
    // req:check.health/rule-signals@f7800f03
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &component(
            "[]",
            "Plain.",
            "Plain.",
            &[
                ("x", "It SHALL print."),
                (
                    "y",
                    "It SHALL print. Every test of a's requirements SHALL sit in a's own files.",
                ),
            ],
        ),
    );
    let out = report(d);
    assert_eq!(
        section(&out, "test-placement"),
        [
            "health/test-placement: 1 requirements",
            "specs/a/SPEC.md: health/test-placement: a/y SHALL sit",
        ],
        "{out}"
    );
}

#[test]
fn scenario_an_exit_in_a_design() {
    // req:check.health/rule-signals@f7800f03
    let dir = repo();
    let d = dir.path();
    let spec = component(
        "[]",
        "It exits 2 when git fails.",
        "A missing file exits 1 with the path.\nIt EXITED WITH STATUS 3 once, and exit code 4 too.\n\
Nothing exits here, nor at exit1.\n```\nexits 5\n```",
        &[("x", "It SHALL exit 6.")],
    );
    assert_eq!(
        spec.lines().nth(11),
        Some("A missing file exits 1 with the path.")
    );
    write(d, "specs/a/SPEC.md", &spec);
    let out = report(d);
    assert_eq!(
        section(&out, "design-exit"),
        [
            "health/design-exit: 2 lines",
            "specs/a/SPEC.md:12: health/design-exit: exits 1",
            "specs/a/SPEC.md:13: health/design-exit: EXITED WITH STATUS 3",
        ],
        "{out}"
    );
}

#[test]
fn scenario_a_test_that_cites_nothing() {
    // req:check.health/rule-signals@f7800f03
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &component(
            "[\"crates/a/**\"]",
            "Plain.",
            "Plain.",
            &[("x", "It SHALL print.")],
        ),
    );
    let cites = concat!("req", ":a/x");
    let t = format!(
        "#[test]\nfn proves_x() {{\n    // {cites}\n    assert!(true);\n}}\n\n\
fn helper() {{}}\n\n#[test]\nfn helper_rounds_up() {{\n    assert!(true);\n}}\n"
    );
    assert_eq!(t.lines().nth(8), Some("#[test]"));
    write(d, "crates/a/tests/t.rs", &t);
    write(
        d,
        "vendor/v.rs",
        "#[test]\nfn vendored() {\n    assert!(true);\n}\n",
    );
    git(d, &["add", "-A"]);
    let out = report(d);
    assert_eq!(
        section(&out, "uncited-test"),
        [
            "health/uncited-test: 1 tests",
            "crates/a/tests/t.rs:9: health/uncited-test: helper_rounds_up",
        ],
        "{out}"
    );
    assert!(!out.contains("vendor/v.rs"), "{out}");
}

#[test]
fn scenario_nothing_found() {
    // req:check.health/rule-signals@f7800f03
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        &component("[]", "Plain.", "Plain.", &[("x", "It SHALL print.")]),
    );
    let (status, out) = call(d);
    assert_eq!(status, 0, "{out}");
    for summary in [
        "health/outside-contract: 0 names",
        "health/test-placement: 0 requirements",
        "health/design-exit: 0 lines",
        "health/uncited-test: 0 tests",
    ] {
        assert!(out.lines().any(|l| l == summary), "{summary}: {out}");
    }
    let order: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("health/"))
        .map(|l| l.split(':').next().unwrap())
        .collect();
    assert_eq!(
        order[order.len() - 5..],
        [
            "health/reconsider",
            "health/outside-contract",
            "health/test-placement",
            "health/design-exit",
            "health/uncited-test",
        ],
        "{out}"
    );
}
