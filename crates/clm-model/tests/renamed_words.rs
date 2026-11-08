//! The words and keys `plain-terms-renames` replaced: the model writes the new ones and
//! still reads the old ones (`renamed-words-still-load`), and the component keys
//! (`component-keys`).

use std::path::{Path, PathBuf};

use clm_model::{BacklogKind, Files, Status, Violation, load_files};

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";
const NODE_A: &str = "---\ntitle: A\nstatus: unbuilt\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nThe node SHALL x.\n\n#### Scenario: s\n- GIVEN g\n- THEN t\n";

fn load(extra: &[(&str, &str)]) -> (clm_model::Graph, Vec<Violation>) {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
    files.insert(PathBuf::from("specs/a/SPEC.md"), NODE_A.into());
    for (path, text) in extra {
        files.insert(PathBuf::from(path), text.as_bytes().to_vec());
    }
    load_files(&files)
}

fn clean(extra: &[(&str, &str)]) -> clm_model::Graph {
    let (graph, violations) = load(extra);
    assert!(violations.is_empty(), "unexpected: {violations:?}");
    graph
}

/// The one violation, which is `rule` on `path` and names each of `names`.
fn one_violation(extra: &[(&str, &str)], rule: &str, path: &str, names: &[&str]) {
    let (_, violations) = load(extra);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, rule, "{violations:?}");
    assert_eq!(violations[0].path, Path::new(path), "{violations:?}");
    for name in names {
        assert!(
            violations[0].message.contains(name),
            "{name}: {violations:?}"
        );
    }
}

fn spec_with_status(status: &str) -> String {
    NODE_A.replace("status: unbuilt", &format!("status: {status}"))
}

#[test]
fn the_old_status_name() {
    // req:model/valid-frontmatter@b4618360
    let old = spec_with_status("specd");
    let graph = {
        let mut files = Files::new();
        files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
        files.insert(PathBuf::from("specs/a/SPEC.md"), old.into_bytes());
        let (graph, violations) = load_files(&files);
        assert!(violations.is_empty(), "{violations:?}");
        graph
    };
    assert_eq!(graph.nodes["a"].frontmatter.status, Status::Specd);
    // The new word loads as the same status.
    let graph = clean(&[]);
    assert_eq!(graph.nodes["a"].frontmatter.status, Status::Specd);
}

#[test]
fn a_bad_status_is_still_rejected() {
    // req:model/valid-frontmatter@b4618360
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
    files.insert(
        PathBuf::from("specs/a/SPEC.md"),
        spec_with_status("done").into_bytes(),
    );
    let (_, violations) = load_files(&files);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/valid-frontmatter");
    assert!(violations[0].message.contains("done"), "{violations:?}");
}

#[test]
fn burdens_by_the_new_key_and_the_old() {
    // req:model/parse-changes@e3b5847d
    let change = |key: &str| format!("---\nstatus: proposed\n{key}: [a paid model run]\n---\n");
    let new = clean(&[("changes/x/change.md", &change("human_burdens"))]);
    assert_eq!(new.changes["x"].frontmatter.burdens, ["a paid model run"]);
    let none = clean(&[("changes/y/change.md", "---\nstatus: proposed\n---\n")]);
    assert!(none.changes["y"].frontmatter.burdens.is_empty());
}

#[test]
fn the_old_burdens_key() {
    // req:model/parse-changes@e3b5847d
    let archived = "---\nstatus: done\nburdens: [a paid model run]\n---\n";
    let live = "---\nstatus: proposed\nburdens: [a paid model run]\n---\n";
    let both = "---\nstatus: proposed\nburdens: [a paid model run]\nhuman_burdens: [a paid model run]\n---\n";
    let graph = clean(&[
        ("changes/archive/w/change.md", archived),
        ("changes/v/change.md", live),
    ]);
    for id in ["w", "v"] {
        assert_eq!(graph.changes[id].frontmatter.burdens, ["a paid model run"]);
    }
    let (graph, violations) = load(&[
        ("changes/archive/w/change.md", archived),
        ("changes/x/change.md", both),
    ]);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/parse-changes");
    assert_eq!(violations[0].path, Path::new("changes/x/change.md"));
    for key in ["`human_burdens`", "`burdens`"] {
        assert!(violations[0].message.contains(key), "{violations:?}");
    }
    assert_eq!(graph.changes["w"].frontmatter.burdens, ["a paid model run"]);
    assert!(!graph.changes.contains_key("x"));
}

