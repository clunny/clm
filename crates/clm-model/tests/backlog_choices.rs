use std::fs;
use std::path::Path;

use clm_model::load;
use tempfile::TempDir;

const NODE: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n";

fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let mut all = vec![("specs/SPEC.md", NODE)];
    all.extend_from_slice(files);
    for (rel, content) in all {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
}

fn item(kind: &str, choices: &str) -> String {
    format!("---\ntitle: T\nkind: {kind}\nchoices: {choices}\n---\n")
}

#[test]
fn a_question_carries_its_choices() {
    // req:model/backlog-choices@04cc6947
    let dir = repo(&[
        ("backlog/q.md", &item("question", "[yes, ' no ']")),
        ("backlog/n.md", "---\ntitle: T\nkind: question\n---\n"),
        ("backlog/e.md", &item("question", "[]")),
    ]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.backlog["q"].frontmatter.choices, ["yes", "no"]);
    assert!(graph.backlog["n"].frontmatter.choices.is_empty());
    assert!(graph.backlog["e"].frontmatter.choices.is_empty());
}

#[test]
fn bad_choices_are_refused_by_load_and_by_parse_backlog_item() {
    // req:model/backlog-choices@04cc6947
    let long = format!("[yes, {}]", "x".repeat(41));
    let bad = [
        ("a", item("idea", "[yes, no]")),
        ("b", item("question", "[yes]")),
        ("c", item("question", "[a, b, c, d, e, f]")),
        ("d", item("question", "[yes, '  ']")),
        ("e", item("question", &long)),
        ("f", item("question", "[yes, \"a\\nb\"]")),
        ("g", item("question", "[yes, ' yes']")),
    ];
    let paths: Vec<String> = bad.iter().map(|(s, _)| format!("backlog/{s}.md")).collect();
    let files: Vec<(&str, &str)> = paths
        .iter()
        .zip(&bad)
        .map(|(p, (_, t))| (p.as_str(), t.as_str()))
        .collect();
    let dir = repo(&files);
    let (_, violations) = load(dir.path());
    for ((slug, text), path) in bad.iter().zip(&paths) {
        assert!(
            violations
                .iter()
                .any(|v| v.rule == "model/backlog-choices" && v.path == Path::new(path)),
            "{slug}: {violations:?}"
        );
        let err = clm_model::parse_backlog_item(slug, text).unwrap_err();
        assert!(
            err.iter().any(|v| v.rule == "model/backlog-choices"),
            "{slug}"
        );
    }
    // five entries, one of 40 characters, are within the limits.
    let ok = item("question", &format!("[{}, b, c, d, e]", "x".repeat(40)));
    assert!(clm_model::parse_backlog_item("ok", &ok).is_ok());
}
