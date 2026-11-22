//! The light route (gate/light): scenarios added to one component's requirements with the
//! code that proves them land together.

use std::fs;
use std::path::Path;

use clm_gate::{Kind, Outcome, evaluate_at};
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

const CAPS: &str = "# Capabilities\n\n## Capabilities\n\n### Capability: see\nA person can see.\n\n#### Why\nW.\n\n#### Scenario: s\n- ok\n";

const FIRST: &str = "\n#### Scenario: first\n- GIVEN a thing\n- WHEN it runs\n- THEN it holds\n";
const SECOND: &str =
    "\n#### Scenario: second\n- GIVEN another thing\n- WHEN it runs\n- THEN it holds too\n";

fn root(wiring: Option<&str>) -> String {
    let wiring = wiring.map_or(String::new(), |w| format!("wiring: {w}\n"));
    format!(
        "---\ntitle: Root\nstatus: active\nowners: []\n{wiring}---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n"
    )
}

/// Component `a` with `contract`, whose requirement `x` holds `scenarios` and serves
/// `capability/see`.
fn spec_a(status: &str, contract: &str, scenarios: &str) -> String {
    format!(
        "---\ntitle: A\nstatus: {status}\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n## Contract\n{contract}\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n{scenarios}\n#### Serves\n- capability/see\n"
    )
}

fn spec_other(title: &str, paths: &str, slug: &str, scenarios: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: active\nowners: []\npaths: [\"{paths}\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: {slug}\n{title} SHALL hold.\n{scenarios}\n#### Serves\n- capability/see\n"
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

/// The version `id` has in the specs of the working tree.
fn version(d: &Path, id: &str) -> String {
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    graph.requirement(id).unwrap().version().to_string()
}

/// `main` with the root (setting `wiring`), components `a`, `b` and `seeing` (which owns the
/// capability tests, as `capability` would but for the reserved ID), `a/x`
/// cited at its version in `crates/a/tests/t.rs` (or, with `cite_in_capability_test`, in
/// `tests/capability/see.rs`), and `backlog/q.md`; `HEAD` on a fresh branch `s`.
fn repo(wiring: Option<&str>, cite_in_capability_test: bool) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", &root(wiring));
    write(d, "CAPABILITIES.md", CAPS);
    write(d, "specs/a/SPEC.md", &spec_a("unbuilt", "c", FIRST));
    write(
        d,
        "specs/b/SPEC.md",
        &spec_other("B", "crates/b/**", "y", FIRST),
    );
    write(
        d,
        "specs/seeing/SPEC.md",
        &spec_other("Seeing", "tests/capability/**", "z", FIRST),
    );
    write(d, "crates/a/src/lib.rs", "pub fn a() {}\n");
    write(d, "crates/b/src/lib.rs", "pub fn b() {}\n");
    write(d, "backlog/q.md", "---\ntitle: Q\nkind: bug\n---\nQ.\n");
    let x = version(d, "a/x");
    let see = [("capability/see", "")];
    if cite_in_capability_test {
        write(d, "crates/a/tests/t.rs", "#[test]\nfn t() {}\n");
        write(d, "tests/capability/see.rs", &cites(&[see[0], ("a/x", &x)]));
    } else {
        write(d, "crates/a/tests/t.rs", &cites(&[("a/x", &x)]));
        write(d, "tests/capability/see.rs", &cites(&see));
    }
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "s"]);
    dir
}

const DONE: &str = "---\nstatus: done\n---\n\n## Intent\nAdd a scenario.\n";
const DELTA: &str = "---\ncomponent: a\n---\n\n## MODIFIED Requirements\n";

/// The edits of branch `s` but its code: the change archived `done` and its delta, and a
/// second scenario added to `a/x`.
fn add_scenario(d: &Path) {
    write(d, "changes/archive/c/change.md", DONE);
    write(d, "changes/archive/c/deltas/a.md", DELTA);
    write(
        d,
        "specs/a/SPEC.md",
        &spec_a("unbuilt", "c", &format!("{FIRST}{SECOND}")),
    );
}

