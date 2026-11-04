//! `clm guide`: the guide compiled into the binary, readable outside any repository.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Runs `clm guide <args>` in a directory that isn't in a git repository.
fn guide(args: &[&str]) -> Output {
    let tmp = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("guide")
        .args(args)
        .current_dir(tmp.path())
        .env("GIT_CEILING_DIRECTORIES", tmp.path())
        .output()
        .unwrap()
}

#[test]
fn index() {
    // req:init/guide-command@32520cc6
    let out = guide(&[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.lines().next().unwrap().starts_with("index  "),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|l| l == "shaping-the-graph  Shaping the graph"),
        "{text}"
    );
    assert_eq!(
        text.lines().last(),
        Some("spec-format  Spec format (v0, provisional)"),
        "{text}"
    );
}

#[test]
fn index_in_the_guide_order() {
    // req:init/guide-table-from-files@5c6bd405
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let title = |file: &Path| {
        let text = std::fs::read_to_string(file).unwrap();
        text.lines()
            .find_map(|l| l.strip_prefix("# "))
            .unwrap()
            .trim()
            .to_owned()
    };
    // `README.md`'s numbered list: lines `<n>. [Title](<name>.md)...`.
    let readme = std::fs::read_to_string(docs.join("guide/README.md")).unwrap();
    let listed = readme.lines().filter_map(|l| {
        let (n, rest) = l.split_once(". ")?;
        (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then_some(())?;
        rest.split_once("](")?
            .1
            .split_once(".md")
            .map(|(name, _)| name)
    });
    let mut names: Vec<&str> = Vec::new();
    for name in listed {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let mut expected = vec![format!("index  {}", title(&docs.join("guide/README.md")))];
    for name in names {
        let file = docs.join(format!("guide/{name}.md"));
        expected.push(format!("{name}  {}", title(&file)));
    }
    let out = guide(&[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    // Pages the list doesn't name may follow it, before `spec-format`.
    assert_eq!(lines[..expected.len()], expected[..], "{text}");
    let spec_format = format!("spec-format  {}", title(&docs.join("spec-format.md")));
    assert_eq!(lines.last(), Some(&spec_format.as_str()), "{text}");
}

#[test]
fn one_page() {
    // req:init/guide-command@32520cc6
    let out = guide(&["shaping-the-graph"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/shaping-the-graph.md");
    assert_eq!(out.stdout, std::fs::read(page).unwrap());
}

#[test]
fn the_spec_format() {
    // req:init/guide-command@32520cc6
    let out = guide(&["spec-format"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec-format.md");
    assert_eq!(out.stdout, std::fs::read(page).unwrap());
}

#[test]
fn unknown_page() {
    // req:init/guide-command@32520cc6
    let out = guide(&["no-such-page"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("no guide page no-such-page"), "{err}");
    assert!(
        err.lines()
            .any(|l| l == "shaping-the-graph  Shaping the graph"),
        "{err}"
    );
}
