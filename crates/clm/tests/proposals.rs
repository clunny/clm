//! `clm claim backlog/<slug>` and `clm proposals` at the command line.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::path::Path;
use std::process::Command;

use common::*;

fn clm(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn scenario_claim_prints_the_branch_then_is_refused() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = repo();
    write(&r.work, "backlog/q.md", &item("c"));
    push_main(&r.work, "q");
    let (code, out, err) = clm(&r.work, &["claim", "backlog/q"]);
    assert_eq!((code, out.as_str()), (Some(0), "propose/q\n"), "{err}");
    let (code, out, err) = clm(&r.work, &["claim", "backlog/q"]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.contains("claimed by propose/q"), "{err}");
    let (code, _, err) = clm(&r.work, &["claim", "backlog/nope"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("no such item"), "{err}");
}

#[test]
fn claim_fix_prints_the_branch_then_is_refused() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    cite(&r.work, "b.rs", &["b/r"]);
    write(
        &r.work,
        "backlog/x.md",
        "---\ntitle: X\nkind: bug\ncomponents: [b]\n---\n",
    );
    push_main(&r.work, "b converged, bug x");
    let (code, _, err) = clm(&r.work, &["claim", "b"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("converged"), "{err}");
    let (code, out, err) = clm(&r.work, &["claim", "b", "--fix", "backlog/x"]);
    assert_eq!((code, out.as_str()), (Some(0), "impl/b\n"), "{err}");
    // The checkout is on the claimed branch by the time it is printed.
    assert_eq!(
        git(&r.work, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "impl/b\n"
    );
    let (code, out, err) = clm(&r.work, &["claim", "b", "--fix", "backlog/x"]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.contains("`b` is claimed"), "{err}");
    // A fix claims a node, never a backlog item.
    let (code, _, err) = clm(&r.work, &["claim", "backlog/x", "--fix", "backlog/x"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("isn't a component"), "{err}");
}

#[test]
fn scenario_proposals_lines_filter_and_warning() {
    // req:orchestrator/proposals@ba5ce6a8
    // req:clm/component-flags@4d6d18e7
    let r = repo();
    write(&r.work, "backlog/q.md", &item("c"));
    write(&r.work, "backlog/s.md", &item("b"));
    push_main(&r.work, "items");
    push_files(
        &r.work,
        "spec/20260101-c",
        &[
            (
                "changes/20260101-c/change.md",
                &change_md("proposed", "resolves: [q]\n"),
            ),
            ("changes/20260101-c/deltas/a.md", &delta("a", &["u"])),
        ],
    );
    push_files(
        &r.work,
        "spec/20260101-e",
        &[("changes/20260101-e/change.md", "# e\n")],
    );
    push_propose(&r.work, "s", 1);
    let (code, out, err) = clm(&r.work, &["proposals"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        "20260101-c proposed [a,c] q\n20260101-e proposed []\nbacklog/s claimed [b]\n"
    );
    assert!(err.contains("origin/spec/20260101-e"), "{err}");
    let (code, out, _) = clm(
        &r.work,
        &["proposals", "--component", "b", "--component", "c"],
    );
    assert_eq!(
        (code, out.as_str()),
        (
            Some(0),
            "20260101-c proposed [a,c] q\nbacklog/s claimed [b]\n"
        )
    );
    let (code, out, _) = clm(&r.work, &["proposals", "--component", "x"]);
    assert_eq!((code, out.as_str()), (Some(0), ""));
}

/// `clm next` in `dir` with `max_builds` from `CLM_MAX_BUILDS`, away from the user's
/// settings.
fn next_with(dir: &Path, max_builds: &str) -> (Option<i32>, String, String) {
    let home = tempfile::TempDir::new().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("next")
        .current_dir(dir)
        .env("XDG_CONFIG_HOME", home.path())
        .env("CLM_MAX_BUILDS", max_builds)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn next_is_capped_by_the_max_builds_setting() {
    // req:orchestrator/next@fd7e1025
    let r = repo();
    let (code, out, err) = next_with(&r.work, "3");
    assert_eq!((code, out.as_str()), (Some(0), "a\nb\nc\n"), "{err}");
    let placed = clm_orchestrator::take_place(
        &r.work,
        "z",
        clm_orchestrator::PlaceKind::Build,
        &["impl/z".to_string()],
        3,
        4,
    );
    assert_eq!(placed.unwrap(), clm_orchestrator::Placed::New);
    let (code, out, err) = next_with(&r.work, "2");
    assert_eq!((code, out.as_str()), (Some(0), "a\n"), "{err}");
    assert_eq!(err, "", "a list that isn't empty says nothing");
    let (code, out, err) = next_with(&r.work, "1");
    assert_eq!((code, out.as_str()), (Some(0), ""));
    assert_eq!(
        err,
        "no free build place: 1 of max_builds = 1 in use, 3 ready\n"
    );
    let (code, _, err) = next_with(&r.work, "none");
    assert_eq!(code, Some(1), "settings that don't load: {err}");
    assert!(err.contains("max_builds"), "{err}");
}

#[test]
fn component_flag_names_the_component() {
    // req:clm/component-flags@4d6d18e7
    let r = repo();
    write(&r.work, "backlog/q.md", &item("c"));
    push_main(&r.work, "q");
    assert_eq!(clm(&r.work, &["claim", "backlog/q"]).0, Some(0));
    let new = clm(&r.work, &["proposals", "--component", "c"]);
    assert_eq!(new.0, Some(0), "{}", new.2);
    assert!(new.1.contains("backlog/q"), "{}", new.1);
    let help = clm(&r.work, &["proposals", "--help"]).1;
    assert!(help.contains("--component"), "{help}");
}
