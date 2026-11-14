//! `markers-in-tests`: a marker proves something only where a test can run it.

mod common;

use std::path::Path;

use clm_coverage::{Marker, markers_in, report};
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
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let text = format!("fn helper() {{\n    // {}\n}}\n", cite(&dir));
    assert_eq!(line(&dir, &[("src/lib.rs", text)]), "a/x unbuilt");
}

#[test]
fn scenario_a_marker_in_a_test() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let text = format!("#[test]\nfn t() {{\n    // {}\n}}\n", cite(&dir));
    assert_eq!(
        line(&dir, &[("tests/x.rs", text)]),
        "a/x current tests/x.rs:3"
    );
}

#[test]
fn scenario_a_file_header_marker() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let text = format!(
        "// {}\n\ndescribe('g', () => {{\n  it('works', () => {{}});\n}});\n",
        cite(&dir)
    );
    assert_eq!(line(&dir, &[("a.test.ts", text)]), "a/x unbuilt");
}

#[test]
fn scenario_a_language_clm_doesnt_read() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let text = format!("// {}\n", cite(&dir));
    assert_eq!(
        line(&dir, &[("a_test.go", text)]),
        "a/x current a_test.go:1"
    );
}

#[test]
fn python_and_group_markers_follow_the_same_rule() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let c = cite(&dir);
    let py = format!("def test_x():\n    # {c}\n\n# {c}\n");
    let found: Vec<usize> = markers_in(Path::new("test_a.py"), &py)
        .iter()
        .map(|m| m.line)
        .collect();
    assert_eq!(found, [2]);
    let ts = format!("describe('g', () => {{\n  // {c}\n}});\n// {c}\n");
    let found: Vec<usize> = markers_in(Path::new("a.test.ts"), &ts)
        .iter()
        .map(|m| m.line)
        .collect();
    assert_eq!(found, [2]);
}

#[test]
fn a_marker_in_a_test_beside_one_outside_counts_once() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let c = cite(&dir);
    let text = format!("fn h() {{\n    // {c}\n}}\n#[test]\nfn t() {{\n    // {c}\n}}\n");
    assert_eq!(line(&dir, &[("t.rs", text)]), "a/x current t.rs:6");
}

#[cfg(feature = "checkout")]
#[test]
fn scenario_same_as_the_files() {
    // req:check.coverage/markers-in-tests@c0070f92
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
    assert_eq!(a.len(), 2);
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