#[test]
fn a_bare_burdens_key_is_both_keys_present() {
    // req:model/parse-changes@e3b5847d
    let both = "---\nstatus: proposed\nburdens:\nhuman_burdens:\n---\n";
    one_violation(
        &[("changes/x/change.md", both)],
        "model/parse-changes",
        "changes/x/change.md",
        &["`human_burdens`", "`burdens`"],
    );
}

const DELTA_BODY: &str = "\n## MODIFIED Design\nd2\n";

fn delta_base() -> String {
    let (graph, _) = load(&[]);
    graph.nodes["a"].hashes.design.clone()
}

fn delta(key: &str) -> String {
    format!(
        "---\n{key}: a\nbase:\n  design: {}\n---\n{DELTA_BODY}",
        delta_base()
    )
}

const LIVE: &str = "---\nstatus: proposed\n---\n";
const DONE: &str = "---\nstatus: done\n---\n";

#[test]
fn a_delta_by_its_new_key() {
    // req:model/component-keys@deb40bca
    let by_new = clean(&[
        ("changes/c/change.md", LIVE),
        ("changes/c/deltas/a.md", &delta("component")),
    ]);
    assert_eq!(by_new.changes["c"].deltas.len(), 1);
    assert_eq!(by_new.changes["c"].deltas[0].node, "a");
}

#[test]
fn the_old_keys_are_refused() {
    // req:model/component-keys@deb40bca
    let (graph, violations) = load(&[
        ("backlog/x.md", &item("nodes", "[a]")),
        ("changes/c/change.md", LIVE),
        ("changes/c/deltas/a.md", &delta("node")),
    ]);
    assert_eq!(violations.len(), 2, "{violations:?}");
    for (path, old, new) in [
        ("backlog/x.md", "`nodes`", "`components`"),
        ("changes/c/deltas/a.md", "`node`", "`component`"),
    ] {
        let found = violations
            .iter()
            .find(|v| v.path == Path::new(path))
            .unwrap_or_else(|| panic!("{path}: {violations:?}"));
        assert!(found.message.contains(old) && found.message.contains(new));
    }
    // The delta is not loaded; the item stays, so a merge base read through here holds it.
    assert!(graph.changes["c"].deltas.is_empty());
    assert_eq!(graph.backlog["x"].frontmatter.nodes, ["a"]);
}

#[test]
fn a_live_delta_with_both_keys_is_one_violation() {
    // req:model/component-keys@deb40bca
    let both = format!(
        "---\ncomponent: a\nnode: a\nbase:\n  design: {}\n---\n{DELTA_BODY}",
        delta_base()
    );
    one_violation(
        &[
            ("changes/c/change.md", LIVE),
            ("changes/c/deltas/a.md", &both),
        ],
        "model/parse-changes",
        "changes/c/deltas/a.md",
        &["`component`", "`node`"],
    );
}

#[test]
fn the_archive_keeps_its_key() {
    // req:model/component-keys@deb40bca
    let by_old = clean(&[
        ("changes/archive/c/change.md", DONE),
        ("changes/archive/c/deltas/a.md", &delta("node")),
    ]);
    let by_new = clean(&[
        ("changes/archive/c/change.md", DONE),
        ("changes/archive/c/deltas/a.md", &delta("component")),
    ]);
    assert_eq!(by_old.changes["c"].deltas.len(), 1);
    assert!(by_old.changes["c"].archived);
    assert_eq!(by_old.changes["c"].deltas, by_new.changes["c"].deltas);
}

