//! A pending capability waits on the `unbuilt` rules that serve it, and on the `behind` ones that an item or a wait blocks
//! (`serving-rules-order-capabilities`).

mod common;

use clm_coverage::{Pending, pending};
use common::{ROOT, current, graph, marker, write};
use tempfile::TempDir;

const CAPABILITY_NODE: &str = "---\ntitle: Capabilities\nstatus: active\nowners: []\npaths: [\"tests/capabilities/**\"]\n---\n\n## Contract\nThe tests.\n\n## Design\nThe harness.\n";

fn node(extra: &str, rules: &[(&str, bool)]) -> String {
    let mut s = format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"src/**\"]\n{extra}---\n\n## Requirements\n"
    );
    for (slug, serves) in rules {
        s.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
        if *serves {
            s.push_str("\n#### Serves\n- capability/c\n");
        }
    }
    s
}

/// Root, the `capability` node, capability `c` and the given node specs.
fn repo(nodes: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/capability/SPEC.md", CAPABILITY_NODE);
    write(d, "ARCHITECTURE.md", "## Requirements\n");
    write(
        d,
        "CAPABILITIES.md",
        "## Capabilities\n\n### Capability: c\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
    );
    for (id, spec) in nodes {
        write(d, &format!("specs/{id}/SPEC.md"), spec);
    }
    dir
}

/// The line `clm pending capability` prints for `id`: it must be on the `capability` node.
fn line(list: &[Pending], id: &str) -> String {
    let p = list.iter().find(|p| p.id == id).unwrap();
    assert_eq!(p.node, "capability", "{id} is pending on the wrong node");
    p.to_string()
}

#[test]
fn capability_waits_on_its_serving_rules() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo(&[
        (
            "a",
            node(
                "serves: [capability/c]\n",
                &[("x", false), ("y", false), ("z", false)],
            ),
        ),
        ("b", node("", &[("w", true)])),
    ]);
    let g = graph(dir.path());
    let markers = [
        marker("a/y", Some("00000000"), "tests/t.rs", 1),
        current(&g, "a/z", "tests/t.rs", 2),
    ];
    let list = pending(&g, &markers);
    let v = g.requirement("capability/c").unwrap().version();
    assert_eq!(
        line(&list, "capability/c"),
        format!("capability/c {v} unbuilt blocked a/x,b/w")
    );
}

#[test]
fn serving_rules_built() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo(&[(
        "a",
        node("serves: [capability/c]\n", &[("y", false), ("z", false)]),
    )]);
    let g = graph(dir.path());
    let markers = [
        marker("a/y", Some("00000000"), "tests/t.rs", 1),
        current(&g, "a/z", "tests/t.rs", 2),
    ];
    let list = pending(&g, &markers);
    let v = g.requirement("capability/c").unwrap().version();
    assert_eq!(
        line(&list, "capability/c"),
        format!("capability/c {v} unbuilt")
    );
}

#[test]
fn edited_capability_waits_on_a_new_serving_rule() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo(&[("a", node("serves: [capability/c]\n", &[("x", false)]))]);
    let g = graph(dir.path());
    let markers = [marker("capability/c", Some("00000000"), "tests/t.rs", 1)];
    let list = pending(&g, &markers);
    assert!(line(&list, "capability/c").ends_with(" blocked a/x"));
}

#[test]
fn serving_rule_on_an_optional_node() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = repo(&[(
        "w",
        node("optional: true\nserves: [capability/c]\n", &[("x", false)]),
    )]);
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [capability/c]\n---\n",
    );
    let g = graph(dir.path());
    let list = pending(&g, &[]);
    let v = g.requirement("capability/c").unwrap().version();
    assert_eq!(
        line(&list, "capability/c"),
        format!("capability/c {v} unbuilt blocked backlog/q,w/x")
    );
}

