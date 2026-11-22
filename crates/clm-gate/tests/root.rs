//! The gate in a repository whose CLM root is a subdirectory of its git repository: every
//! path read relative to that root (gate/paths-under-the-root).

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use clm_gate::{ForeignFile, Kind, evaluate_at, files_outside_spec_paths, foreign_files};
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) -> String {
    clm_git::git(d, args).unwrap()
}

fn write(d: &Path, rel: &str, content: &str) {
    let path = d.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn remove(d: &Path, rel: &str) {
    fs::remove_file(d.join(rel)).unwrap();
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

fn root_spec(extra: &str) -> String {
    format!(
        "---\ntitle: Root\nstatus: active\nowners: []\n{extra}---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n"
    )
}

const A: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\npaths: [\"crates/a/**\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

/// A git repository whose CLM root is its directory `tool/`, holding component `a`, with a
/// file `b` at the repository's top and `CLAUDE.md` there too; `pr` checked out from `main`.
fn repo(root_extra: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "b", "top\n");
    write(d, "CLAUDE.md", "the repository's own\n");
    write(d, "tool/specs/SPEC.md", &root_spec(root_extra));
    write(d, "tool/specs/a/SPEC.md", A);
    write(d, "tool/ARCHITECTURE.md", "invariants\n");
    write(d, "tool/crates/a/src/lib.rs", "pub fn a() {}\n");
    write(d, "tool/crates/x/src/lib.rs", "pub fn x() {}\n");
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

/// `clm gate` with `args`, run in `root`: its status and standard output.
fn gate(root: &Path, args: &[&str]) -> (u8, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_gate::command(
        args.iter().map(OsStr::new),
        root,
        &|_| None,
        &mut out,
        &mut err,
    );
    assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
    (status, String::from_utf8(out).unwrap())
}

#[test]
fn scenario_spec_branch_in_a_subdirectory_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    let tool = d.join("tool");
    write(d, "tool/ARCHITECTURE.md", "invariants, more\n");
    commit(d, "architecture");
    let (status, out) = gate(&tool, &["--base", "main"]);
    assert_eq!(status, 1, "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "spec review", "{out}");
    assert!(
        out.contains("gate/approval") && out.contains("clm approve"),
        "{out}"
    );
}

#[test]
fn scenario_a_file_outside_the_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    let tool = d.join("tool");
    write(d, "tool/specs/a/SPEC.md", &format!("{A}\nmore\n"));
    write(d, "b", "changed\n");
    commit(d, "spec and a file outside the root");
    let (status, out) = gate(&tool, &["--base", "main", "--approved"]);
    assert_eq!(status, 1, "{out}");
    assert_eq!(out.lines().next(), Some("mixed"), "{out}");
    // named from the root, climbing out
    assert!(out.contains("(../b)"), "{out}");

    // a spec-path name at the repository's top, outside the root, is code too
    git(d, &["checkout", "-q", "main"]);
    git(d, &["checkout", "-q", "-b", "top-claude"]);
    write(d, "CLAUDE.md", "changed\n");
    commit(d, "the repository's CLAUDE.md");
    let out = evaluate_at(&tool, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Code, false));
}

#[test]
fn scenario_a_file_outside_the_root_listed() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    write(d, "tool/specs/a/SPEC.md", &format!("{A}\nmore\n"));
    write(d, "b", "changed\n");
    commit(d, "spec and a file outside the root");
    let files = files_outside_spec_paths(&d.join("tool"), "main", "pr").unwrap();
    assert_eq!(files, ["../b"]);
}

#[test]
fn scenario_a_build_changes_a_file_outside_the_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("wiring: [\"Cargo.lock\"]\n");
    let d = dir.path();
    write(d, "tool/crates/a/src/lib.rs", "pub fn changed() {}\n");
    write(d, "b", "changed\n");
    commit(d, "h");
    let h = git(d, &["rev-parse", "HEAD"]);
    let found = foreign_files(&d.join("tool"), "main", &h, "a").unwrap();
    assert_eq!(
        found,
        [ForeignFile {
            path: "../b".into(),
            owner: String::new(),
        }]
    );
}

