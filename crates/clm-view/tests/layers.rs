//! The human layers' entries in the views: a cited capability counts on the node the
//! capabilities are pending on, an entry standing by a proof counts on no node, and the
//! summary counts the architecture entries enforced and the untested entries apart.

use std::fs;
use std::path::Path;

use clm_model::Graph;
use tempfile::TempDir;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn node(title: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n"
    )
}

/// An architecture entry `slug`, with an `#### Enforced by` block naming `rules` (none: no
/// block) and, when `untested`, an `#### Untested` block.
fn arch(slug: &str, rules: &[&str], untested: bool) -> String {
    let mut s = format!("### Constraint: {slug}\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n");
    if !rules.is_empty() {
        s.push_str("\n#### Enforced by\n");
        for r in rules {
            s.push_str(&format!("- {r}\n"));
        }
    }
    if untested {
        s.push_str("\n#### Untested\nA judgement.\n");
    }
    s
}

/// Capabilities `(slug, untested)`.
fn capabilities(entries: &[(&str, bool)]) -> String {
    let entries: Vec<String> = entries
        .iter()
        .map(|(s, untested)| {
            let mut e =
                format!("### Capability: {s}\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n");
            if *untested {
                e.push_str("\n#### Untested\nNo program sees it.\n");
            }
            e
        })
        .collect();
    format!("## Capabilities\n\n{}", entries.join("\n"))
}

/// Root `clm` with no requirements, node `a` with `a/x`, the given architecture entries
/// (no `ARCHITECTURE.md` when none) and capabilities, and the extra spec files.
fn graph(
    arch_entries: &[String],
    caps: &[(&str, bool)],
    extra: &[(&str, String)],
) -> (TempDir, Graph) {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("A"));
    if !arch_entries.is_empty() {
        write(
            d,
            "ARCHITECTURE.md",
            &format!("## Constraints\n\n{}", arch_entries.join("\n")),
        );
    }
    write(d, "CAPABILITIES.md", &capabilities(caps));
    for (rel, text) in extra {
        write(d, rel, text);
    }
    let (graph, violations) = clm_model::load(d);
    assert!(violations.is_empty(), "{violations:?}");
    (dir, graph)
}

/// The scenarios' repository: `capability/b` cited by nothing, `capability/u` untested,
/// `architecture/e` enforced by `a/x`, and `architecture/f` holding only `#### Untested`.
fn scenario() -> (TempDir, Graph) {
    graph(
        &[arch("e", &["a/x"], false), arch("f", &[], true)],
        &[("b", false), ("u", true)],
        &[],
    )
}

fn marker_on_x(graph: &Graph) -> clm_check::Marker {
    clm_check::Marker {
        id: "a/x".into(),
        version: Some(graph.requirement("a/x").unwrap().version().into()),
        path: "tests/t.rs".into(),
        line: 1,
    }
}

fn counts(view: &clm_view::SpecView, id: &str) -> (usize, usize, usize, usize) {
    let n = view.nodes.iter().find(|n| n.node == id).unwrap();
    (n.current, n.behind, n.unbuilt, n.total)
}

#[test]
fn spec_view_counts_cited_capabilities_on_the_root_only() {
    // req:view/spec-view@380d5f8a
    let (_dir, graph) = scenario();
    let view = clm_view::spec_view(&graph, &[marker_on_x(&graph)], None);
    let ids: Vec<&str> = view.nodes.iter().map(|n| n.node.as_str()).collect();
    assert_eq!(ids, ["clm", "a"]);
    assert_eq!(counts(&view, "clm"), (0, 0, 1, 1));
    assert_eq!(counts(&view, "a"), (1, 0, 0, 1));
}

#[test]
fn spec_view_counts_capabilities_on_the_capability_node() {
    // req:view/spec-view@380d5f8a
    let (_dir, graph) = graph(
        &[arch("e", &["a/x"], false)],
        &[("b", false), ("u", true)],
        &[(
            "specs/capability/SPEC.md",
            ROOT.replace("title: R", "title: C"),
        )],
    );
    let view = clm_view::spec_view(&graph, &[], None);
    assert_eq!(counts(&view, "clm"), (0, 0, 0, 0));
    assert_eq!(counts(&view, "capability"), (0, 0, 1, 1));
    assert_eq!(counts(&view, "a"), (0, 0, 1, 1));
}

fn summary(graph: &Graph, markers: &[clm_check::Marker]) -> String {
    let text = clm_view::status_text(graph, markers, &[], &[], 0, None, None);
    text.lines()
        .find(|l| l.starts_with("  components "))
        .unwrap()
        .to_string()
}

#[test]
fn summary_counts_entries() {
    // req:view/status-report@538240fb
    let (_dir, graph) = scenario();
    let line = summary(&graph, &[marker_on_x(&graph)]);
    assert!(
        line.ends_with("; 1/2 requirements current, architecture 1/1 enforced, 2 untested"),
        "{line}"
    );
    // `a/x` uncited: `architecture/e` is no longer enforced.
    let line = summary(&graph, &[]);
    assert!(
        line.ends_with("; 0/2 requirements current, architecture 0/1 enforced, 2 untested"),
        "{line}"
    );
}

#[test]
fn summary_entry_with_both_blocks_counts_twice() {
    // req:view/status-report@538240fb
    let (_dir, graph) = graph(&[arch("e", &["a/x"], true)], &[("b", false)], &[]);
    let line = summary(&graph, &[marker_on_x(&graph)]);
    assert!(
        line.ends_with("; 1/2 requirements current, architecture 1/1 enforced, 1 untested"),
        "{line}"
    );
}

#[test]
fn summary_without_architecture_or_untested() {
    // req:view/status-report@538240fb
    let (_dir, graph) = graph(&[], &[("b", false)], &[]);
    let line = summary(&graph, &[marker_on_x(&graph)]);
    assert!(line.ends_with("; 1/2 requirements current"), "{line}");
    // Entries, none holding `#### Enforced by`: the part still shows, as 0/0.
    let (_dir, graph) = graph_untested_only();
    let line = summary(&graph, &[marker_on_x(&graph)]);
    assert!(
        line.ends_with("; 1/1 requirements current, architecture 0/0 enforced, 1 untested"),
        "{line}"
    );
}

fn graph_untested_only() -> (TempDir, Graph) {
    graph(&[arch("f", &[], true)], &[], &[])
}
