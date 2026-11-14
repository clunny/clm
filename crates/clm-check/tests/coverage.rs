use std::fs;
use std::path::Path;
use std::process::Command;

use clm_check::{State, coverage, run};
use clm_model::Graph;
use tempfile::TempDir;

// Markers are assembled so this file doesn't itself cite (or dangle) them.
const REQ: &str = concat!("req", ":");
const X: &str = concat!("req", ":a/x");
const GONE: &str = concat!("req", ":a/gone");

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";
const A: &str = "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nY.\n\n#### Scenario: s\n- ok\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A spec for node `a` with one requirement per slug.
fn spec(slugs: &[&str]) -> String {
    let mut s = String::from("---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n");
    for slug in slugs {
        s.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// Where a change in `status` lives: finished changes are archived.
fn change_dir(status: &str) -> &'static str {
    match status {
        "done" | "abandoned" => "changes/archive/c",
        _ => "changes/c",
    }
}

/// Specs with `a/x` and `a/y`, and change `c` (status `status`) that added `a/x`.
fn repo(status: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    let c = change_dir(status);
    write(
        d,
        &format!("{c}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    write(
        d,
        &format!("{c}/deltas/a.md"),
        "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n",
    );
    dir
}

fn graph(root: &Path) -> Graph {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

/// The marker citing `id` at its current version in `graph`.
fn cite(graph: &Graph, id: &str) -> String {
    format!("{REQ}{id}@{}", graph.requirement(id).unwrap().version())
}

fn rules(root: &Path) -> Vec<(String, &'static str, String)> {
    run(root)
        .into_iter()
        .map(|v| (v.path.display().to_string(), v.rule, v.message))
        .collect()
}

fn lines(root: &Path) -> Vec<String> {
    coverage(root)
        .unwrap()
        .iter()
        .map(|c| c.to_string())
        .collect()
}

#[test]
fn dangling_marker_reports_file_and_line() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    write(d, "tests/t.rs", &format!("\n\n// {GONE}\n"));
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/dangling-markers");
    assert_eq!(found[0].0, "tests/t.rs:3");
    assert!(found[0].2.contains("a/gone"), "{found:?}");
}

#[test]
fn bare_and_versioned_markers_both_count_for_dangling() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    write(d, "tests/t.rs", &format!("// {GONE}@01234567\n// {X}\n"));
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "tests/t.rs:1");
}

#[test]
fn only_files_git_would_track_are_scanned() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(d)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    };
    git(&["init", "-q"]);
    write(d, ".gitignore", "target/\n");
    write(d, "target/debug/build.rs", &format!("// {GONE}\n"));
    assert_eq!(rules(d), []);
    write(d, "untracked.rs", &format!("// {GONE}\n"));
    assert_eq!(rules(d).len(), 1);
}

#[test]
fn scenario_mixed_coverage() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["w", "x", "y", "z"]));
    let g = graph(d);
    let x = cite(&g, "a/x");
    assert_ne!(g.requirement("a/y").unwrap().version(), "bbbbbbbb");
    write(
        d,
        "t.rs",
        &format!("\n// {x}\n\n// {REQ}a/y@bbbbbbbb\n\n// {REQ}a/z\n"),
    );
    assert_eq!(
        lines(d),
        [
            "a/w unbuilt",
            "a/x current t.rs:2",
            "a/y behind t.rs:4",
            "a/z behind t.rs:6"
        ]
    );
}

#[test]
fn scenario_stale_marker_beside_a_current_one() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["x"]));
    let g = graph(d);
    write(d, "crates/a/t.rs", &format!("\n// {}\n", cite(&g, "a/x")));
    write(
        d,
        "crates/b/t.rs",
        &format!("\n\n\n\n\n\n// {REQ}a/x@bbbbbbbb\n"),
    );
    assert_eq!(lines(d), ["a/x behind crates/b/t.rs:7"]);
}

#[test]
fn report_carries_the_state_and_markers() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = repo("applied");
    let d = dir.path();
    let g = graph(d);
    write(
        d,
        "t.rs",
        &format!("// {}\n// {}\n", cite(&g, "a/x"), cite(&g, "a/x")),
    );
    let report = coverage(d).unwrap();
    assert_eq!(report[0].id, "a/x");
    assert_eq!(report[0].state, State::Current);
    assert_eq!(report[0].markers.len(), 2);
    assert_eq!(report[0].markers[1].line, 2);
    assert_eq!(report[1].state, State::Unbuilt);
    assert_eq!(State::Behind.to_string(), "behind");
}

