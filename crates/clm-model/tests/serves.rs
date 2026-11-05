//! What each rule serves: a node's `serves` and a requirement's `#### Serves` block.

use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Violation, content_hash, load_files};

const ARCHITECTURE: &str = "## Constraints\n\n### Constraint: single-source\nGit SHALL hold it.\n\n#### Scenario: s\n- ok\n";

const CAPABILITIES: &str = "## Capabilities\n\n### Capability: approve\nA reviewer can approve.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";

/// A node SPEC.md with `fm_extra` frontmatter lines and requirement blocks.
fn spec(fm_extra: &str, requirements: &[&str]) -> String {
    format!(
        "---\ntitle: T\nstatus: active\nowners: []{fm_extra}\n---\n\n## Contract\nc\n\n## Requirements\n\n{}",
        requirements.join("\n")
    )
}

/// Requirement `slug` with one scenario, then `extra` lines.
fn req(slug: &str, extra: &str) -> String {
    format!("### Requirement: {slug}\nIt SHALL.\n\n#### Scenario: s\n- ok\n{extra}")
}

/// Root `clm`, both layers, and node `a`.
fn load(a: &str) -> (Graph, Vec<Violation>) {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), spec("", &[]).into());
    files.insert(PathBuf::from("ARCHITECTURE.md"), ARCHITECTURE.into());
    files.insert(PathBuf::from("CAPABILITIES.md"), CAPABILITIES.into());
    files.insert(PathBuf::from("specs/a/SPEC.md"), a.into());
    load_files(&files)
}

/// The violations, each `model/serves` on `specs/a/SPEC.md`, one per name in `names`,
/// in order, with node `a` still loaded.
fn serves_violations(a: &str, names: &[&str]) {
    let (graph, violations) = load(a);
    assert_eq!(violations.len(), names.len(), "{violations:?}");
    for (v, name) in violations.iter().zip(names) {
        assert_eq!(v.rule, "model/serves", "{v:?}");
        assert_eq!(v.path, Path::new("specs/a/SPEC.md"), "{v:?}");
        assert!(v.message.contains(name), "{name}: {v:?}");
    }
    assert!(graph.nodes.contains_key("a"));
}

#[test]
fn serves_block_overrides() {
    // req:model/serves@3537f5b4
    let x = req("x", "\n#### Serves\n- architecture/single-source\n");
    let a = spec("\nserves: [capability/approve]", &[&x, &req("y", "")]);
    let (graph, violations) = load(&a);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.nodes["a"].frontmatter.serves, ["capability/approve"]);
    assert_eq!(
        graph.requirement("a/x").unwrap().serves,
        ["architecture/single-source"]
    );
    assert_eq!(graph.effective_serves("a/x"), ["architecture/single-source"]);
    assert_eq!(graph.effective_serves("a/y"), ["capability/approve"]);
    assert!(graph.effective_serves("capability/approve").is_empty());
    assert!(graph.effective_serves("a/none").is_empty());
    assert!(graph.effective_serves("clm/none").is_empty());
}

#[test]
fn serves_block_in_a_code_fence_is_not_a_block() {
    // req:model/serves@3537f5b4
    let x = req("x", "\n```markdown\n#### Serves\n- land/approve\n```\n");
    let (graph, violations) = load(&spec("", &[&x]));
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.effective_serves("a/x").is_empty());
}

#[test]
fn serves_unknown_entry_rejected() {
    // req:model/serves@3537f5b4
    serves_violations(
        &spec("\nserves: [capability/nope]", &[&req("x", "")]),
        &["`capability/nope`"],
    );
    let x = req("x", "\n#### Serves\n- architecture/nope\n");
    serves_violations(&spec("", &[&x]), &["`a/x` serves `architecture/nope`"]);
}

#[test]
fn serves_not_a_layer_id_rejected() {
    // req:model/serves@3537f5b4
    serves_violations(&spec("\nserves: [land]", &[&req("x", "")]), &["`land`"]);
    let x = req("x", "\n#### Serves\n- land/approve\n");
    serves_violations(&spec("", &[&x]), &["`land/approve`"]);
    for line in [
        "capability/approve",
        "- capability/a/b",
        "- `capability/approve`",
    ] {
        let x = req("x", &format!("\n#### Serves\n{line}\n"));
        serves_violations(&spec("", &[&x]), &["`a/x`"]);
    }
}

#[test]
fn second_serves_block_rejected() {
    // req:model/serves@3537f5b4
    let block = "\n#### Serves\n- capability/approve\n";
    let x = req("x", &format!("{block}{block}"));
    serves_violations(&spec("", &[&x]), &["`a/x`"]);
}