#[test]
fn a_file_outside_the_root_is_no_components_whatever_its_paths_match() {
    // req:gate/paths-under-the-root@6ac586a5
    // `a` owns every file, yet a file outside the root is still no component's
    let every = A.replace("paths: [\"crates/a/**\"]", "paths: [\"**\"]");
    let dir = repo("wiring: [\"Cargo.lock\"]\n");
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "tool/specs/a/SPEC.md", &every);
    commit(d, "a owns all");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "tool/crates/a/src/lib.rs", "pub fn changed() {}\n");
    write(d, "b", "changed\n");
    commit(d, "h");
    let found = foreign_files(&d.join("tool"), "main", "HEAD", "a").unwrap();
    assert_eq!(
        found,
        [ForeignFile {
            path: "../b".into(),
            owner: String::new(),
        }]
    );
}

#[test]
fn scenario_status_only_in_a_subdirectory_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    write(d, "tool/crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "tool/specs/a/SPEC.md",
        &A.replace("status: specd", "status: active"),
    );
    commit(d, "code and status");
    let (status, out) = gate(&d.join("tool"), &["--base", "main"]);
    assert_eq!((status, out.as_str()), (0, "code\n"));

    // the root's spec file is a spec path: a line other than `status:` makes it mixed
    write(
        d,
        "tool/specs/a/SPEC.md",
        &A.replace("status: specd", "status: active")
            .replace("## Contract\nc\n", "## Contract\nchanged\n"),
    );
    commit(d, "contract too");
    let (status, out) = gate(&d.join("tool"), &["--base", "main"]);
    assert_eq!((status, out.lines().next()), (1, Some("mixed")), "{out}");
}

#[test]
fn scenario_a_code_fix_in_a_subdirectory_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "tool/backlog/x.md",
        "---\nkind: bug\ncomponents: [a]\n---\nBroken.\n",
    );
    commit(d, "the bug");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "tool/crates/a/src/lib.rs", "pub fn fixed() {}\n");
    remove(d, "tool/backlog/x.md");
    commit(d, "fix it\n\nResolves: backlog/x");
    let (status, out) = gate(&d.join("tool"), &["--base", "main"]);
    assert_eq!((status, out.as_str()), (0, "code\n"));

    // the root's backlog item is a spec path: deleted with no trailer, the branch is mixed
    git(d, &["checkout", "-q", "-B", "untrailed", "main"]);
    write(d, "tool/crates/a/src/lib.rs", "pub fn fixed() {}\n");
    remove(d, "tool/backlog/x.md");
    commit(d, "fix it");
    let (status, out) = gate(&d.join("tool"), &["--base", "main"]);
    assert_eq!((status, out.lines().next()), (1, Some("mixed")), "{out}");

    // a fix to a test outside `a`'s paths that cites `a` belongs to `a` too
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "tool/tests/t.rs",
        concat!("// req", ":a/x@00000000\n#[test]\nfn t() {}\n"),
    );
    commit(d, "a test citing a");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(
        d,
        "tool/tests/t.rs",
        concat!(
            "// req",
            ":a/x@00000000\n#[test]\nfn t() { assert!(true) }\n"
        ),
    );
    remove(d, "tool/backlog/x.md");
    commit(d, "fix the test\n\nResolves: backlog/x");
    let (status, out) = gate(&d.join("tool"), &["--base", "main"]);
    assert_eq!((status, out.as_str()), (0, "code\n"));
}

#[test]
fn reads_at_a_revision_name_the_roots_file() {
    // req:gate/paths-under-the-root@6ac586a5
    // req:gate/approval-setting@fec3d7ef
    // the root's `clm.toml` is the policy, read at each revision as the root's file
    let dir = repo("");
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "tool/clm.toml", "approval = \"other\"\n");
    write(d, "clm.toml", "approval = \"other\"\n");
    commit(d, "settings");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "tool/clm.toml", "approval = \"any\"\n");
    commit(d, "weaken");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    // a `clm.toml` at the repository's top, outside the root, is code like any other file
    git(d, &["checkout", "-q", "-B", "top", "main"]);
    write(d, "clm.toml", "approval = \"any\"\n");
    commit(d, "weaken the top's");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Code, false));

    // a branch that adds an exemption to the root's component needs review, read at the root
    git(d, &["checkout", "-q", "-B", "ex", "main"]);
    write(
        d,
        "tool/specs/a/SPEC.md",
        &A.replace(
            "owners: [\"@a\"]\n",
            "owners: [\"@a\"]\nexemptions:\n  - rule: skipped-tests\n    files: [\"crates/a/**\"]\n    until: backlog/y\n",
        ),
    );
    write(
        d,
        "tool/backlog/y.md",
        "---\nkind: deferred\ncomponents: [a]\n---\nEnds it.\n",
    );
    commit(d, "exempt");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!(
        (out.kind, out.review),
        (Kind::Spec, true),
        "{:?}",
        out.violations
    );
    assert!(
        !out.violations
            .iter()
            .any(|v| v.rule == "gate/exemption-names-its-item"),
        "{:?}",
        out.violations
    );
    // and its specs load at the root on both sides, so nothing says they don't
    assert!(
        out.cuts
            .iter()
            .all(|c| !c.to_string().contains("don't load")),
        "{:?}",
        out.cuts
    );
}

