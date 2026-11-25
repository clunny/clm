//! `markers-in-tests`: a marker proves something only where a test can run it.

mod common;

use std::path::{Path, PathBuf};

use clm_coverage::{
    Marker, deleted_tests, markers_in, markers_outside_tests, report, uncited_tests, uncounted,
};
use common::{ROOT, graph, spec, write};
use tempfile::TempDir;

// Fixture markers are built so the literal never appears whole (`dangling-markers`).
const REQ: &str = "req";

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &spec("[\"src/**\"]", &["x"]));
    dir
}

fn version(dir: &TempDir) -> String {
    graph(dir.path())
        .requirement("a/x")
        .unwrap()
        .version()
        .to_string()
}

/// `a/x`'s coverage line over the markers `markers_in` finds in `files`, as `clm coverage`
/// computes it from a checkout of them.
fn line(dir: &TempDir, files: &[(&str, String)]) -> String {
    let markers: Vec<Marker> = files
        .iter()
        .flat_map(|(path, text)| markers_in(Path::new(path), text))
        .collect();
    report(&graph(dir.path()), &markers)
        .into_iter()
        .find(|c| c.id == "a/x")
        .unwrap()
        .to_string()
}

fn cite(dir: &TempDir) -> String {
    format!("{REQ}:a/x@{}", version(dir))
}

#[test]
fn scenario_a_marker_in_code_that_is_no_test() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let text = format!("fn helper() {{\n    // {}\n}}\n", cite(&dir));
    assert_eq!(line(&dir, &[("src/lib.rs", text)]), "a/x unbuilt");
}

#[test]
fn scenario_a_marker_in_a_test() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let text = format!("#[test]\nfn t() {{\n    // {}\n}}\n", cite(&dir));
    assert_eq!(
        line(&dir, &[("tests/x.rs", text)]),
        "a/x current tests/x.rs:3"
    );
}

#[test]
fn scenario_a_file_header_marker() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let text = format!(
        "// {}\n\ndescribe('g', () => {{\n  it('works', () => {{}});\n}});\n",
        cite(&dir)
    );
    assert_eq!(line(&dir, &[("a.test.ts", text)]), "a/x unbuilt");
}

#[test]
fn scenario_a_language_clm_doesnt_read() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let text = format!("// {}\n", cite(&dir));
    assert_eq!(
        line(&dir, &[("a_test.go", text)]),
        "a/x current a_test.go:1"
    );
}

#[test]
fn python_and_group_markers_follow_the_same_rule() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let c = cite(&dir);
    let py = format!("def test_x():\n    # {c}\n\n# {c}\n");
    let found: Vec<usize> = markers_in(Path::new("test_a.py"), &py)
        .iter()
        .map(|m| m.line)
        .collect();
    assert_eq!(found, [2]);
    // A marker on a group's own lines, outside its tests, counts for none of them.
    let ts = format!(
        "describe('g', () => {{\n  // {c}\n  beforeEach(() => {{}});\n  it('t', () => {{\n    // {c}\n  }});\n}});\n// {c}\n"
    );
    let found: Vec<usize> = markers_in(Path::new("a.test.ts"), &ts)
        .iter()
        .map(|m| m.line)
        .collect();
    assert_eq!(found, [5]);
    let py = format!("class TestG:\n    # {c}\n    x = 1\n    def test_x(self):\n        # {c}\n");
    let found: Vec<usize> = markers_in(Path::new("test_a.py"), &py)
        .iter()
        .map(|m| m.line)
        .collect();
    assert_eq!(found, [5]);
}

#[test]
fn a_marker_in_a_test_beside_one_outside_counts_once() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let c = cite(&dir);
    let text = format!("fn h() {{\n    // {c}\n}}\n#[test]\nfn t() {{\n    // {c}\n}}\n");
    assert_eq!(line(&dir, &[("t.rs", text)]), "a/x current t.rs:6");
}

/// What the four readers give for `a.test.ts`: `a/x`'s coverage line, `uncounted`'s IDs
/// and lines over the file's markers outside tests, `uncited_tests`' tests, and
/// `deleted_tests`' tests and IDs with the file as base and `head` as head.
#[allow(clippy::type_complexity)]
fn four(
    dir: &TempDir,
    text: &str,
    head: &str,
) -> (
    String,
    Vec<(String, Vec<usize>)>,
    Vec<String>,
    Vec<(String, Vec<String>)>,
) {
    let path = Path::new("a.test.ts");
    let outside = markers_outside_tests(path, text);
    let uncounted = uncounted(&graph(dir.path()), &outside)
        .into_iter()
        .map(|u| (u.id, u.markers.iter().map(|m| m.line).collect()))
        .collect();
    let base = [(PathBuf::from("a.test.ts"), text.to_string())];
    let head = [(PathBuf::from("a.test.ts"), head.to_string())];
    let uncited = uncited_tests(&base).into_iter().map(|t| t.test).collect();
    let deleted = deleted_tests(&base, &head)
        .into_iter()
        .map(|t| (t.test, t.ids))
        .collect();
    (
        line(dir, &[("a.test.ts", text.to_string())]),
        uncounted,
        uncited,
        deleted,
    )
}

