//! `clm triage` against a bare `origin` and a clone.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use clm_triage::triage;
use common::*;

/// The repo with nodes `check`, `gate` (requirements `x`, `y`), `model` (`z`) and `view`
/// on main, and `items` as `(slug, frontmatter lines)` in `backlog/`.
fn with_items(items: &[(&str, &str)]) -> Repo {
    let r = repo();
    write(&r.work, "specs/check/SPEC.md", &node("", &[]));
    write(&r.work, "specs/gate/SPEC.md", &node("", &["x", "y"]));
    write(&r.work, "specs/model/SPEC.md", &node("", &["z"]));
    write(&r.work, "specs/view/SPEC.md", &node("", &[]));
    for (slug, fm) in items {
        write(
            &r.work,
            &format!("backlog/{slug}.md"),
            &format!("---\ntitle: T\n{fm}---\nBody of {slug}.\n"),
        );
    }
    push_main(&r.work, "items");
    r
}

fn lines(r: &Repo) -> Vec<String> {
    triage(&r.work, 4).unwrap().lines
}

fn groups(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .filter(|l| l.starts_with("group "))
        .map(String::as_str)
        .collect()
}

#[test]
fn scenario_groups_by_kind_and_components() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("a", "kind: bug\ncomponents: [check]\n"),
        ("b", "kind: bug\ncomponents: [check]\n"),
        ("c", "kind: idea\ncomponents: [check]\n"),
        ("d", "kind: deferred\ncomponents: [check]\n"),
        ("e", "kind: bug\n"),
    ]);
    let lines = lines(&r);
    assert_eq!(
        lines,
        [
            "group bug: [] P5 0 e",
            "group bug:check [check] P5 0 a b",
            "idea c [check] P5 0",
            "deferred d [check] P5 0",
        ]
    );
}

#[test]
fn scenario_budget_cuts_a_group() {
    // req:orchestrator.triage/triage@91b10ab7
    let mut items: Vec<(String, String)> = ["a", "b", "c", "d", "e", "f", "h"]
        .iter()
        .map(|s| {
            (
                s.to_string(),
                "kind: bug\ncomponents: [check]\n".to_string(),
            )
        })
        .collect();
    items.push((
        "g".into(),
        "kind: bug\ncomponents: [check]\npriority: P2\n".into(),
    ));
    let items: Vec<(&str, &str)> = items.iter().map(|(s, f)| (&**s, &**f)).collect();
    let r = with_items(&items);
    assert_eq!(
        groups(&lines(&r)),
        [
            "group bug:check [check] P2 0 g a b c d e",
            "group bug:check [check] P5 0 f h",
        ]
    );
}

#[test]
fn scenario_releases_and_order() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        (
            "q",
            "kind: idea\ncomponents: [gate]\nblocks: [gate/x, gate/y]\n",
        ),
        ("r", "kind: idea\ncomponents: [check]\n"),
        (
            "s",
            "kind: question\ncomponents: [model]\nblocks: [model/z]\n",
        ),
        ("v", "kind: human-task\ncomponents: [model]\n"),
        ("t", "kind: bug\ncomponents: [view]\n"),
    ]);
    assert_eq!(
        lines(&r),
        [
            "group bug:view [view] P5 0 t",
            "human s [model] P5 1",
            "human v [model] P5 0",
            "idea q [gate] P5 2",
            "idea r [check] P5 0",
        ]
    );
}

#[test]
fn ideas_sort_by_releases_before_slug() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("a", "kind: idea\ncomponents: [check]\n"),
        ("b", "kind: idea\ncomponents: [gate]\nblocks: [gate/x]\n"),
    ]);
    assert_eq!(lines(&r), ["idea b [gate] P5 1", "idea a [check] P5 0"]);
}

#[test]
fn open_sorts_before_main() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[("x", "kind: bug\n"), ("y", "kind: bug\n")]);
    write(
        &r.work,
        "changes/20260101-a/change.md",
        &change_md("applied", "resolves: [x]\n"),
    );
    push_main(&r.work, "m");
    let id = "20260101-z";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(
            &format!("changes/{id}/change.md"),
            &change_md("proposed", "resolves: [y]\n"),
        )],
    );
    assert_eq!(
        lines(&r),
        ["open 20260101-z [] P5 0 y", "main 20260101-a [] P5 0 x"]
    );
}