/// A second change, live or archived, whose delta for `a` removes `gone`.
fn removing_change(root: &Path, archived: bool) {
    let dir = if archived {
        "changes/archive/r"
    } else {
        "changes/r"
    };
    let status = if archived { "done" } else { "applied" };
    write(
        root,
        &format!("{dir}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    write(
        root,
        &format!("{dir}/deltas/a.md"),
        "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0123456789abcdef -->\n",
    );
}

#[test]
fn marker_of_requirement_a_live_change_removed_is_obsolete_not_dangling() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    removing_change(dir.path(), false);
    write(dir.path(), "tests/t.rs", &format!("\n\n// {GONE}\n"));
    assert_eq!(rules(dir.path()), []);
}

#[test]
fn marker_dangles_again_once_the_removing_change_is_archived() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    removing_change(dir.path(), true);
    write(dir.path(), "tests/t.rs", &format!("\n\n// {GONE}\n"));
    let found = rules(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/dangling-markers");
    assert_eq!(found[0].0, "tests/t.rs:3");
}

#[test]
fn scenario_obsolete_marker() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = repo("applied");
    let d = dir.path();
    removing_change(d, false);
    let x = cite(&graph(d), "a/x");
    write(d, "t.rs", &format!("\n// {x}\n\n\n// {GONE}\n"));
    assert_eq!(
        lines(d),
        [
            "a/x current t.rs:2",
            "a/y unbuilt",
            "a/gone obsolete t.rs:5"
        ]
    );
}

#[test]
fn removed_requirement_without_markers_is_not_reported() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = repo("applied");
    removing_change(dir.path(), false);
    assert_eq!(lines(dir.path()), ["a/x unbuilt", "a/y unbuilt"]);
}

#[test]
fn scenario_same_as_checkout() {
    // req:check.coverage/markers-from-files@f29b72b7
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["x", "y", "z"]));
    let g = graph(d);
    let x = cite(&g, "a/x");
    write(d, "t.rs", &format!("// {x}\n// {REQ}a/y@00000000\n"));
    write(d, "n.md", &format!("See {REQ}a/z.\n"));
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(d)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        out.stdout
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    let listed = git(&["ls-files", "-z"]);
    let markers: Vec<_> = listed
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .flat_map(|s| {
            let path = Path::new(std::str::from_utf8(s).unwrap());
            clm_check::markers_in(path, &fs::read_to_string(d.join(path)).unwrap())
        })
        .collect();
    let text =
        |c: Vec<clm_check::Coverage>| -> Vec<String> { c.iter().map(|c| c.to_string()).collect() };
    let from_files = text(clm_check::report(&g, &markers));
    assert_eq!(
        from_files,
        ["a/x current t.rs:1", "a/y behind t.rs:2", "a/z unbuilt"]
    );
    assert_eq!(from_files, text(coverage(d).unwrap()));
    assert_eq!(
        clm_check::markers_in(Path::new("n.md"), &format!("{x}\n")),
        []
    );
}

#[test]
fn markers_in_carries_versions() {
    // req:check.coverage/markers-from-files@f29b72b7
    let text =
        format!("{REQ}a/x@0123abcd {REQ}a/y {REQ}a/z@xyz {REQ}a/w@0123abcd9 {REQ}a/v@0123ABCD\n");
    let found = clm_check::markers_in(Path::new("t.rs"), &text);
    let seen: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|m| (m.id.as_str(), m.version.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [
            ("a/x", Some("0123abcd")),
            ("a/y", None),
            ("a/z", None),
            ("a/w", None),
            ("a/v", None)
        ]
    );
}

