//! A build holds its own files (land/build-holds-own-files).

mod common;

use common::{Fixture, refused};

const REQ: &str = "\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

fn root() -> String {
    "---\ntitle: Root\nstatus: active\nowners: []\npaths: [\"Cargo.lock\"]\nwiring: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n".into()
}

fn component(title: &str, paths: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: [\"@a\"]\npaths: [\"{paths}\"]\n---\n\n## Contract\nc\n\n## Design\nthe spine\n\n## Requirements\n{REQ}"
    )
}

/// A root with `wiring: []`, components `a` and `b`, pushed on `main`.
fn with_specs() -> Fixture {
    let f = Fixture::new("");
    f.commit_files(
        &[
            ("specs/SPEC.md", &root()),
            ("specs/a/SPEC.md", &component("A", "crates/a/**")),
            ("specs/b/SPEC.md", &component("B", "crates/b/**")),
            ("crates/a/src/lib.rs", "a\n"),
            ("crates/b/src/lib.rs", "b\n"),
        ],
        "specs",
    );
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f
}

const BOTH: [(&str, &str); 2] = [
    ("crates/a/src/lib.rs", "a2\n"),
    ("crates/b/src/lib.rs", "b2\n"),
];

#[test]
fn another_components_source_refuses_the_land() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(
        why.starts_with("crates/b/src/lib.rs: land/build-holds-own-files: owned by `b`;"),
        "{why}"
    );
    assert!(!why.contains("crates/a/src/lib.rs"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
}

#[test]
fn the_builds_pre_check_prints_the_same_line() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(
        why.contains("crates/b/src/lib.rs: land/build-holds-own-files: owned by `b`;"),
        "{why}"
    );
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
}

#[test]
fn its_own_files_land() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("impl/a", &BOTH[..1]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_branch_of_another_name_is_the_gates_to_block() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("tidy-b", &BOTH);
    // A gate that blocks it with a `gate/own-files` line for each file, as `gate/own-files`
    // blocks such a branch.
    let gate_lines = "crates/a/src/lib.rs: gate/own-files: owned by a\n\
                      crates/b/src/lib.rs: gate/own-files: owned by b\n";
    f.write("blocking-gate-out", &format!("code\n{gate_lines}"));
    let gate = f.path("bin/blocking-gate");
    let script = format!(
        "#!/usr/bin/env bash\ncat '{}'\nexit 1\n",
        f.path("blocking-gate-out").display()
    );
    common::write_exec(&gate, &script);
    let old = f.remote("main");
    let why = refused(clm_land::land(
        &f.work(),
        Some("tidy-b"),
        &f.env(&[("CLM_GATE", gate.to_str().unwrap())]),
    ));
    assert!(!why.contains("land/build-holds-own-files"), "{why}");
    assert!(why.contains(gate_lines.trim_end()), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), old);
}

#[test]
fn no_origin_main_to_judge_against() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    f.git(&f.work(), &["update-ref", "-d", "refs/remotes/origin/main"]);
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
    assert!(
        out.warnings
            .iter()
            .all(|w| !w.contains("land/build-holds-own-files"))
    );
    assert_eq!(f.lines("check.log").len(), 1);
}

fn push_specs(f: &Fixture, files: &[(&str, &str)]) {
    f.commit_files(files, "specs");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
}