#[test]
fn an_archived_delta_with_both_keys_is_rejected() {
    // req:model/component-keys@deb40bca
    let both = format!(
        "---\ncomponent: a\nnode: a\nbase:\n  design: {}\n---\n{DELTA_BODY}",
        delta_base()
    );
    one_violation(
        &[
            ("changes/archive/c/change.md", DONE),
            ("changes/archive/c/deltas/a.md", &both),
        ],
        "model/parse-changes",
        "changes/archive/c/deltas/a.md",
        &["`component`", "`node`"],
    );
}

#[test]
fn a_delta_naming_no_component_is_rejected() {
    // req:model/component-keys@deb40bca
    let neither = "---\ncreate: true\n---\n";
    one_violation(
        &[
            ("changes/c/change.md", LIVE),
            ("changes/c/deltas/a.md", neither),
        ],
        "model/parse-changes",
        "changes/c/deltas/a.md",
        &["`component`"],
    );
}

fn item(key: &str, value: &str) -> String {
    format!("---\ntitle: T\nkind: bug\n{key}: {value}\n---\n")
}

#[test]
fn an_item_by_its_new_key() {
    // req:model/component-keys@deb40bca
    let new = clean(&[("backlog/x.md", &item("components", "[a]"))]);
    assert_eq!(new.backlog["x"].frontmatter.nodes, ["a"]);
}

#[test]
fn a_history_read_still_takes_the_old_key() {
    // req:model/component-keys@deb40bca
    let read = |text: String| -> clm_model::BacklogFrontmatter {
        let (fm, _) = clm_model::split_frontmatter(&text).unwrap();
        serde_yaml::from_str(fm).unwrap()
    };
    let old = read(item("nodes", "[a]"));
    let new = read(item("components", "[a]"));
    assert_eq!(old, new);
}

#[test]
fn an_unknown_component() {
    // req:model/component-keys@deb40bca
    one_violation(
        &[("backlog/x.md", &item("components", "[ghost]"))],
        "model/parse-backlog",
        "backlog/x.md",
        &["ghost"],
    );
}

#[test]
fn an_item_with_both_keys_is_rejected() {
    // req:model/component-keys@deb40bca
    let text = "---\ntitle: T\nkind: bug\ncomponents: [a]\nnodes: [a]\n---\n";
    let (_, violations) = load(&[("backlog/x.md", text)]);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].path, Path::new("backlog/x.md"));
    for key in ["`components`", "`nodes`"] {
        assert!(violations[0].message.contains(key), "{violations:?}");
    }
}

#[test]
fn the_old_kind_names_load_and_print_as_the_new() {
    // req:model/parse-backlog@fa4b09c8
    let flag = |kind: &str, requirement: String| {
        format!("---\ntitle: T\nkind: {kind}\n{requirement}---\n")
    };
    let (graph, _) = load(&[]);
    let version = graph.requirement("a/x").unwrap().version().to_string();
    let req = format!("requirement: a/x@{version}\n");
    let graph = clean(&[
        ("backlog/h.md", &flag("human", String::new())),
        ("backlog/h2.md", &flag("human-task", String::new())),
        ("backlog/f.md", &flag("friction", req.clone())),
        ("backlog/f2.md", &flag("workaround", req)),
    ]);
    for slug in ["h", "h2"] {
        assert_eq!(graph.backlog[slug].frontmatter.kind, BacklogKind::Human);
    }
    for slug in ["f", "f2"] {
        assert_eq!(graph.backlog[slug].frontmatter.kind, BacklogKind::Friction);
    }
    assert_eq!(BacklogKind::Human.to_string(), "human-task");
    assert_eq!(BacklogKind::Friction.to_string(), "workaround");
}