/// The code of branch `s`: `crates/a/src/lib.rs` changed and `a/x` cited at its new version.
fn prove(d: &Path) {
    write(d, "crates/a/src/lib.rs", "pub fn a2() {}\n");
    let x = version(d, "a/x");
    write(d, "crates/a/tests/t.rs", &cites(&[("a/x", &x)]));
}

/// `clm gate --base main [extra]` in `d`: its status and what it prints.
fn gate(d: &Path, extra: &[&str]) -> (u8, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let args = [&["--base", "main"][..], extra].concat();
    let status = clm_gate::command(args, d, &|_| None, &mut out, &mut err);
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
    (status, String::from_utf8(out).unwrap())
}

fn outcome(d: &Path) -> Outcome {
    let s = git(d, &["rev-parse", "s"]);
    evaluate_at(d, "main", &s, false).unwrap()
}

// req:gate/light@3d58764d
#[test]
fn scenario_scenarios_added_with_their_code() {
    let dir = repo(Some("[]"), false);
    let d = dir.path();
    add_scenario(d);
    prove(d);
    commit(d, "s");
    assert_eq!(gate(d, &[]), (0, "light\n".to_string()));
    assert_eq!(gate(d, &["--approved"]), (0, "light\n".to_string()));
    let out = outcome(d);
    assert_eq!((out.kind, out.review), (Kind::Light, false));
    assert!(out.cuts.is_empty() && out.violations.is_empty(), "{out:?}");
    assert_eq!(out.component.as_deref(), Some("a"));
}

// req:gate/light@3d58764d
#[test]
fn scenario_what_a_light_branch_carries_beside_it() {
    type Variant = fn(&Path);
    let variants: [(&str, Variant); 4] = [
        ("a status flip", |d| {
            write(
                d,
                "specs/a/SPEC.md",
                &spec_a("active", "c", &format!("{FIRST}{SECOND}")),
            );
        }),
        ("an item its change resolves", |d| {
            write(
                d,
                "changes/archive/c/change.md",
                &DONE.replace("status: done", "status: done\nresolves: [q]"),
            );
            fs::remove_file(d.join("backlog/q.md")).unwrap();
        }),
        ("an item filed", |d| {
            write(d, "backlog/found.md", "---\ntitle: F\nkind: idea\n---\n");
        }),
        ("no wiring", |_| {}),
    ];
    for (name, variant) in variants {
        let wiring = (name != "no wiring").then_some("[]");
        let dir = repo(wiring, false);
        let d = dir.path();
        add_scenario(d);
        prove(d);
        variant(d);
        commit(d, "s");
        let out = outcome(d);
        assert_eq!(out.kind, Kind::Light, "{name}: {:?}", out.violations);
        assert_eq!(out.component.as_deref(), Some("a"), "{name}");
    }
    // the only code file a capability test `a/x` serves, modified
    let dir = repo(Some("[]"), true);
    let d = dir.path();
    add_scenario(d);
    let x = version(d, "a/x");
    write(
        d,
        "tests/capability/see.rs",
        &cites(&[("capability/see", ""), ("a/x", &x)]),
    );
    commit(d, "s");
    let out = outcome(d);
    assert_eq!(out.kind, Kind::Light, "{:?}", out.violations);
    assert_eq!(out.component.as_deref(), Some("a"));
}

