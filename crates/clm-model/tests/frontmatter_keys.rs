//! Node frontmatter keys that live changes declare in `frontmatter_keys`.

use std::fs;
use std::path::Path;

use clm_model::{Violation, check_node_frontmatter, load};
use tempfile::TempDir;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

const FM: &str = "title: T\nstatus: specd\nowners: [\"@a\"]";

/// A repo whose root node has frontmatter `FM` plus `extra`.
fn repo(extra: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        &format!("---\n{FM}{extra}\n---\n\n## Contract\nc\n"),
    );
    dir
}

fn change(root: &Path, dir: &str, status: &str, keys: &str) {
    write(
        root,
        &format!("changes/{dir}/change.md"),
        &format!("---\nstatus: {status}\n{keys}---\n"),
    );
}

#[test]
fn declared_frontmatter_keys_are_parsed() {
    // req:model/parse-changes@e3b5847d
    let dir = repo("");
    change(
        dir.path(),
        "x",
        "proposed",
        "frontmatter_keys: [colour, size_hint]\n",
    );
    change(dir.path(), "y", "proposed", "");
    change(dir.path(), "z", "proposed", "frontmatter_keys: [Colour]\n");
    let (graph, violations) = load(dir.path());
    assert_eq!(
        graph.changes["x"].frontmatter.frontmatter_keys,
        ["colour", "size_hint"]
    );
    assert!(graph.changes["y"].frontmatter.frontmatter_keys.is_empty());
    assert!(!graph.changes.contains_key("z"));
    assert_eq!(rules(&violations), ["model/parse-changes"]);
    assert_eq!(violations[0].path, Path::new("changes/z/change.md"));
    assert!(violations[0].message.contains("Colour"), "{violations:?}");
}

#[test]
fn malformed_key_names_are_rejected() {
    // req:model/parse-changes@e3b5847d
    for bad in [
        "[1st]", "[_x]", "[a-b]", "[\"\"]", "[a.b]", "colour", "[[a]]",
    ] {
        let dir = repo("");
        change(
            dir.path(),
            "z",
            "proposed",
            &format!("frontmatter_keys: {bad}\n"),
        );
        let (graph, violations) = load(dir.path());
        assert!(!graph.changes.contains_key("z"), "{bad}");
        assert_eq!(rules(&violations), ["model/parse-changes"], "{bad}");
    }
}

