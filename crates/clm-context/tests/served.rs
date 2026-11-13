//! The capabilities a node serves, in its bundle.

use std::path::PathBuf;

use clm_model::{Files, Graph, load_files};

const ARCHITECTURE: &str =
    "## Constraints\n\n### Constraint: x\nIt SHALL x.\n\n#### Scenario: s\n- ok\n";

/// Capability `slug` whose statement is `<SLUG>-TEXT`.
fn capability(slug: &str) -> String {
    format!(
        "### Capability: {slug}\n{}-TEXT\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n",
        slug.to_uppercase()
    )
}

fn capabilities(slugs: &[&str]) -> String {
    let entries: Vec<String> = slugs.iter().map(|s| capability(s)).collect();
    format!("## Capabilities\n\n{}", entries.join("\n"))
}

/// Root `clm`, both layers, and node `n` with `serves` and requirement blocks.
fn graph(capabilities: &str, serves: &str, requirements: &str) -> Graph {
    let root = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";
    let n = format!(
        "---\ntitle: N\nstatus: active\nowners: []\nserves: [{serves}]\n---\n\n\
         ## Contract\nc\n\n## Requirements\n\n{requirements}"
    );
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), root.into());
    files.insert(PathBuf::from("ARCHITECTURE.md"), ARCHITECTURE.into());
    files.insert(PathBuf::from("CAPABILITIES.md"), capabilities.into());
    files.insert(PathBuf::from("specs/n/SPEC.md"), n.into());
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

/// A rule that serves its node's `serves`.
const PLAIN: &str = "### Requirement: p\nIt SHALL p.\n\n#### Scenario: s\n- ok\n";

/// A rule whose own block serves `capability/b`.
const SERVES_B: &str =
    "### Requirement: r\nIt SHALL r.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- capability/b\n";

fn bundle(graph: &Graph) -> String {
    clm_context::bundle_from(graph, Some(ARCHITECTURE), "n").unwrap()
}

#[test]
fn bundle_holds_served_capabilities() {
    // req:context/bundle-contents@88b3e0e8
    // `serves` names `a` and an architecture ID; a rule's `#### Serves` block names `b`.
    let g = graph(
        &capabilities(&["a", "b", "c"]),
        "capability/a, architecture/x",
        &format!("{PLAIN}\n{SERVES_B}"),
    );
    let out = bundle(&g);
    let served = out.find("# Capabilities served").expect(&out);
    let (arch, spine) = (
        out.find("# ARCHITECTURE.md").unwrap(),
        out.find("# Ancestors").unwrap(),
    );
    let (a, b) = (out.find("A-TEXT").unwrap(), out.find("B-TEXT").unwrap());
    assert!(arch < served && served < a && a < b && b < spine, "{out}");
    assert!(out.contains(capability("a").trim_end()), "{out}");
    assert!(!out.contains("C-TEXT"), "{out}");
    assert_eq!(out.matches("### Capability: a").count(), 1, "{out}");
    // `architecture/x` is in the bundle once, with the whole of `ARCHITECTURE.md`.
    assert_eq!(out.matches("It SHALL x.").count(), 1, "{out}");
}

#[test]
fn overridden_node_default_adds_nothing() {
    // req:context/bundle-contents@88b3e0e8
    let g = graph(&capabilities(&["a", "b"]), "capability/a", SERVES_B);
    let out = bundle(&g);
    assert!(out.contains("B-TEXT"), "{out}");
    assert!(!out.contains("A-TEXT"), "{out}");
}

#[test]
fn served_capabilities_follow_file_order_and_appear_once() {
    // req:context/bundle-contents@88b3e0e8
    let g = graph(
        &capabilities(&["a", "b", "c"]),
        "capability/b, capability/a",
        &format!("{PLAIN}\n{SERVES_B}"),
    );
    let out = bundle(&g);
    assert!(
        out.find("A-TEXT").unwrap() < out.find("B-TEXT").unwrap(),
        "{out}"
    );
    assert_eq!(out.matches("B-TEXT").count(), 1, "{out}");
}

#[test]
fn no_served_section_when_none() {
    // req:context/bundle-contents@88b3e0e8
    let g = graph(&capabilities(&["a"]), "architecture/x", PLAIN);
    let out = bundle(&g);
    assert!(!out.contains("# Capabilities served"), "{out}");
    assert!(!out.contains("A-TEXT"), "{out}");
}

/// Asserts `out` holds `a`'s and `b`'s blocks once each, in that order, under
/// `# Capabilities served` and before the spine.
fn holds_every_capability(out: &str) {
    let served = out.find("# Capabilities served").expect(out);
    let (a, b) = (
        out.find("A-TEXT").expect(out),
        out.find("B-TEXT").expect(out),
    );
    assert!(
        served < a && a < b && b < out.find("# Ancestors").unwrap(),
        "{out}"
    );
    assert_eq!(out.matches("A-TEXT").count(), 1, "{out}");
    assert_eq!(out.matches("B-TEXT").count(), 1, "{out}");
}

#[test]
fn capability_node_gets_every_capability() {
    // req:context/bundle-contents@88b3e0e8
    let g = root_graph(true);
    holds_every_capability(
        &clm_context::bundle_from(&g, Some(ARCHITECTURE), "capability").unwrap(),
    );
}

/// Root `clm` with `serves: [capability/a]` and one plain rule, capabilities `a` and `b`,
/// and the capability node when `with_capability_node`.
fn root_graph(with_capability_node: bool) -> Graph {
    let root = format!(
        "---\ntitle: Root\nstatus: active\nowners: []\nserves: [capability/a]\n---\n\n\
         ## Contract\nr\n\n## Requirements\n\n{PLAIN}"
    );
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), root.into());
    files.insert(PathBuf::from("ARCHITECTURE.md"), ARCHITECTURE.into());
    files.insert(
        PathBuf::from("CAPABILITIES.md"),
        capabilities(&["a", "b"]).into(),
    );
    if with_capability_node {
        let node = "---\ntitle: Capabilities\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";
        files.insert(PathBuf::from("specs/capability/SPEC.md"), node.into());
    }
    let (g, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    g
}

#[test]
fn root_without_capability_node_gets_every_capability() {
    // req:context/bundle-contents@88b3e0e8
    let g = root_graph(false);
    holds_every_capability(&clm_context::bundle_from(&g, Some(ARCHITECTURE), "clm").unwrap());
}

#[test]
fn root_beside_a_capability_node_gets_only_what_it_serves() {
    // req:context/bundle-contents@88b3e0e8
    let g = root_graph(true);
    let out = clm_context::bundle_from(&g, Some(ARCHITECTURE), "clm").unwrap();
    assert!(out.contains("A-TEXT"), "{out}");
    assert!(!out.contains("B-TEXT"), "{out}");
}

#[test]
fn hash_moves_with_a_served_capability() {
    // req:context/bundle-hash@7010972d
    let hash = |caps: &str| clm_context::hash(&bundle(&graph(caps, "capability/a", PLAIN)));
    let before = hash(&capabilities(&["a", "c"]));
    let a_edited = capabilities(&["a", "c"]).replace("A-TEXT", "A-TEXT, reworded");
    let c_edited = capabilities(&["a", "c"]).replace("C-TEXT", "C-TEXT, reworded");
    assert_ne!(hash(&a_edited), before);
    assert_eq!(hash(&c_edited), before);
}
