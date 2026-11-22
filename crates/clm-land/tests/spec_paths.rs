//! A spec branch holds spec paths only (land/spec-branch-holds-spec-paths).

mod common;

use std::path::Path;

use common::{Fixture, refused};

#[test]
fn code_on_a_spec_branch() {
    // req:land/spec-branch-holds-spec-paths@7730aebd
    let f = Fixture::new("");
    // The gate's process says nothing against it: the in-process kind refuses it.
    f.write("gate-out", "spec\n");
    f.branch("spec/x", &[("crates/x/src/lib.rs", "fn x() {}\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("crates/x/src/lib.rs"), "{why}");
    assert!(why.contains("spec paths"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("gate.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
}

#[test]
fn a_suggestion_on_a_spec_branch() {
    // req:land/spec-branch-holds-spec-paths@7730aebd
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    f.branch("spec/x", &[("backlog/x.md", "x\n")]);
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn code_on_an_impl_branch() {
    // req:land/spec-branch-holds-spec-paths@7730aebd
    let f = Fixture::new("");
    f.branch("impl/x", &[("crates/x/src/lib.rs", "fn x() {}\n")]);
    clm_land::land(&f.work(), Some("impl/x"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

const CAPS: &str = "# Capabilities\n\n## Capabilities\n\n### Capability: see\nA person can see.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n";
const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const FIRST: &str = "\n#### Scenario: first\n- GIVEN a thing\n- WHEN it runs\n- THEN it holds\n";
const SECOND: &str =
    "\n#### Scenario: second\n- GIVEN another thing\n- WHEN it runs\n- THEN it holds too\n";

fn spec_a(scenarios: &str) -> String {
    format!(
        "---\ntitle: A\nstatus: unbuilt\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n{scenarios}\n#### Serves\n- capability/see\n"
    )
}

/// A test citing each of `ids` at its version, built so this file holds no marker.
fn cites(ids: &[(&str, &str)]) -> String {
    let mut out = String::from("#[test]\nfn t() {\n");
    for (id, version) in ids {
        out.push_str(&format!("    // {}{id}@{version}\n", concat!("req", ":")));
    }
    out.push_str("}\n");
    out
}

/// The version `id` has in the specs of the working tree `d`.
fn version(d: &Path, id: &str) -> String {
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    graph.requirement(id).unwrap().version().to_string()
}

#[test]
fn a_light_branch_named_spec() {
    // req:land/spec-branch-holds-spec-paths@7730aebd
    let f = Fixture::new("");
    let work = f.work();
    f.commit_files(
        &[
            ("specs/SPEC.md", ROOT),
            ("CAPABILITIES.md", CAPS),
            ("specs/a/SPEC.md", &spec_a(FIRST)),
            ("crates/a/src/lib.rs", "pub fn a() {}\n"),
            ("tests/capability/see.rs", &cites(&[("capability/see", "")])),
        ],
        "specs",
    );
    let x = version(&work, "a/x");
    f.commit_files(&[("crates/a/tests/t.rs", &cites(&[("a/x", &x)]))], "cite");
    f.git(&work, &["push", "-q", "origin", "main"]);
    // `spec/c` adds a scenario to `a/x`, archived `done`, with the code that proves it.
    f.git(&work, &["checkout", "-q", "-b", "spec/c"]);
    f.commit_files(
        &[
            (
                "changes/archive/c/change.md",
                "---\nstatus: done\n---\n\n## Intent\nAdd a scenario.\n",
            ),
            (
                "changes/archive/c/deltas/a.md",
                "---\ncomponent: a\n---\n\n## MODIFIED Requirements\n",
            ),
            ("specs/a/SPEC.md", &spec_a(&format!("{FIRST}{SECOND}"))),
            ("crates/a/src/lib.rs", "pub fn a2() {}\n"),
        ],
        "scenario",
    );
    let x = version(&work, "a/x");
    f.commit_files(&[("crates/a/tests/t.rs", &cites(&[("a/x", &x)]))], "prove");
    f.git(&work, &["push", "-q", "origin", "spec/c"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let outcome = clm_gate::evaluate_at(&work, "origin/main", "spec/c", false).unwrap();
    assert_eq!(
        outcome.kind,
        clm_gate::Kind::Light,
        "{:?}",
        outcome.violations
    );
    f.write("gate-out", "spec\n");
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("spec/c"), &f.env(&[])));
    assert!(why.contains("crates/a/src/lib.rs"), "{why}");
    assert!(why.contains("crates/a/tests/t.rs"), "{why}");
    assert!(why.contains("spec paths"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), old);
}
