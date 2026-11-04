use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Violation, load_files};

/// A node SPEC.md with `fm_extra` frontmatter lines, `design` text and requirement blocks.
fn spec(fm_extra: &str, design: &str, requirements: &[&str]) -> String {
    format!(
        "---\ntitle: T\nstatus: active\nowners: []{fm_extra}\n---\n\n## Contract\nc\n\n## Design\n{design}\n\n## Requirements\n\n{}",
        requirements.join("\n")
    )
}

/// Requirement `slug` with one scenario, then `extra` lines.
fn req(slug: &str, extra: &str) -> String {
    format!("### Requirement: {slug}\nIt SHALL.\n\n#### Scenario: s\n- ok\n{extra}")
}

fn needs(ids: &[&str]) -> String {
    let lines: Vec<String> = ids.iter().map(|id| format!("- {id}\n")).collect();
    format!("\n#### Needs\n{}", lines.concat())
}

/// Loads root `clm` plus nodes `(id, spec)`.
fn load(nodes: &[(&str, String)]) -> (Graph, Vec<Violation>) {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), spec("", "d", &[]).into());
    for (id, text) in nodes {
        let path = format!("specs/{}/SPEC.md", id.replace('.', "/"));
        files.insert(PathBuf::from(path), text.clone().into());
    }
    load_files(&files)
}

fn clean(nodes: &[(&str, String)]) -> Graph {
    let (graph, violations) = load(nodes);
    assert!(violations.is_empty(), "unexpected: {violations:?}");
    graph
}

fn needs_of<'a>(graph: &'a Graph, id: &str) -> &'a [String] {
    &graph.requirement(id).unwrap().needs
}

/// The one violation, which is `model/requirement-needs` on `specs/a/SPEC.md` and names
/// each of `names`, with node `a` still loaded.
fn one_violation(nodes: &[(&str, String)], names: &[&str]) {
    let (graph, violations) = load(nodes);
    assert_eq!(violations.len(), 1, "{violations:?}");
    let v = &violations[0];
    assert_eq!(v.rule, "model/requirement-needs");
    assert_eq!(v.path, Path::new("specs/a/SPEC.md"));
    for name in names {
        assert!(v.message.contains(&format!("`{name}`")), "{name}: {v:?}");
    }
    assert!(graph.nodes.contains_key("a"));
}

#[test]
fn need_parsed() {
    // req:model/requirement-needs@0d58e0b7
    let graph = clean(&[
        ("a", spec("", "d", &[&req("x", &needs(&["b/y"]))])),
        ("b", spec("", "d", &[&req("y", "")])),
    ]);
    assert_eq!(needs_of(&graph, "a/x"), ["b/y"]);
    assert!(needs_of(&graph, "b/y").is_empty());
}

#[test]
fn block_ignores_blank_lines_and_ends_at_the_next_heading() {
    // req:model/requirement-needs@0d58e0b7
    let x =
        "### Requirement: x\nIt SHALL.\n\n#### Needs\n\n- b/y\n\n- b/z\n\n#### Scenario: s\n- ok\n";
    let graph = clean(&[
        ("a", spec("", "d", &[x, &req("w", "")])),
        ("b", spec("", "d", &[&req("y", ""), &req("z", "")])),
    ]);
    assert_eq!(needs_of(&graph, "a/x"), ["b/y", "b/z"]);
    assert_eq!(graph.requirement("a/x").unwrap().scenarios, 1);
    assert!(needs_of(&graph, "a/w").is_empty());
    // A block last in the file's requirements ends at the next section.
    let a = format!(
        "{}\n## Notes\n- a note\n",
        spec("", "d", &[&req("x", &needs(&["b/y"]))])
    );
    let graph = clean(&[("a", a), ("b", spec("", "d", &[&req("y", "")]))]);
    assert_eq!(needs_of(&graph, "a/x"), ["b/y"]);
}

#[test]
fn needs_heading_in_a_code_fence_is_not_a_block() {
    // req:model/requirement-needs@0d58e0b7
    let fenced = req("x", "\n```markdown\n#### Needs\nneeds b/y\n```\n");
    let graph = clean(&[("a", spec("", "```\n#### Needs\n```", &[&fenced]))]);
    assert!(needs_of(&graph, "a/x").is_empty());
}

#[test]
fn malformed_line() {
    // req:model/requirement-needs@0d58e0b7
    for line in ["needs b/y", "- b", "- b/y z", "- /y", "- b/", "- `b/y`"] {
        let x = req("x", &format!("\n#### Needs\n{line}\n"));
        let nodes = [
            ("a", spec("", "d", &[&x])),
            ("b", spec("", "d", &[&req("y", "")])),
        ];
        one_violation(&nodes, &["a/x"]);
        assert!(needs_of(&load(&nodes).0, "a/x").is_empty(), "{line}");
    }
}