#[test]
fn scenario_a_marker_above_a_group() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let c = cite(&dir);
    let one = "  it('one', () => {\n    expect(1).toBe(1);\n  });\n";
    let two = "  it('two', () => {\n    expect(2).toBe(2);\n  });\n";
    let text = format!("// {c}\ndescribe('g', () => {{\n{one}{two}}});\n");
    let head = format!("// {c}\ndescribe('g', () => {{\n{one}}});\n");
    let (coverage, uncounted, uncited, deleted) = four(&dir, &text, &head);
    assert_eq!(coverage, "a/x unbuilt");
    assert_eq!(uncounted, [("a/x".to_string(), vec![1])]);
    assert_eq!(uncited, ["a.test.ts::one", "a.test.ts::two"]);
    assert!(deleted.is_empty(), "{deleted:?}");
}

#[test]
fn scenario_a_marker_on_each_test_of_a_group() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let c = cite(&dir);
    let one = format!("  // {c}\n  it('one', () => {{\n    expect(1).toBe(1);\n  }});\n");
    let two = format!("  // {c}\n  it('two', () => {{\n    expect(2).toBe(2);\n  }});\n");
    let text = format!("describe('g', () => {{\n{one}{two}}});\n");
    let head = format!("describe('g', () => {{\n{one}}});\n");
    let (coverage, uncounted, uncited, deleted) = four(&dir, &text, &head);
    assert_eq!(coverage, "a/x current a.test.ts:2 a.test.ts:6");
    assert!(uncounted.is_empty(), "{uncounted:?}");
    assert!(uncited.is_empty(), "{uncited:?}");
    assert_eq!(
        deleted,
        [("a.test.ts::two".to_string(), vec!["a/x".to_string()])]
    );
}

#[cfg(feature = "checkout")]
#[test]
fn scenario_same_as_the_files() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let c = cite(&dir);
    let files = [
        (
            "tests/t.rs",
            format!("fn h() {{\n    // {c}\n}}\n#[test]\nfn t() {{\n    // {c}\n}}\n"),
        ),
        ("lib.go", format!("// {c}\n")),
        ("src/lib.rs", format!("fn h() {{\n    // {c}\n}}\n")),
        ("notes.md", format!("{c}\n")),
        ("notes.MD", format!("{c}\n")),
    ];
    for (path, text) in &files {
        write(dir.path(), path, text);
    }
    let scan = clm_coverage::scan(dir.path());
    let from_files: Vec<Marker> = files
        .iter()
        .flat_map(|(path, text)| markers_in(Path::new(path), text))
        .collect();
    let mut a: Vec<_> = scan.req.iter().map(|m| (&m.path, m.line, &m.id)).collect();
    let mut b: Vec<_> = from_files
        .iter()
        .map(|m| (&m.path, m.line, &m.id))
        .collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    assert_eq!(a.len(), 3);
    let g = graph(dir.path());
    let one = |ms: &[Marker]| {
        let mut ms = ms.to_vec();
        ms.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
        report(&g, &ms)
            .into_iter()
            .find(|c| c.id == "a/x")
            .unwrap()
            .to_string()
    };
    assert_eq!(one(&scan.req), one(&from_files));
    assert_eq!(scan.outside.len(), 2);
}

#[test]
fn only_the_extension_md_exactly_holds_no_markers() {
    // req:check.coverage/markers-in-tests@4b64b47b
    let dir = repo();
    let text = format!("{}\n", cite(&dir));
    assert!(markers_in(Path::new("notes.md"), &text).is_empty());
    // A file named `.md` has no extension, so it isn't one of exactly `md`.
    assert_eq!(markers_in(Path::new("docs/.md"), &text).len(), 1);
    assert_eq!(
        line(&dir, &[("notes.MD", text.clone())]),
        "a/x current notes.MD:1"
    );
    assert_eq!(
        line(&dir, &[("notes.markdown", text)]),
        "a/x current notes.markdown:1"
    );
}
