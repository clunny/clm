//! `clm coverage`'s lists through `clm-check`'s call (`commands-in-crate`).

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_check::coverage_command;
use tempfile::TempDir;

// Markers are assembled so this file doesn't itself cite (or dangle) them.
const REQ: &str = concat!("req", ":");

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

/// The exit status a call returned, read as a process's.
#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(self.0.into())
    }

    fn success(&self) -> bool {
        self.0 == 0
    }
}

/// What a call wrote to its output stream, and what it returned.
struct Output {
    status: Status,
    stdout: Vec<u8>,
}

/// `clm coverage <args> --root <root>`.
fn clm(root: &Path, args: &[&str]) -> Output {
    let mut words: Vec<std::ffi::OsString> = args.iter().map(Into::into).collect();
    words.push("--root".into());
    words.push(root.into());
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = coverage_command(&words, Path::new("."), &|_| None, &mut stdout, &mut stderr);
    Output {
        status: Status(code),
        stdout,
    }
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let o = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}");
}

/// A git repo whose component `a` holds the requirements `slugs`.
fn repo(slugs: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    write(d, "specs/SPEC.md", ROOT);
    let mut a = String::from("---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n");
    for s in slugs {
        a.push_str(&format!(
            "\n### Requirement: {s}\n{s}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    write(d, "specs/a/SPEC.md", &a);
    dir
}

/// The marker citing `a/<slug>` at its current version.
fn cite(root: &Path, slug: &str) -> String {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    format!(
        "{REQ}a/{slug}@{}",
        graph.requirement(&format!("a/{slug}")).unwrap().version()
    )
}

/// An `applied` change that removes `a/x`.
fn removing_change(root: &Path) {
    write(root, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        root,
        "changes/r/deltas/a.md",
        "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
}

#[test]
fn an_uncounted_marker_is_listed_after_the_coverage() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    let d = dir.path();
    let marker = cite(d, "x");
    write(
        d,
        "src/lib.rs",
        &format!("fn f() {{\n    // {marker}\n}}\n"),
    );
    let o = clm(d, &[]);
    assert!(o.status.success());
    let lines: Vec<String> = out(&o).lines().map(String::from).collect();
    assert!(lines[0].starts_with("a/x unbuilt"), "{lines:?}");
    assert_eq!(lines.last().unwrap(), "a/x uncounted src/lib.rs:2");
}

#[test]
fn uncounted_markers_of_one_id_share_a_line() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    let d = dir.path();
    let marker = cite(d, "x");
    write(
        d,
        "src/lib.rs",
        &format!("fn f() {{\n    // {marker}\n}}\nfn g() {{\n    // {marker}\n}}\n"),
    );
    write(d, "src/b.rs", &format!("fn h() {{\n    // {marker}\n}}\n"));
    let o = clm(d, &[]);
    assert_eq!(
        out(&o).lines().last().unwrap(),
        "a/x uncounted src/b.rs:2 src/lib.rs:2 src/lib.rs:5"
    );
}

#[test]
fn a_removed_requirements_tests_are_listed_by_what_they_lose() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["y"]);
    let d = dir.path();
    removing_change(d);
    let y = cite(d, "y");
    let x = format!("{REQ}a/x");
    write(
        d,
        "tests/t.rs",
        &format!(
            "#[test]\nfn both() {{\n    // {x}\n    // {y}\n}}\n\n#[test]\nfn only_x() {{\n    // {x}\n}}\n"
        ),
    );
    let o = clm(d, &["--obsolete-tests"]);
    assert!(o.status.success());
    assert_eq!(
        out(&o),
        "# tests to delete: 1; tests that only lose a marker: 1\n\
         UNCITE tests/t.rs::both\tloses: a/x\n\
         DELETE tests/t.rs::only_x\tcites only removed: a/x\n"
    );
}

#[test]
fn obsolete_tests_print_no_coverage() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["y"]);
    let d = dir.path();
    let o = clm(d, &["--obsolete-tests"]);
    assert!(o.status.success());
    assert_eq!(
        out(&o),
        "# tests to delete: 0; tests that only lose a marker: 0\n"
    );
}

#[test]
fn a_deleted_test_is_listed_against_a_base() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    let d = dir.path();
    let x = cite(d, "x");
    write(
        d,
        "tests/t.rs",
        &format!("#[test]\nfn one() {{\n    // {x}\n}}\n\n#[test]\nfn two() {{\n    // {x}\n}}\n"),
    );
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "b"]);
    write(
        d,
        "tests/t.rs",
        &format!("#[test]\nfn one() {{\n    // {x}\n}}\n"),
    );
    let o = clm(d, &["--deleted-tests", "HEAD"]);
    assert!(o.status.success());
    assert_eq!(
        out(&o),
        "# tests deleted since HEAD: 1\nDELETED tests/t.rs::two\ta/x\n"
    );
}