#[test]
fn diff_relative_changes_nothing() {
    // req:gate/paths-under-the-root@6ac586a5
    // git's `diff.relative` would print paths from `tool/` and drop those outside it
    let dir = repo("");
    let d = dir.path();
    git(d, &["config", "diff.relative", "true"]);
    write(d, "tool/ARCHITECTURE.md", "invariants, more\n");
    commit(d, "architecture");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    write(d, "b", "changed\n");
    commit(d, "and a file outside the root");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
}

#[test]
fn foreign_files_are_sorted_from_the_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("wiring: [\"Cargo.lock\"]\n");
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "tool/specs/b/SPEC.md",
        &A.replace("title: A", "title: B")
            .replace("crates/a/**", "crates/b/**"),
    );
    write(d, "tool/crates/b/src/lib.rs", "pub fn b() {}\n");
    commit(d, "b");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "tool/crates/b/src/lib.rs", "pub fn changed() {}\n");
    write(d, "z", "after tool/ at the top\n");
    commit(d, "h");
    let found = foreign_files(&d.join("tool"), "main", "HEAD", "a").unwrap();
    let found: Vec<(&str, &str)> = found
        .iter()
        .map(|f| (f.path.as_str(), f.owner.as_str()))
        .collect();
    assert_eq!(found, [("../z", ""), ("crates/b/src/lib.rs", "b")]);
}

#[test]
fn a_marker_outside_the_root_earns_no_code_fix() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "tool/backlog/x.md",
        "---\nkind: bug\ncomponents: [a]\n---\nBroken.\n",
    );
    write(d, "t.rs", concat!("// req", ":a/x@00000000\nfn t() {}\n"));
    commit(d, "the bug");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "t.rs", concat!("// req", ":a/x@00000000\nfn t() { }\n"));
    remove(d, "tool/backlog/x.md");
    commit(d, "fix it\n\nResolves: backlog/x");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
}

#[test]
fn changes_and_backlog_items_are_the_roots() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    // a human-task item the branch adds beside a change with no burdens
    write(
        d,
        "tool/changes/c/change.md",
        "---\nstatus: proposed\n---\n\n## Intent\nx\n",
    );
    write(
        d,
        "tool/backlog/h.md",
        "---\nkind: human-task\n---\nMake a key.\n",
    );
    commit(d, "a human step");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    let rules: Vec<&str> = out.violations.iter().map(|v| v.rule).collect();
    assert!(rules.contains(&"gate/undeclared-burden"), "{rules:?}");

    // an item a live change on main resolves, deleted by a build
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "tool/changes/c/change.md",
        "---\nstatus: applied\nresolves: [x]\n---\n\n## Intent\nx\n",
    );
    write(d, "tool/backlog/x.md", "---\nkind: idea\n---\nx\n");
    commit(d, "resolves x");
    git(d, &["checkout", "-q", "-B", "build"]);
    write(d, "tool/crates/a/src/lib.rs", "pub fn built() {}\n");
    remove(d, "tool/backlog/x.md");
    commit(d, "build it");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code, "{:?}", out.violations);
}

#[test]
fn a_spec_changed_on_both_sides_is_named_from_the_root() {
    // req:gate/paths-under-the-root@6ac586a5
    let dir = repo("");
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "tool/specs/a/SPEC.md", &format!("{A}\nmain's\n"));
    commit(d, "main moves");
    git(d, &["checkout", "-q", "pr"]);
    write(d, "tool/specs/a/SPEC.md", &format!("the branch's\n{A}"));
    commit(d, "the branch moves");
    let out = evaluate_at(&d.join("tool"), "main", "HEAD", true).unwrap();
    let paths: Vec<String> = out
        .violations
        .iter()
        .filter(|v| v.rule == "gate/up-to-date")
        .map(|v| v.path.display().to_string())
        .collect();
    assert_eq!(paths, ["specs/a/SPEC.md"], "{:?}", out.violations);
}
