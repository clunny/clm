//! `clm guide`: the guide compiled into the binary, readable outside any repository, through
//! `clm-init`'s call.
// clm:reads ARCHITECTURE.md
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md

use std::path::Path;
use std::process::Output;

use tempfile::TempDir;

mod common;

/// Calls `clm guide <args>` in a directory outside any CLM repository.
fn guide(args: &[&str]) -> Output {
    let tmp = TempDir::new().unwrap();
    assert_eq!(common::found_root(tmp.path()), None);
    let mut all = vec!["guide"];
    all.extend(args);
    common::call(tmp.path(), &all, &common::process_env, &[])
}

#[test]
fn index() {
    // req:init/guide-command@5a6d8da9
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
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[lines.len() - 2],
        "spec-format  Spec format (v0, provisional)",
        "{text}"
    );
    assert_eq!(
        lines[lines.len() - 1],
        "constraints  CLM's constraints",
        "{text}"
    );
}

#[test]
fn index_in_the_guide_order() {
    // req:init/guide-table-from-files@fa1b0e6f
    // req:init/guide-command@5a6d8da9
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let title = |file: &Path| {
        let text = std::fs::read_to_string(file).unwrap();
        text.lines()
            .find_map(|l| l.strip_prefix("# "))
            .unwrap()
            .trim()
            .to_owned()
    };
    // Each page `README.md` links, at its first link `](<name>.md)` or `](<name>.md#...)`,
    // then the pages it doesn't link, by name in byte order.
    let readme = std::fs::read_to_string(docs.join("guide/README.md")).unwrap();
    let mut pages: Vec<String> = std::fs::read_dir(docs.join("guide"))
        .unwrap()
        .filter_map(|e| {
            let file = e.unwrap().file_name().into_string().unwrap();
            file.strip_suffix(".md")
                .filter(|stem| *stem != "README")
                .map(str::to_owned)
        })
        .collect();
    pages.sort();
    let mut names: Vec<String> = Vec::new();
    for name in first_links(&readme).into_iter().chain(pages.clone()) {
        if pages.contains(&name) && !names.contains(&name) {
            names.push(name);
        }
    }
    let mut expected = vec![format!("index  {}", title(&docs.join("guide/README.md")))];
    for name in &names {
        let file = docs.join(format!("guide/{name}.md"));
        expected.push(format!("{name}  {}", title(&file)));
    }
    expected.push(format!(
        "spec-format  {}",
        title(&docs.join("spec-format.md"))
    ));
    expected.push("constraints  CLM's constraints".to_owned());
    let out = guide(&[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines, expected, "{text}");
}

/// Each `<name>` of a link `](<name>.md)` or `](<name>.md#<anchor>)` in `text`, in order,
/// `<name>` being ASCII letters, digits, `_` and `-`.
fn first_links(text: &str) -> Vec<String> {
    text.split("](")
        .skip(1)
        .filter_map(|rest| {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .unwrap_or(rest.len());
            let (name, after) = rest.split_at(end);
            let after = after.strip_prefix(".md")?;
            (!name.is_empty() && (after.starts_with(')') || after.starts_with('#')))
                .then(|| name.to_owned())
        })
        .collect()
}

#[test]
fn one_page() {
    // req:init/guide-command@5a6d8da9
    let out = guide(&["shaping-the-graph"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/shaping-the-graph.md");
    assert_eq!(out.stdout, std::fs::read(page).unwrap());
}

#[test]
fn the_spec_format() {
    // req:init/guide-command@5a6d8da9
    let out = guide(&["spec-format"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec-format.md");
    assert_eq!(out.stdout, std::fs::read(page).unwrap());
}

#[test]
fn unknown_page() {
    // req:init/guide-command@5a6d8da9
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

#[test]
fn constraints_page() {
    // req:init/guide-constraints-page@1502a90a
    let out = guide(&["constraints"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "# CLM's constraints");
    let after = |heading: &str| -> Vec<&str> {
        let at = lines.iter().position(|l| *l == heading).expect(heading);
        let end = lines[at + 1..]
            .iter()
            .position(|l| l.starts_with("## "))
            .map_or(lines.len(), |i| at + 1 + i);
        lines[at + 1..end].to_vec()
    };
    assert!(after("## proven-by-tests").contains(&"Tier: method"));
    let replaceable = after("## expand-contract");
    assert!(replaceable.contains(&"Tier: replaceable"));
    assert!(replaceable.iter().any(|l| l.starts_with("Guarantee: ")));
    assert!(after("## yagni").contains(&"Tier: default"));
    assert!(!lines.contains(&"## web-optional"), "{text}");
    for tier in ["method", "replaceable", "default"] {
        assert!(
            lines[1..]
                .iter()
                .take_while(|l| !l.starts_with("## "))
                .any(|l| l.contains(&format!("`{tier}`"))),
            "{tier}"
        );
    }
    assert!(text.contains("#### Replaces") && text.contains("CLM's repository"));
    // Each entry's statement is printed, in the order of CLM's `ARCHITECTURE.md`.
    assert!(
        after("## proven-by-tests")
            .iter()
            .any(|l| l.contains("A requirement SHALL count as")),
        "{text}"
    );
    let at = |h: &str| lines.iter().position(|l| *l == h).unwrap();
    assert!(at("## proven-by-tests") < at("## expand-contract"));
}