#[test]
fn scenario_open_and_taken_work() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("q", "kind: idea\ncomponents: [model]\nblocks: [model/z]\n"),
        ("t", "kind: bug\n"),
        ("u", "kind: bug\n"),
    ]);
    let id = "20260101-c";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[
            (
                &format!("changes/{id}/change.md"),
                &change_md("proposed", "resolves: [q, gone]\n"),
            ),
            (&format!("changes/{id}/deltas/check.md"), "delta\n"),
        ],
    );
    push_propose(&r.work, "t", 1);
    let lines = lines(&r);
    assert_eq!(lines[0], "group bug: [] P5 0 u");
    assert_eq!(lines[1], "open 20260101-c [check,model] P5 1 q");
    assert!(
        lines.iter().all(|l| !l.split(' ').any(|w| w == "t")),
        "{lines:?}"
    );
}

#[test]
fn held_by_change_and_claim_stays_on_the_open_line() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[("q", "kind: human-task\ncomponents: [model]\npriority: P3\n")]);
    let id = "20260101-c";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(
            &format!("changes/{id}/change.md"),
            &change_md("proposed", "resolves: [q]\n"),
        )],
    );
    push_propose(&r.work, "q", 1);
    assert_eq!(lines(&r), ["open 20260101-c [model] P3 0 q"]);
}

#[test]
fn a_change_naming_no_backlog_item_has_no_line() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[]);
    let id = "20260101-c";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(
            &format!("changes/{id}/change.md"),
            &change_md("proposed", "resolves: [gone]\n"),
        )],
    );
    assert!(lines(&r).is_empty());
}

#[test]
fn a_lapsed_claim_holds_nothing() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[("t", "kind: bug\n")]);
    push_propose(&r.work, "t", 5);
    assert_eq!(lines(&r), ["group bug: [] P5 0 t"]);
}

#[test]
fn several_components_key_one_group_whatever_their_order() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("a", "kind: bug\ncomponents: [model, check]\n"),
        ("b", "kind: bug\ncomponents: [check, model]\n"),
    ]);
    assert_eq!(lines(&r), ["group bug:check+model [check,model] P5 0 a b"]);
}

#[test]
fn a_change_on_main_holds_its_items() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("q", "kind: idea\ncomponents: [model]\n"),
        ("u", "kind: bug\n"),
    ]);
    write(&r.work, "specs/check/SPEC.md", &node("", &["w"]));
    write(
        &r.work,
        "changes/20260102-m/change.md",
        &change_md("applied", "resolves: [q]\n"),
    );
    write(
        &r.work,
        "changes/20260102-m/deltas/check.md",
        &delta("check", &["w"]),
    );
    push_main(&r.work, "m");
    assert_eq!(
        lines(&r),
        ["group bug: [] P5 0 u", "main 20260102-m [check] P5 0 q"]
    );
}

#[test]
fn scenario_designed_and_on_main() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("x", "kind: bug\n"),
        ("y", "kind: bug\n"),
        ("u", "kind: bug\n"),
    ]);
    write(&r.work, "specs/gate/SPEC.md", &node("", &["x", "y", "w"]));
    write(
        &r.work,
        "changes/20260101-m/change.md",
        &change_md("applied", "resolves: [y]\n"),
    );
    write(
        &r.work,
        "changes/20260101-m/deltas/gate.md",
        &delta("gate", &["w"]),
    );
    push_main(&r.work, "m");
    let id = "20260101-d";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[
            (
                &format!("changes/{id}/change.md"),
                &change_md("applied", "resolves: [x]\n"),
            ),
            (&format!("changes/{id}/deltas/check.md"), "delta\n"),
        ],
    );
    assert_eq!(
        lines(&r),
        [
            "group bug: [] P5 0 u",
            "designed 20260101-d [check] P5 0 x",
            "main 20260101-m [gate] P5 0 y",
        ]
    );
}

#[test]
fn a_change_only_on_autosync_is_read_from_there() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[("x", "kind: bug\n"), ("z", "kind: bug\n")]);
    for (id, status, item) in [
        ("20260101-a", "applied", "x"),
        ("20260101-b", "approved", "z"),
    ] {
        push_files(
            &r.work,
            &format!("autosync/spec/{id}"),
            &[(
                &format!("changes/{id}/change.md"),
                &change_md(status, &format!("resolves: [{item}]\n")),
            )],
        );
    }
    assert_eq!(
        lines(&r),
        ["designed 20260101-a [] P5 0 x", "open 20260101-b [] P5 0 z"]
    );
}

