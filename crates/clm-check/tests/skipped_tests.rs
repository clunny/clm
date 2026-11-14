//! A skipped test in a file that cites requirements (`skipped-tests`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

// Assembled so this file holds no marker and no skipped test itself.
const CITES: &str = concat!("// req", ":a/x");
const IGNORE: &str = concat!("#[ign", "ore]");

const MESSAGE: &str = "a skipped test in a file that cites requirements; run it, or move it to a file that cites none";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A repository (not a git work tree) with node `a` holding requirement `x`, and `files`.
fn repo(files: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    write(
        root,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\n---\n## Contract\n\n## Requirements\n\n### Requirement: x\nIt SHALL x.\n\n#### Scenario: x\n- WHEN x\n- THEN x\n",
    );
    for (rel, text) in files {
        write(root, rel, text);
    }
    dir
}

/// `<path>: <message>` of each `check/skipped-tests` violation.
fn found(root: &Path) -> Vec<String> {
    clm_check::run(root)
        .iter()
        .filter(|v| v.rule == "check/skipped-tests")
        .map(|v| format!("{}: {}", v.path.display(), v.message))
        .collect()
}

#[test]
fn ignored_rust_test() {
    // req:check/skipped-tests@347f43dc
    let text = format!("#[test]\n{CITES}\nfn a() {{}}\n\n    {IGNORE}\n#[test]\nfn b() {{}}\n");
    let dir = repo(&[("tests/t.rs", text)]);
    assert_eq!(found(dir.path()), [format!("tests/t.rs:5: {MESSAGE}")]);
}

#[test]
fn ignored_with_a_reason() {
    // req:check/skipped-tests@347f43dc
    let text = format!("{CITES}\n{}\n", concat!("#[ign", "ore = \"slow\"]"));
    let dir = repo(&[("tests/t.rs", text)]);
    assert_eq!(found(dir.path()), [format!("tests/t.rs:2: {MESSAGE}")]);
}

#[test]
fn skipped_typescript_tests() {
    // req:check/skipped-tests@347f43dc
    let skips = [
        concat!("it.sk", "ip(\"x\", () => {})"),
        concat!("  test.sk", "ipIf(true)(\"x\", () => {})"),
        concat!("describe.to", "do(\"x\")"),
        concat!("x", "it(\"x\", () => {})"),
        concat!("  x", "describe(\"x\", () => {})"),
        concat!("x", "test(\"x\", () => {})"),
    ];
    let text = format!("{CITES}\n{}\n", skips.join("\n"));
    let dir = repo(&[("web/t.test.ts", text)]);
    let lines: Vec<_> = (2..=7)
        .map(|n| format!("web/t.test.ts:{n}: {MESSAGE}"))
        .collect();
    assert_eq!(found(dir.path()), lines);
}

#[test]
fn not_skips_in_typescript() {
    // req:check/skipped-tests@347f43dc
    let text = format!(
        "{CITES}\n{}\n{}\n{}\n{}\n",
        concat!("audit.sk", "ip(1)"),
        concat!("foo.it.sk", "ip(1)"),
        concat!("box", "it(1)"),
        concat!("it.sk", "ipped(1)"),
    );
    let dir = repo(&[("web/t.test.js", text)]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn skipped_python_tests() {
    // req:check/skipped-tests@347f43dc
    let text = format!(
        "{CITES}\n\nimport pytest\n{}\ndef test_a():\n    {}\n  {}\n",
        concat!("@pytest.mark.sk", "ipif(True, reason=\"x\")"),
        concat!("pytest.sk", "ip(\"no\")"),
        concat!("@unittest.sk", "ip(\"x\")"),
    );
    let dir = repo(&[("tests/test_a.py", text)]);
    assert_eq!(
        found(dir.path()),
        [
            format!("tests/test_a.py:4: {MESSAGE}"),
            format!("tests/test_a.py:6: {MESSAGE}"),
            format!("tests/test_a.py:7: {MESSAGE}"),
        ]
    );
}

#[test]
fn no_markers() {
    // req:check/skipped-tests@347f43dc
    let dir = repo(&[
        (
            "tests/slow.rs",
            format!("{IGNORE}\n#[test]\nfn slow() {{}}\n"),
        ),
        ("docs/t.md", format!("{CITES}\n{IGNORE}\n")),
    ]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn exempt_skip() {
    // req:check/skipped-tests@347f43dc
    let text = format!("#[test]\n{CITES}\nfn a() {{}}\n\n    {IGNORE}\n#[test]\nfn b() {{}}\n");
    let dir = repo(&[("tests/t.rs", text.clone()), ("tests/u.rs", text)]);
    let spec = dir.path().join("specs/a/SPEC.md");
    let original = fs::read_to_string(&spec).unwrap();
    // The exemption names `tests/t.rs` alone, so `tests/u.rs` is still reported, and an
    // exemption from another rule excuses nothing.
    let with = |frontmatter: &str| {
        fs::write(
            &spec,
            original.replacen("owners: []\n", &format!("owners: []\n{frontmatter}\n"), 1),
        )
        .unwrap();
        found(dir.path())
    };
    let own = "paths: [\"tests/**\"]\nexemptions: [{rule: skipped-tests, files: [\"tests/t.rs\"], until: backlog/x}]";
    assert_eq!(with(own), [format!("tests/u.rs:5: {MESSAGE}")]);
    let other = "paths: [\"tests/**\"]\nexemptions: [{rule: undeclared-imports, files: [\"tests/**\"], until: backlog/x}]";
    assert_eq!(
        with(other),
        [
            format!("tests/t.rs:5: {MESSAGE}"),
            format!("tests/u.rs:5: {MESSAGE}")
        ]
    );
}
