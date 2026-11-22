use std::fs;
use std::path::Path;
use std::process::Command;

use clm_check::run;
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

/// `s` as a Rust file whose markers count as a test's: those on its third line on are inside
/// a test, the earlier ones directly above it. Every line of `s` keeps its line number.
fn in_test(s: &str) -> String {
    let (head, body) = match s.strip_prefix('\n') {
        Some(rest) => ("", rest),
        None => s.split_once("\n\n").map_or((s, ""), |(a, b)| (a, b)),
    };
    let nl = if body.ends_with('\n') || body.is_empty() {
        ""
    } else {
        "\n"
    };
    if s.starts_with('\n') {
        format!("#[test]\nfn t() {{{body}{nl}}}\n")
    } else if body.is_empty() {
        format!("{s}#[test]\nfn t() {{}}\n")
    } else {
        format!("{head}\n#[test]\nfn t() {{{body}{nl}}}\n")
    }
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

fn rules(root: &Path) -> Vec<(String, &'static str, String)> {
    run(root)
        .into_iter()
        .map(|v| (v.path.display().to_string(), v.rule, v.message))
        .collect()
}

#[test]
fn dangling_marker_reports_file_and_line() {
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    let d = dir.path();
    write(d, "tests/t.rs", &in_test(&format!("\n\n// {GONE}\n")));
    let found: Vec<String> = run(d).iter().map(clm_check::format).collect();
    assert_eq!(
        found,
        ["tests/t.rs:3: check/dangling-markers: marker cites `a/gone`, which is not a requirement"]
    );
}

#[test]
fn dangling_marker_outside_a_test_is_reported_once() {
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    let d = dir.path();
    write(d, "src/lib.rs", &format!("fn f() {{\n    // {GONE}\n}}\n"));
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/dangling-markers");
    assert_eq!(found[0].0, "src/lib.rs:2");
    assert!(found[0].2.contains("a/gone"), "{found:?}");
}

#[test]
fn bare_and_versioned_markers_both_count_for_dangling() {
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    let d = dir.path();
    write(
        d,
        "tests/t.rs",
        &in_test(&format!("// {GONE}@01234567\n// {X}\n")),
    );
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "tests/t.rs:1");
}

#[test]
fn only_files_git_would_track_are_scanned() {
    // req:check/dangling-markers@80e30156
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
    write(
        d,
        "target/debug/build.rs",
        &in_test(&format!("// {GONE}\n")),
    );
    assert_eq!(rules(d), []);
    write(d, "untracked.rs", &in_test(&format!("// {GONE}\n")));
    assert_eq!(rules(d).len(), 1);
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
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    removing_change(dir.path(), false);
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!("\n\n// {GONE}\n")),
    );
    assert_eq!(rules(dir.path()), []);
}

#[test]
fn marker_dangles_again_once_the_removing_change_is_archived() {
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    removing_change(dir.path(), true);
    write(
        dir.path(),
        "tests/t.rs",
        &in_test(&format!("\n\n// {GONE}\n")),
    );
    let found = rules(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/dangling-markers");
    assert_eq!(found[0].0, "tests/t.rs:3");
}

#[test]
fn finished_changes_must_be_archived_and_only_they() {
    // req:check/archive-done@09f5a69f
    let line = |v: &(String, &str, String)| format!("{}: {}: {}", v.0, v.1, v.2);
    for status in ["done", "abandoned"] {
        let dir = repo(status);
        fs::rename(
            dir.path().join("changes/archive/c"),
            dir.path().join("changes/c"),
        )
        .unwrap();
        let found: Vec<String> = rules(dir.path()).iter().map(line).collect();
        assert_eq!(
            found,
            [format!(
                "changes/c/change.md: check/archive-done: change `c` is {status}; run `clm archive c`"
            )]
        );
    }

    let dir = repo("applied");
    fs::create_dir_all(dir.path().join("changes/archive")).unwrap();
    fs::rename(
        dir.path().join("changes/c"),
        dir.path().join("changes/archive/c"),
    )
    .unwrap();
    let found: Vec<String> = rules(dir.path()).iter().map(line).collect();
    assert_eq!(
        found,
        [
            "changes/archive/c/change.md: check/archive-done: archived change `c` is applied; only done or abandoned changes belong in the archive"
        ]
    );
}

#[test]
fn backlog_sorted_by_priority_then_kind_then_slug() {
    // req:check/backlog-report@546756ba
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
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    let d = dir.path();
    write(
        d,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: approve\nOne can approve.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    write(
        d,
        "tests/t.rs",
        &in_test(&format!("// {REQ}capability/approve\n")),
    );
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
    // req:check/dangling-markers@80e30156
    let dir = repo("applied");
    let d = dir.path();
    write(
        d,
        "tests/t.rs",
        &in_test(&format!("\n// {REQ}capability/nope@01234567\n")),
    );
    let found = rules(d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "check/dangling-markers");
    assert_eq!(found[0].0, "tests/t.rs:2");
    assert!(found[0].2.contains("capability/nope"), "{found:?}");
    assert!(
        found[0].2.ends_with(
            "; a change that removes the capability lists it under removes_capabilities"
        ),
        "{found:?}"
    );
}

#[test]
fn marker_on_architecture_or_untested_entry_reported() {
    // req:check/dangling-markers@80e30156
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
        &in_test(&format!(
            "// {REQ}architecture/single-source@01234567\n\n// {REQ}capability/watch\n// {REQ}architecture/both\n"
        )),
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
    // req:check/dangling-markers@80e30156
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
        write(d, "tests/t.rs", &in_test(&format!("\n\n// {gone}\n")));
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