#[test]
fn finished_changes_must_be_archived_and_only_they() {
    // req:check/archive-done@8c5d478b
    let dir = repo("abandoned");
    fs::rename(
        dir.path().join("changes/archive/c"),
        dir.path().join("changes/c"),
    )
    .unwrap();
    let found = rules(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/archive-done");
    assert!(found[0].2.contains("clm archive c"), "{found:?}");

    let dir = repo("applied");
    fs::create_dir_all(dir.path().join("changes/archive")).unwrap();
    fs::rename(
        dir.path().join("changes/c"),
        dir.path().join("changes/archive/c"),
    )
    .unwrap();
    let found = rules(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/archive-done");
}

#[test]
fn backlog_sorted_by_priority_then_kind_then_slug() {
    // req:check/backlog-report@392ec9fd
    let dir = repo("applied");
    write(dir.path(), "specs/sync/SPEC.md", ROOT);
    write(
        dir.path(),
        "backlog/b.md",
        "---\ntitle: Bee\nkind: idea\ncomponents: [sync]\n---\n",
    );
    write(
        dir.path(),
        "backlog/a.md",
        "---\ntitle: Ay\nkind: deferred\n---\n",
    );
    write(
        dir.path(),
        "backlog/c.md",
        "---\ntitle: Sea\nkind: deferred\n---\n",
    );
    write(
        dir.path(),
        "backlog/z.md",
        "---\ntitle: Zed\nkind: idea\npriority: P2\n---\n",
    );
    let lines: Vec<String> = clm_check::backlog(dir.path())
        .unwrap()
        .iter()
        .map(|i| i.to_string())
        .collect();
    assert_eq!(
        lines,
        [
            "z idea P2 [] Zed",
            "a deferred P5 [] Ay",
            "c deferred P5 [] Sea",
            "b idea P5 [sync] Bee"
        ]
    );
}

#[test]
fn marker_on_entry_not_dangling() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    write(
        d,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: approve\nOne can approve.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(d, "tests/t.rs", &format!("// {REQ}capability/approve\n"));
    // With an entry, `a`'s rules must serve one (`check/untraced`).
    let a = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    write(
        d,
        "specs/a/SPEC.md",
        &a.replacen("---\n", "---\nserves: [capability/approve]\n", 1),
    );
    let found = rules(d);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn marker_on_missing_entry_dangling() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    write(
        d,
        "tests/t.rs",
        &format!("\n// {REQ}capability/nope@01234567\n"),
    );
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/dangling-markers");
    assert_eq!(found[0].0, "tests/t.rs:2");
    assert!(found[0].2.contains("capability/nope"), "{found:?}");
}

#[test]
fn marker_on_architecture_or_untested_entry_reported() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    write(
        d,
        "ARCHITECTURE.md",
        "## Constraints\n\n### Constraint: single-source\nGit SHALL be the truth.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n\n### Constraint: both\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Untested\nA judgement.\n",
    );
    write(
        d,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: watch\nOne can watch.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n\n#### Untested\nNo program sees it.\n",
    );
    let a = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    write(
        d,
        "specs/a/SPEC.md",
        &a.replacen("---\n", "---\nserves: [capability/watch]\n", 1),
    );
    write(
        d,
        "tests/t.rs",
        &format!(
            "// {REQ}architecture/single-source@01234567\n\n// {REQ}capability/watch\n// {REQ}architecture/both\n"
        ),
    );
    let arch =
        "architecture entries are proven by the rules their #### Enforced by names, not by tests";
    let at = |line: &str, message: &str| {
        (
            format!("tests/t.rs:{line}"),
            "check/dangling-markers",
            message.to_string(),
        )
    };
    assert_eq!(
        rules(d),
        [
            at("1", arch),
            at("3", "capability/watch is untested"),
            at("4", arch)
        ]
    );
}

#[test]
fn capability_removed_by_a_live_change_is_not_dangling() {
    // req:check/dangling-markers@2e31c5b8
    let suffix = "; a change that removes the capability lists it under removes_capabilities";
    let gone = format!("{REQ}capability/gone");
    for (dir, status, removes, dangling) in [
        ("changes/x", "proposed", "[gone]", false),
        ("changes/archive/x", "done", "[gone]", true),
        ("changes/x", "proposed", "[]", true),
    ] {
        let tmp = repo("applied");
        let d = tmp.path();
        write(d, "CAPABILITIES.md", "## Capabilities\n");
        write(d, "tests/t.rs", &format!("\n\n// {gone}\n"));
        write(
            d,
            &format!("{dir}/change.md"),
            &format!("---\nstatus: {status}\nremoves_capabilities: {removes}\n---\n"),
        );
        let found: Vec<_> = rules(d)
            .into_iter()
            .filter(|v| v.1 == "check/dangling-markers")
            .collect();
        if dangling {
            assert_eq!(found.len(), 1, "{dir} {removes}: {found:?}");
            assert_eq!(found[0].0, "tests/t.rs:3");
            assert!(found[0].2.contains("capability/gone"), "{found:?}");
            assert!(found[0].2.ends_with(suffix), "{found:?}");
        } else {
            assert!(found.is_empty(), "{found:?}");
        }
    }
}

#[test]
fn only_a_capability_marker_gets_the_removal_hint() {
    // req:check/dangling-markers@2e31c5b8
    let dir = repo("applied");
    let d = dir.path();
    write(d, "tests/t.rs", &format!("// {GONE}\n"));
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(!found[0].2.contains("removes_capabilities"), "{found:?}");
}