// req:gate/light@3d58764d
#[test]
fn scenario_a_branch_light_doesnt_take() {
    type Variant = fn(&Path);
    let variants: [(&[&str], Variant); 8] = [
        // beyond the scenario: an archived change not `done`, and a block other than a
        // scenario added
        (&["`changes/archive/c/change.md`", "done"], |d| {
            write(
                d,
                "changes/archive/c/change.md",
                &DONE.replace("done", "applied"),
            );
        }),
        (&["`a/x`", "more than scenarios"], |d| {
            let needs = format!("{FIRST}{SECOND}\n#### Needs\n- b/y\n");
            write(d, "specs/a/SPEC.md", &spec_a("unbuilt", "c", &needs));
        }),
        (&["component `a`", "Contract"], |d| {
            write(
                d,
                "specs/a/SPEC.md",
                &spec_a("unbuilt", "changed", &format!("{FIRST}{SECOND}")),
            );
        }),
        (&["`changes/c/change.md`"], |d| {
            fs::remove_dir_all(d.join("changes/archive")).unwrap();
            write(d, "changes/c/change.md", &DONE.replace("done", "applied"));
            write(d, "changes/c/deltas/a.md", DELTA);
        }),
        (&["`specs/b/SPEC.md`"], |d| {
            let both = format!("{FIRST}{SECOND}");
            write(
                d,
                "specs/b/SPEC.md",
                &spec_other("B", "crates/b/**", "y", &both),
            );
        }),
        (&["`crates/b/src/lib.rs`", "`b`"], |d| {
            write(d, "crates/b/src/lib.rs", "pub fn b2() {}\n");
        }),
        (&["`a/x`", "`crates/a/tests/u.rs`"], |d| {
            let old = git(d, &["show", "main:crates/a/tests/t.rs"]);
            write(d, "crates/a/tests/u.rs", &format!("{old}\n"));
        }),
        (&["cuts"], |d| {
            let changed = FIRST.replace("THEN it holds", "THEN it fails");
            write(
                d,
                "specs/a/SPEC.md",
                &spec_a("unbuilt", "c", &format!("{changed}{SECOND}")),
            );
        }),
    ];
    for (named, variant) in variants {
        let dir = repo(Some("[]"), false);
        let d = dir.path();
        add_scenario(d);
        prove(d);
        variant(d);
        commit(d, "s");
        let (status, out) = gate(d, &["--approved"]);
        assert_eq!(status, 1, "{out}");
        assert_eq!(out.lines().next(), Some("mixed"), "{out}");
        let line = out
            .lines()
            .find(|l| l.contains("gate/classify"))
            .unwrap_or_default();
        let (_, why) = line
            .split_once("spec and code land together only on a light branch (gate/light): ")
            .unwrap_or_else(|| panic!("{out}"));
        for word in named {
            assert!(why.contains(word), "{named:?}: {why}");
        }
    }
}

// req:gate/evaluate-at@a51bf198
#[test]
fn scenario_what_a_spec_part_would_need() {
    let parts = |out: &Outcome| (out.kind, out.review, out.spec_part_review);
    // the human layer beside code
    let dir = repo(Some("[]"), false);
    let d = dir.path();
    write(d, "ARCHITECTURE.md", "# A\n");
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    commit(d, "s");
    assert_eq!(parts(&outcome(d)), (Kind::Mixed, false, true));
    // a Contract beside code
    let dir = repo(Some("[]"), false);
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &spec_a("unbuilt", "changed", FIRST));
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    commit(d, "s");
    assert_eq!(parts(&outcome(d)), (Kind::Mixed, false, false));
    // a light branch
    let dir = repo(Some("[]"), false);
    let d = dir.path();
    add_scenario(d);
    prove(d);
    commit(d, "s");
    assert_eq!(parts(&outcome(d)), (Kind::Light, false, false));
}

// req:gate/rules-version@ad92df64
#[test]
fn scenario_a_light_branch_under_each_version() {
    let dir = repo(Some("[]"), false);
    let d = dir.path();
    add_scenario(d);
    prove(d);
    commit(d, "s");
    let s = git(d, &["rev-parse", "s"]);
    let old = clm_gate::evaluate_by(d, "main", &s, false, 6).unwrap();
    assert_eq!(old.kind, Kind::Mixed);
    let rules: Vec<&str> = old.violations.iter().map(|v| v.rule).collect();
    assert_eq!(rules, ["gate/classify"]);
    assert!(!old.violations[0].message.contains("light"), "{old:?}");
    let now = clm_gate::evaluate_by(d, "main", &s, false, clm_gate::RULES).unwrap();
    assert_eq!((now.kind, now.review), (Kind::Light, false));
    assert!(now.violations.is_empty(), "{now:?}");
}