#[test]
fn empty_serves_block_rejected() {
    // req:model/serves@3537f5b4
    let x = req("x", "\n#### Serves\n\n#### Scenario: t\n- ok\n");
    serves_violations(&spec("", &[&x]), &["`a/x`"]);
}

#[test]
fn serves_block_outside_requirement_rejected() {
    // req:model/serves@3537f5b4
    let a = format!(
        "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n\n#### Serves\n- capability/approve\n\n## Requirements\n\n{}",
        req("x", "")
    );
    serves_violations(&a, &["outside"]);
}

#[test]
fn legacy_capabilities_is_ignored() {
    // req:model/valid-frontmatter@b4618360
    let (graph, violations) = load(&spec("\ncapabilities: [x, y]", &[&req("x", "")]));
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.nodes["a"].frontmatter.serves.is_empty());
}

#[test]
fn graph_check_reports_serves_on_an_edited_graph() {
    // req:model/graph-check@e0dbe7bc
    let (mut graph, violations) = load(&spec("\nserves: [capability/approve]", &[&req("x", "")]));
    assert!(violations.is_empty(), "{violations:?}");
    graph.layers.clear();
    let found = clm_model::check_graph(&graph);
    let rules: Vec<&str> = found.iter().map(|v| v.rule).collect();
    assert_eq!(rules, ["model/serves"], "{found:?}");
}

#[test]
fn serves_block_leaves_version() {
    // req:model/content-hash@92d37469
    // req:model/serves@3537f5b4
    let plain = load(&spec("", &[&req("x", "")])).0;
    let x = req("x", "\n#### Serves\n- capability/approve\n");
    let served = load(&spec("", &[&x])).0;
    let (plain, served) = (
        plain.requirement("a/x").unwrap(),
        served.requirement("a/x").unwrap(),
    );
    // Same version, different hash; without the block the hash is the block's own.
    assert_eq!(plain.version(), served.version());
    assert_ne!(plain.hash, served.hash);
    assert_eq!(plain.hash, content_hash(&plain.text));
    assert_eq!(served.hash[8..], content_hash(&served.text)[8..]);
    assert!(
        served.text.contains("#### Serves\n- capability/approve"),
        "{}",
        served.text
    );
}

#[test]
fn serves_block_between_scenarios_leaves_version() {
    // req:model/content-hash@92d37469
    let tail = "\n#### Scenario: t\n- ok\n";
    let plain = load(&spec("", &[&req("x", tail)])).0;
    let x = req("x", &format!("\n#### Serves\n- capability/approve\n{tail}"));
    let served = load(&spec("", &[&x])).0;
    let (plain, served) = (
        plain.requirement("a/x").unwrap(),
        served.requirement("a/x").unwrap(),
    );
    assert_eq!(plain.version(), served.version());
    assert_ne!(plain.hash, served.hash);
    assert_eq!(served.hash[8..], content_hash(&served.text)[8..]);
}

#[test]
fn loaded_hash_ignores_line_endings_and_trailing_whitespace() {
    // req:model/content-hash@92d37469
    let x = req("x", "\n#### Serves\n- capability/approve\n");
    let lf = load(&spec("", &[&x])).0;
    let crlf = load(&spec("", &[&x.replace('\n', " \t\n")]).replace('\n', "\r\n")).0;
    let edited = load(&spec("", &[&x.replace("It SHALL.", "It SHALL NOT.")])).0;
    let hash = |g: &Graph| g.requirement("a/x").unwrap().hash.clone();
    assert_eq!(hash(&lf), hash(&crlf));
    assert_ne!(hash(&lf)[..8], hash(&edited)[..8]);
    assert_ne!(hash(&lf)[8..], hash(&edited)[8..]);
}

#[test]
fn serves_only_edit_moves_hash_not_version() {
    // req:model/content-hash@92d37469
    // req:model/serves@3537f5b4
    let one = req("x", "\n#### Serves\n- capability/approve\n");
    let two = req("x", "\n#### Serves\n- architecture/single-source\n");
    let (one, two) = (load(&spec("", &[&one])).0, load(&spec("", &[&two])).0);
    let (one, two) = (
        one.requirement("a/x").unwrap(),
        two.requirement("a/x").unwrap(),
    );
    assert_eq!(one.version(), two.version());
    assert_ne!(one.hash, two.hash);
}

#[test]
fn node_parser_reports_serves_blocks() {
    // req:model/graph-check@e0dbe7bc
    let x = req("x", "\n#### Serves\n\n#### Scenario: t\n- ok\n");
    let path = Path::new("specs/a/SPEC.md");
    let (node, violations) =
        clm_model::parse_node("a", path, &spec("", &[&x]), &Default::default());
    assert!(node.is_some());
    let rules: Vec<&str> = violations.iter().map(|v| v.rule).collect();
    assert_eq!(rules, ["model/serves"], "{violations:?}");
}
