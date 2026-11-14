//! `markers-in-tests`: a marker counts only where a test can run it.

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

/// `a/x`'s coverage line as `markers_in` over `files` and `report` give it.
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

#[test]
fn scenario_a_marker_in_code_that_is_no_test() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let m = format!("{REQ}:a/x@{}", version(&dir));
    let text = format!("fn helper() {{\n    // {m}\n}}\n");
    assert_eq!(line(&dir, &[("src/lib.rs", text)]), "a/x unbuilt");
}

#[test]
fn scenario_a_marker_in_a_test() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let m = format!("{REQ}:a/x@{}", version(&dir));
    let text = format!("#[test]\nfn proves() {{\n    // {m}\n}}\n");
    assert_eq!(
        line(&dir, &[("tests/x.rs", text)]),
        "a/x current tests/x.rs:3"
    );
}

#[test]
fn scenario_a_file_header_marker() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let m = format!("{REQ}:a/x@{}", version(&dir));
    let text = format!("// {m}\n\nit('works', () => {{\n}});\n");
    assert_eq!(line(&dir, &[("a.test.ts", text)]), "a/x unbuilt");
}

#[test]
fn scenario_a_language_clm_doesnt_read() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let m = format!("{REQ}:a/x@{}", version(&dir));
    let text = format!("package a\n// {m}\n");
    assert_eq!(
        line(&dir, &[("a_test.go", text)]),
        "a/x current a_test.go:2"
    );
}

#[test]
fn markdown_still_holds_nothing() {
    // req:check.coverage/markers-in-tests@c0070f92
    let text = format!("{REQ}:a/x\n");
    assert!(markers_in(Path::new("n.md"), &text).is_empty());
}

#[test]
fn a_group_and_a_python_test_count() {
    // req:check.coverage/markers-in-tests@c0070f92
    let spec_line = format!("// {REQ}:a/g\n\ndescribe('g', () => {{\n  // {REQ}:a/h\n}});\n");
    let found = markers_in(Path::new("a.test.ts"), &spec_line);
    assert_eq!(found.iter().map(|m| m.line).collect::<Vec<_>>(), [4]);
    let py = format!("def test_x():\n    # {REQ}:a/x\nx = 1  # {REQ}:a/y\n");
    let found = markers_in(Path::new("test_a.py"), &py);
    assert_eq!(
        found.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["a/x"]
    );
}

#[cfg(feature = "checkout")]
#[test]
fn scenario_same_as_the_files() {
    // req:check.coverage/markers-in-tests@c0070f92
    let dir = repo();
    let m = format!("{REQ}:a/x@{}", version(&dir));
    let files = [
        (
            "src/lib.rs",
            format!("fn helper() {{\n    // {m}\n}}\n#[test]\nfn t() {{\n    // {m}\n}}\n"),
        ),
        ("a_test.go", format!("// {m}\n")),
        ("n.md", format!("{m}\n")),
    ];
    for (path, text) in &files {
        write(dir.path(), path, text);
    }
    let scanned = clm_coverage::scan(dir.path()).req;
    let by_files: Vec<Marker> = files
        .iter()
        .flat_map(|(path, text)| markers_in(Path::new(path), text))
        .collect();
    let key = |ms: &[Marker]| {
        let mut v: Vec<_> = ms
            .iter()
            .filter(|m| m.path != Path::new("specs/a/SPEC.md"))
            .map(|m| (m.path.clone(), m.line, m.id.clone()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(key(&scanned), key(&by_files));
    assert_eq!(key(&scanned).len(), 2);
    let graph = graph(dir.path());
    let sorted = |ms: &[Marker]| {
        let mut v = ms.to_vec();
        v.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
        v
    };
    let a = report(&graph, &sorted(&scanned));
    let b = report(&graph, &sorted(&by_files));
    let find = |c: &[clm_coverage::Coverage]| c.iter().find(|c| c.id == "a/x").unwrap().to_string();
    assert_eq!(find(&a), find(&b));
}