#[test]
fn unknown_key_is_rejected() {
    // req:model/valid-frontmatter@b4618360
    let dir = repo("\ncolour: blue");
    change(
        dir.path(),
        "x",
        "applied",
        "frontmatter_keys: [size_hint]\n",
    );
    let (graph, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/valid-frontmatter"]);
    assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
    assert!(violations[0].message.contains("colour"), "{violations:?}");
    assert!(graph.nodes.is_empty());
}

#[test]
fn key_a_live_change_declares() {
    // req:model/valid-frontmatter@b4618360
    let plain = repo("");
    let plain = &load(plain.path()).0.nodes["clm"];
    for extra in [
        "\ncolour: blue",
        "\ncolour:\n  - a\n  - b",
        "\ncolour:\n- a",
    ] {
        let dir = repo(extra);
        change(dir.path(), "x", "applied", "frontmatter_keys: [colour]\n");
        let (graph, violations) = load(dir.path());
        assert!(violations.is_empty(), "{extra}: {violations:?}");
        let node = &graph.nodes["clm"];
        assert_eq!(node.frontmatter, plain.frontmatter, "{extra}");
        assert_ne!(node.hashes.frontmatter, plain.hashes.frontmatter, "{extra}");
    }
}

#[test]
fn declaration_no_longer_live() {
    // req:model/valid-frontmatter@b4618360
    for (dir_name, status) in [("archive/x", "done"), ("x", "abandoned")] {
        let dir = repo("\ncolour: blue");
        change(dir.path(), dir_name, status, "frontmatter_keys: [colour]\n");
        let (_, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/valid-frontmatter"], "{status}");
        assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
        assert!(violations[0].message.contains("colour"), "{violations:?}");
    }
}

#[test]
fn unparseable_change_declares_nothing() {
    // req:model/valid-frontmatter@b4618360
    let dir = repo("\ncolour: blue");
    change(
        dir.path(),
        "x",
        "applied",
        "frontmatter_keys: [colour]\nbogus: 1\n",
    );
    let (_, violations) = load(dir.path());
    assert_eq!(
        rules(&violations),
        ["model/parse-changes", "model/valid-frontmatter"]
    );
}

#[test]
fn declared_key_the_model_knows() {
    // req:model/valid-frontmatter@b4618360
    let dir = repo("\noptional: maybe");
    change(dir.path(), "x", "applied", "frontmatter_keys: [optional]\n");
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/optional-nodes"]);
    assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
    assert!(violations[0].message.contains("optional"), "{violations:?}");

    let dir = repo("\nsuccessor: [1, 2]");
    change(
        dir.path(),
        "x",
        "applied",
        "frontmatter_keys: [successor]\n",
    );
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/valid-frontmatter"]);

    let dir = repo("\nserves: 5");
    change(dir.path(), "x", "applied", "frontmatter_keys: [serves]\n");
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/valid-frontmatter"]);
    assert!(violations[0].message.contains("serves"), "{violations:?}");
}

#[test]
fn declared_keys_of_the_graph() {
    // req:model/valid-frontmatter@b4618360
    let dir = repo("");
    change(dir.path(), "a", "proposed", "frontmatter_keys: [colour]\n");
    change(dir.path(), "b", "abandoned", "frontmatter_keys: [size]\n");
    change(
        dir.path(),
        "archive/c",
        "done",
        "frontmatter_keys: [shape]\n",
    );
    change(
        dir.path(),
        "d",
        "applied",
        "frontmatter_keys: [hue, colour]\n",
    );
    let (graph, _) = load(dir.path());
    assert_eq!(
        graph
            .declared_frontmatter_keys()
            .into_iter()
            .collect::<Vec<_>>(),
        ["colour", "hue"]
    );
}

#[test]
fn errors_point_at_the_file_lines() {
    // req:model/valid-frontmatter@b4618360
    let declared = ["colour".to_string()].into();
    let fm = "colour:\n  - a\n  - b\ntitle: T\nowners: [\"@a\"]\nstatus: done";
    let errors =
        check_node_frontmatter("clm", Path::new("specs/SPEC.md"), fm, &declared).unwrap_err();
    assert!(errors[0].message.contains("line 6"), "{errors:?}");
}

#[test]
fn bare_frontmatter_keys_declare_none() {
    // req:model/parse-changes@e3b5847d
    let dir = repo("");
    change(dir.path(), "x", "proposed", "frontmatter_keys:\n");
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.changes["x"].frontmatter.frontmatter_keys.is_empty());
}

#[test]
fn the_frontmatter_check_apply_shares() {
    // req:model/valid-frontmatter@b4618360
    // req:model/optional-nodes@37ade64d
    let path = Path::new("specs/a/SPEC.md");
    let declared = ["colour".to_string()].into();
    let with = |extra: &str| format!("{FM}{extra}");
    let check = |fm: &str| check_node_frontmatter("a", path, fm, &declared);
    let (fm, violations) = check(&with("\ncolour: blue")).unwrap();
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(fm, check(FM).unwrap().0);
    // Other rules for any node: the ones the text alone decides.
    for (extra, key) in [
        ("\nshape: round", "shape"),
        ("\nsize_budget: { colour: 1 }", "size_budget"),
        ("\noptional: maybe", "optional"),
    ] {
        let errors = check(&with(extra)).unwrap_err();
        assert_eq!(errors.len(), 1, "{extra}: {errors:?}");
        assert_eq!(errors[0].path, path);
        assert!(errors[0].message.contains(key), "{extra}: {errors:?}");
    }
}