#[test]
fn a_clm_root_in_a_subdirectory_reads_its_base_from_that_root() {
    // req:check/coverage-lists@963db740
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    git(top, &["init", "-q"]);
    let sub = top.join("sub");
    write(&sub, "specs/SPEC.md", ROOT);
    write(
        &sub,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n",
    );
    let x = cite(&sub, "x");
    let test = |name: &str| format!("#[test]\nfn {name}() {{\n    // {x}\n}}\n");
    write(
        &sub,
        "tests/t.rs",
        &format!("{}\n{}", test("one"), test("two")),
    );
    write(top, "other/tests/u.rs", &test("three"));
    git(top, &["add", "-A"]);
    git(top, &["commit", "-q", "-m", "b"]);
    write(
        &sub,
        "tests/t.rs",
        &format!("{}\n{}", test("one"), test("second")),
    );
    let o = clm(&sub, &["--deleted-tests", "HEAD"]);
    assert!(o.status.success(), "{}", out(&o));
    assert_eq!(out(&o), "# tests deleted since HEAD: 0\n");
}

#[test]
fn an_unknown_base_exits_1_naming_it() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    let o = clm(dir.path(), &["--deleted-tests", "nope"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("nope"), "{}", out(&o));
}

#[test]
fn the_two_flags_together_are_a_usage_error() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    let o = clm(dir.path(), &["--obsolete-tests", "--deleted-tests", "HEAD"]);
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn a_graph_that_doesnt_load_fails_each_list() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    let d = dir.path();
    write(d, "specs/a/SPEC.md", "not a spec");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "b"]);
    let plain = clm(d, &[]);
    assert_eq!(plain.status.code(), Some(1));
    assert!(out(&plain).contains("specs/a/SPEC.md"), "{}", out(&plain));
    for args in [&["--obsolete-tests"][..], &["--deleted-tests", "HEAD"]] {
        let o = clm(d, args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert_eq!(out(&o), out(&plain), "{args:?}");
    }
}

#[test]
fn the_count_line_counts_deletes_and_uncites_apart() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["y"]);
    let d = dir.path();
    removing_change(d);
    let x = format!("{REQ}a/x");
    let one =
        format!("#[test]\nfn one() {{\n    // {x}\n}}\n\n#[test]\nfn two() {{\n    // {x}\n}}\n");
    write(d, "tests/t.rs", &one);
    let o = clm(d, &["--obsolete-tests"]);
    assert_eq!(
        out(&o).lines().next().unwrap(),
        "# tests to delete: 2; tests that only lose a marker: 0"
    );
}

#[test]
fn uncounted_lines_follow_the_obsolete_lines() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["y"]);
    let d = dir.path();
    removing_change(d);
    let y = cite(d, "y");
    write(
        d,
        "tests/t.rs",
        &format!("#[test]\nfn t() {{\n    // {REQ}a/x\n}}\n"),
    );
    write(d, "src/lib.rs", &format!("fn f() {{\n    // {y}\n}}\n"));
    let o = clm(d, &[]);
    let lines: Vec<String> = out(&o).lines().map(String::from).collect();
    let obsolete = lines
        .iter()
        .position(|l| l.starts_with("a/x obsolete"))
        .unwrap();
    let uncounted = lines
        .iter()
        .position(|l| l.starts_with("a/y uncounted"))
        .unwrap();
    assert!(obsolete < uncounted, "{lines:?}");
}

#[test]
fn plain_coverage_fails_on_a_graph_that_doesnt_load() {
    // req:check/coverage-lists@963db740
    let dir = repo(&["x"]);
    write(dir.path(), "specs/a/SPEC.md", "not a spec");
    let o = clm(dir.path(), &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("specs/a/SPEC.md"), "{}", out(&o));
}

#[test]
fn coverage_writes_the_lines_clm_coverage_gives_unchanged() {
    // req:check/commands-in-crate@ed866c99
    let dir = repo(&["x", "y"]);
    let d = dir.path();
    let x = cite(d, "x");
    write(
        d,
        "tests/t.rs",
        &format!("#[test]\nfn t() {{\n    // {x}\n}}\n"),
    );
    let o = clm(d, &[]);
    assert_eq!(o.status.code(), Some(0));
    let (graph, _) = clm_model::load(d);
    let expected: String = clm_coverage::report(&graph, &clm_coverage::scan(d).req)
        .iter()
        .map(|c| format!("{c}\n"))
        .collect();
    assert!(
        expected.contains("a/x current tests/t.rs:3\n"),
        "{expected}"
    );
    assert_eq!(out(&o), expected);
}