#[test]
fn the_spec_branch_status_wins_over_autosync() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[("x", "kind: bug\n")]);
    let id = "20260101-a";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(
            &format!("changes/{id}/change.md"),
            "---\nstatus: applied\nresolves: [x]\n---\n",
        )],
    );
    // The same change on `autosync/spec/<id>` with another status: `spec/<id>` wins.
    push_files(
        &r.work,
        &format!("autosync/spec/{id}"),
        &[(
            &format!("changes/{id}/change.md"),
            &change_md("proposed", "resolves: [x]\n"),
        )],
    );
    assert_eq!(lines(&r), ["designed 20260101-a [] P5 0 x"]);
}

#[test]
fn scenario_deferred_and_workaround_come_last() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[
        ("u", "kind: bug\n"),
        ("i", "kind: idea\n"),
        ("k", "kind: deferred\ncomponents: [gate]\n"),
    ]);
    write(&r.work, "specs/check/SPEC.md", &node("", &["w"]));
    let graph = load(&r.work);
    let version = graph.requirement("check/w").unwrap().version();
    write(
        &r.work,
        "backlog/f.md",
        &format!(
            "---\ntitle: T\nkind: workaround\ncomponents: [check]\n\
             requirement: check/w@{version}\n---\nBody of f.\n"
        ),
    );
    push_main(&r.work, "workaround");
    assert_eq!(
        lines(&r),
        [
            "group bug: [] P5 0 u",
            "idea i [] P5 0",
            "deferred k [gate] P5 0",
            "workaround f [check] P5 0",
        ]
    );
}

#[test]
fn scenario_rename_branch_waiting_for_review() {
    // req:orchestrator.triage/designed-archived@e2c91326
    let r = with_items(&[("x", "kind: bug\n")]);
    let id = "20260101-r";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(
            &format!("changes/archive/{id}/change.md"),
            &change_md("done", "renames: []\nresolves: [x]\n"),
        )],
    );
    assert_eq!(lines(&r), ["designed 20260101-r [] P5 0 x"]);
}

#[test]
fn an_archived_change_with_another_status_is_open() {
    // req:orchestrator.triage/designed-archived@e2c91326
    let r = with_items(&[("x", "kind: bug\n"), ("y", "kind: bug\n")]);
    push_files(
        &r.work,
        "spec/20260101-p",
        &[(
            "changes/archive/20260101-p/change.md",
            &change_md("proposed", "resolves: [x]\n"),
        )],
    );
    // A live `change.md` is judged alone, whatever the archive holds.
    push_files(
        &r.work,
        "spec/20260101-s",
        &[
            (
                "changes/20260101-s/change.md",
                &change_md("proposed", "resolves: [y]\n"),
            ),
            (
                "changes/archive/20260101-s/change.md",
                &change_md("done", "resolves: [y]\n"),
            ),
        ],
    );
    assert_eq!(
        lines(&r),
        ["open 20260101-p [] P5 0 x", "open 20260101-s [] P5 0 y"]
    );
}

#[test]
fn an_unparseable_archived_change_is_open() {
    // req:orchestrator.triage/designed-archived@e2c91326
    let r = with_items(&[("x", "kind: bug\n")]);
    push_files(
        &r.work,
        "spec/20260101-u",
        &[(
            "changes/archive/20260101-u/change.md",
            "---\nstatus: done\nresolves: [x]\n  bad: [\n---\n",
        )],
    );
    let got = lines(&r);
    assert!(got.iter().all(|l| !l.starts_with("designed ")), "{got:?}");
}

#[test]
fn an_archived_change_only_on_autosync_is_read_from_there() {
    // req:orchestrator.triage/designed-archived@e2c91326
    let r = with_items(&[("x", "kind: bug\n"), ("y", "kind: bug\n")]);
    for (id, status, item) in [("20260101-a", "done", "x"), ("20260101-b", "applied", "y")] {
        push_files(
            &r.work,
            &format!("autosync/spec/{id}"),
            &[(
                &format!("changes/archive/{id}/change.md"),
                &change_md(status, &format!("resolves: [{item}]\n")),
            )],
        );
    }
    assert_eq!(
        lines(&r),
        ["designed 20260101-a [] P5 0 x", "open 20260101-b [] P5 0 y"]
    );
}

#[test]
fn a_failing_fetch_is_a_git_failure() {
    // req:orchestrator.triage/triage@91b10ab7
    let r = with_items(&[("u", "kind: bug\n")]);
    std::fs::remove_dir_all(&r.origin).unwrap();
    match triage(&r.work, 4) {
        Err(clm_triage::Error::Git(_)) => {}
        other => panic!("{:?}", other.map(|t| t.lines)),
    }
}