#[test]
fn a_root_with_no_wiring_refuses_nothing() {
    // req:land/build-holds-own-files@c8a9b151
    let f = Fixture::new("");
    push_specs(
        &f,
        &[
            ("specs/SPEC.md", &root().replace("wiring: []\n", "")),
            ("specs/a/SPEC.md", &component("A", "crates/a/**")),
            ("specs/b/SPEC.md", &component("B", "crates/b/**")),
            ("crates/a/src/lib.rs", "a\n"),
            ("crates/b/src/lib.rs", "b\n"),
        ],
    );
    f.branch("impl/a", &BOTH);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn an_archive_branch_is_not_held_even_when_a_component_is_named_archive() {
    // req:land/build-holds-own-files@c8a9b151
    let f = Fixture::new("");
    push_specs(
        &f,
        &[
            ("specs/SPEC.md", &root()),
            (
                "specs/archive/SPEC.md",
                &component("Archive", "crates/archive/**"),
            ),
            ("specs/b/SPEC.md", &component("B", "crates/b/**")),
            ("crates/archive/src/lib.rs", "a\n"),
            ("crates/b/src/lib.rs", "b\n"),
        ],
    );
    f.branch("impl/archive/x", &[("crates/b/src/lib.rs", "b2\n")]);
    clm_land::land(&f.work(), Some("impl/archive/x"), &f.env(&[])).unwrap();
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn a_detached_head_is_checked_as_before() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    f.git(&f.work(), &["checkout", "-q", "--detach", "impl/a"]);
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
    assert_eq!(f.lines("check.log").len(), 1);
}

#[test]
fn the_refusal_names_the_owner_and_what_to_do() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch("impl/a", &BOTH);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(
        why.ends_with(
            "Build this in `b`, or leave the requirement unbuilt and add a question item that blocks it"
        ),
        "{why}"
    );
    assert!(
        why.contains("impl/a changes its own files, the wiring and a capability test it serves."),
        "{why}"
    );
}

#[test]
fn each_foreign_file_gets_its_own_line() {
    // req:land/build-holds-own-files@c8a9b151
    let f = with_specs();
    f.branch(
        "impl/a",
        &[
            ("crates/b/src/lib.rs", "b2\n"),
            ("crates/b/src/more.rs", "m\n"),
        ],
    );
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    let lines: Vec<&str> = why.lines().collect();
    assert_eq!(lines.len(), 2, "{why}");
    for (line, file) in lines
        .iter()
        .zip(["crates/b/src/lib.rs", "crates/b/src/more.rs"])
    {
        assert_eq!(
            *line,
            format!(
                "{file}: land/build-holds-own-files: owned by `b`; impl/a changes its own files, \
                 the wiring and a capability test it serves. Build this in `b`, or leave the \
                 requirement unbuilt and add a question item that blocks it"
            )
        );
    }
}

#[test]
fn a_component_named_archive_is_held_on_its_own_branch() {
    // req:land/build-holds-own-files@c8a9b151
    let f = Fixture::new("");
    push_specs(
        &f,
        &[
            ("specs/SPEC.md", &root()),
            (
                "specs/archive/SPEC.md",
                &component("Archive", "crates/archive/**"),
            ),
            ("specs/b/SPEC.md", &component("B", "crates/b/**")),
            ("crates/archive/src/lib.rs", "a\n"),
            ("crates/b/src/lib.rs", "b\n"),
        ],
    );
    f.branch("impl/archive", &[("crates/b/src/lib.rs", "b2\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/archive"), &f.env(&[])));
    assert!(
        why.starts_with("crates/b/src/lib.rs: land/build-holds-own-files: owned by `b`;"),
        "{why}"
    );
}

#[test]
fn a_file_outside_the_root_names_no_owner() {
    // req:land/build-holds-own-files@c8a9b151
    let f = Fixture::nested();
    push_specs(
        &f,
        &[
            ("specs/SPEC.md", &root()),
            ("specs/a/SPEC.md", &component("A", "crates/a/**")),
            ("crates/a/src/lib.rs", "a\n"),
        ],
    );
    f.branch(
        "impl/a",
        &[("crates/a/src/lib.rs", "a2\n"), ("../b", "b\n")],
    );
    let line = "../b: land/build-holds-own-files: outside the CLM root, which no build of the \
                root's components may change. Leave the requirement unbuilt and add a question \
                item that blocks it";
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert_eq!(why, line);
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
    // The build's pre-check prints the same line.
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert_eq!(why, line);
    assert!(f.lines("check.log").is_empty());
    assert!(f.lines("clm-check.log").is_empty());
}