#[test]
fn obsolete_capability_waits_on_no_serving_rule() {
    // req:check.coverage/pending-report@4a6b0203
    // Only an `unbuilt` or `behind` capability waits on its serving rules. A node still serving
    // a removed capability is a `model/serves` violation, but the queue reads the graph anyway.
    let dir = repo(&[("a", node("serves: [capability/gone]\n", &[("x", false)]))]);
    let d = dir.path();
    write(
        d,
        "changes/20260101-r/change.md",
        "---\nstatus: applied\n---\n",
    );
    write(
        d,
        "changes/20260101-r/deltas/capability.md",
        "---\ncomponent: capability\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0123456789abcdef -->\n",
    );
    let (g, _) = clm_model::load(d);
    let list = pending(&g, &[marker("capability/gone", None, "tests/t.rs", 3)]);
    let p = list.iter().find(|p| p.id == "capability/gone").unwrap();
    assert_eq!(p.to_string(), "capability/gone obsolete tests/t.rs:3");
    assert!(list.iter().any(|p| p.id == "a/x"), "{list:?}");
}

#[test]
fn every_wait_in_one_list() {
    // req:check.coverage/pending-report@4a6b0203
    let a = "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"a/**\"]\n---\n\n## Requirements\n\n### Requirement: y\ny.\n\n#### Scenario: s\n- ok\n\n#### Needs\n- b/z\n";
    let dir = repo(&[
        ("a", a.to_string()),
        ("b", node("", &[("z", false)]).replace("src/**", "b/**")),
        (
            "w",
            node("serves: [capability/c]\n", &[("x", false)]).replace("src/**", "w/**"),
        ),
    ]);
    let list = pending(&graph(dir.path()), &[]);
    let of = |id: &str| list.iter().find(|p| p.id == id).unwrap();
    assert_eq!(of("a/y").waits_on, ["b/z"]);
    assert_eq!(of("capability/c").waits_on, ["w/x"]);
    assert!(of("a/y").blocked_by.is_empty() && of("capability/c").blocked_by.is_empty());
}

/// The scenario's components: `a` serving `capability/c` with `x` (needing `b/z`), `w` and `y`,
/// `b` holding `z`, and every `a` rule cited at an older version.
fn changed_rules() -> TempDir {
    let a = "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"a/**\"]\nserves: [capability/c]\n---\n\n## Requirements\n\n### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n\n#### Needs\n- b/z\n\n### Requirement: w\nw.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\ny.\n\n#### Scenario: s\n- ok\n";
    let dir = repo(&[
        ("a", a.to_string()),
        ("b", node("", &[("z", false)]).replace("src/**", "b/**")),
    ]);
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [a/w]\n---\n",
    );
    dir
}

fn old_markers() -> Vec<clm_coverage::Marker> {
    ["capability/c", "a/x", "a/w", "a/y"]
        .iter()
        .enumerate()
        .map(|(i, id)| marker(id, Some("00000000"), "tests/t.rs", i + 1))
        .collect()
}

#[test]
fn changed_serving_rule_that_waits() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = changed_rules();
    let list = pending(&graph(dir.path()), &old_markers());
    let of = |id: &str| list.iter().find(|p| p.id == id).unwrap().to_string();
    assert!(of("capability/c").ends_with(" blocked a/w,a/x"), "{list:?}");
    assert!(of("a/x").ends_with(" blocked b/z"));
    assert!(of("a/w").ends_with(" blocked backlog/q"));
    assert!(!of("a/y").contains("blocked"));
}

#[test]
fn changed_serving_rule_released() {
    // req:check.coverage/pending-report@4a6b0203
    let dir = changed_rules();
    std::fs::remove_file(dir.path().join("backlog/q.md")).unwrap();
    let g = graph(dir.path());
    let mut markers = old_markers();
    markers.push(current(&g, "b/z", "tests/t.rs", 9));
    let list = pending(&g, &markers);
    let v = g.requirement("capability/c").unwrap().version();
    assert_eq!(
        line(&list, "capability/c"),
        format!("capability/c {v} behind 00000000")
    );
}
