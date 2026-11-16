//! `clm proposals` and `clm claim backlog/<slug>` at the command line, for the root's
//! `component-flags`.

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
fn scenario_proposals_lines_filter_and_warning() {
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