#[test]
fn second_needs_block() {
    // req:model/requirement-needs@0d58e0b7
    let x = req("x", &format!("{}{}", needs(&["b/y"]), needs(&["b/y"])));
    one_violation(
        &[
            ("a", spec("", "d", &[&x])),
            ("b", spec("", "d", &[&req("y", "")])),
        ],
        &["a/x"],
    );
}

#[test]
fn needs_outside_any_requirement() {
    // req:model/requirement-needs@0d58e0b7
    let (graph, violations) = load(&[
        ("a", spec("", &needs(&["b/y"]), &[&req("x", "")])),
        ("b", spec("", "d", &[&req("y", "")])),
    ]);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/requirement-needs");
    assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
    assert!(graph.nodes.contains_key("a"));
}

#[test]
fn dangling_need() {
    // req:model/requirement-needs@0d58e0b7
    for target in ["b/ghost", "nope/y"] {
        one_violation(
            &[
                ("a", spec("", "d", &[&req("x", &needs(&[target]))])),
                ("b", spec("", "d", &[&req("y", "")])),
            ],
            &["a/x", target],
        );
    }
}

#[test]
fn same_node_need() {
    // req:model/requirement-needs@0d58e0b7
    one_violation(
        &[(
            "a",
            spec("", "d", &[&req("x", &needs(&["a/y"])), &req("y", "")]),
        )],
        &["a/x", "a/y"],
    );
}

#[test]
fn need_cycle() {
    // req:model/requirement-needs@0d58e0b7
    one_violation(
        &[
            ("a", spec("", "d", &[&req("x", &needs(&["b/y"]))])),
            ("b", spec("", "d", &[&req("y", &needs(&["a/x"]))])),
        ],
        &["a/x", "b/y"],
    );
    // Three nodes: reported once.
    one_violation(
        &[
            ("a", spec("", "d", &[&req("x", &needs(&["b/y"]))])),
            ("b", spec("", "d", &[&req("y", &needs(&["c/z"]))])),
            ("c", spec("", "d", &[&req("z", &needs(&["a/x"]))])),
        ],
        &["a/x", "b/y", "c/z"],
    );
}

#[test]
fn opposite_directions_are_not_a_cycle() {
    // req:model/requirement-needs@0d58e0b7
    let graph = clean(&[
        (
            "a",
            spec("", "d", &[&req("x", &needs(&["b/y"])), &req("w", "")]),
        ),
        (
            "b",
            spec("", "d", &[&req("y", ""), &req("z", &needs(&["a/w"]))]),
        ),
    ]);
    assert_eq!(needs_of(&graph, "b/z"), ["a/w"]);
}

#[test]
fn required_needs_optional() {
    // req:model/requirement-needs@0d58e0b7
    let optional = "\noptional: true";
    one_violation(
        &[
            ("a", spec("", "d", &[&req("x", &needs(&["b/y"]))])),
            ("b", spec(optional, "d", &[&req("y", "")])),
        ],
        &["a/x", "b/y"],
    );
    // Optional by inheritance.
    one_violation(
        &[
            ("a", spec("", "d", &[&req("x", &needs(&["b.c/y"]))])),
            ("b", spec(optional, "d", &[])),
            ("b.c", spec("", "d", &[&req("y", "")])),
        ],
        &["a/x", "b.c/y"],
    );
    // An optional node may need a required one.
    clean(&[
        ("a", spec("", "d", &[&req("x", "")])),
        ("b", spec(optional, "d", &[&req("y", &needs(&["a/x"]))])),
    ]);
    // An optional node may need another optional one.
    clean(&[
        ("a", spec(optional, "d", &[&req("x", &needs(&["b/y"]))])),
        ("b", spec(optional, "d", &[&req("y", "")])),
    ]);
}

#[test]
fn a_need_is_part_of_the_version() {
    // req:model/requirement-needs@0d58e0b7
    let b = ("b", spec("", "d", &[&req("y", "")]));
    let without = clean(&[("a", spec("", "d", &[&req("x", "")])), b.clone()]);
    let with = clean(&[("a", spec("", "d", &[&req("x", &needs(&["b/y"]))])), b]);
    assert_ne!(
        without.requirement("a/x").unwrap().version(),
        with.requirement("a/x").unwrap().version()
    );
}
