use std::fs;
use std::path::Path;
use std::process::Command;

use clm_coverage::State;
use clm_model::Graph;
use tempfile::TempDir;

// Markers are assembled so this file doesn't itself cite (or dangle) them.
const REQ: &str = concat!("req", ":");
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

fn lines(root: &Path) -> Vec<String> {
    coverage(root)
        .unwrap()
        .iter()
        .map(|c| c.to_string())
        .collect()
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

/// Every requirement's coverage for the checkout at `root`: its graph loaded and its markers
/// scanned, as `clm-check` does for the command. `Err` carries spec violations.
fn coverage(root: &Path) -> Result<Vec<clm_coverage::Coverage>, Vec<clm_model::Violation>> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        return Err(violations);
    }
    Ok(clm_coverage::report(&graph, &clm_coverage::scan(root).req))
}

#[test]
fn scenario_mixed_coverage() {
    // req:check.coverage/coverage-report@72845fef
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
        &in_test(&format!(
            "\n// {x}\n\n// {REQ}a/y@bbbbbbbb\n\n// {REQ}a/z\n"
        )),
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
    // req:check.coverage/coverage-report@72845fef
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["x"]));
    let g = graph(d);
    write(d, "crates/a/t.rs", &format!("\n// {}\n", cite(&g, "a/x")));
    write(
        d,
        "crates/b/t.rs",
        &in_test(&format!("\n\n\n\n\n\n// {REQ}a/x@bbbbbbbb\n")),
    );
    assert_eq!(lines(d), ["a/x behind crates/b/t.rs:7"]);
}

#[test]
fn report_carries_the_state_and_markers() {
    // req:check.coverage/coverage-report@72845fef
    let dir = repo("applied");
    let d = dir.path();
    let g = graph(d);
    write(
        d,
        "t.rs",
        &in_test(&format!("// {}\n// {}\n", cite(&g, "a/x"), cite(&g, "a/x"))),
    );
    let report = coverage(d).unwrap();
    assert_eq!(report[0].id, "a/x");
    assert_eq!(report[0].state, State::Current);
    assert_eq!(report[0].markers.len(), 2);
    assert_eq!(report[0].markers[1].line, 2);
    assert_eq!(report[1].state, State::Unbuilt);
    assert_eq!(State::Behind.to_string(), "behind");
}

#[test]
fn scenario_obsolete_marker() {
    // req:check.coverage/coverage-report@72845fef
    let dir = repo("applied");
    let d = dir.path();
    removing_change(d, false);
    let x = cite(&graph(d), "a/x");
    write(d, "t.rs", &in_test(&format!("\n// {x}\n\n\n// {GONE}\n")));
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
    // req:check.coverage/coverage-report@72845fef
    let dir = repo("applied");
    removing_change(dir.path(), false);
    assert_eq!(lines(dir.path()), ["a/x unbuilt", "a/y unbuilt"]);
}

#[test]
fn scenario_same_as_checkout() {
    // req:check.coverage/markers-from-files@50921197
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(&["x", "y", "z"]));
    let g = graph(d);
    let x = cite(&g, "a/x");
    write(
        d,
        "t.rs",
        &in_test(&format!("// {x}\n// {REQ}a/y@00000000\n")),
    );
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
            clm_coverage::markers_in(path, &fs::read_to_string(d.join(path)).unwrap())
        })
        .collect();
    let text = |c: Vec<clm_coverage::Coverage>| -> Vec<String> {
        c.iter().map(|c| c.to_string()).collect()
    };
    let from_files = text(clm_coverage::report(&g, &markers));
    assert_eq!(
        from_files,
        ["a/x current t.rs:1", "a/y behind t.rs:2", "a/z unbuilt"]
    );
    assert_eq!(from_files, text(coverage(d).unwrap()));
    assert_eq!(
        clm_coverage::markers_in(Path::new("n.md"), &format!("{x}\n")),
        []
    );
}

#[test]
fn markers_in_carries_versions() {
    // req:check.coverage/markers-from-files@50921197
    let text = in_test(&format!(
        "// {REQ}a/x@0123abcd {REQ}a/y {REQ}a/z@xyz {REQ}a/w@0123abcd9 {REQ}a/v@0123ABCD {REQ}a/u@0123abcdg\n"
    ));
    let found = clm_coverage::markers_in(Path::new("t.rs"), &text);
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
            ("a/v", None),
            ("a/u", Some("0123abcd"))
        ]
    );
}
