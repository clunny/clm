//! The human layers' entries in coverage, `pending` and `clm status` (`human-layer-proof`):
//! a capability is cited like a rule and pending on `capability` or else the root; an
//! architecture entry stands by the rules it names; an untested entry is never pending.

mod common;

use clm_coverage::{Proof, State, pending, report, status_report};
use common::{ROOT, current, graph, marker, spec, write};
use tempfile::TempDir;

/// An architecture entry `slug` with an `#### Enforced by` block naming `rules` (none: no
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

fn architecture(entries: &[String]) -> String {
    format!("## Constraints\n\n{}", entries.join("\n"))
}

/// Capability entries, `(slug, untested)`.
fn capabilities(entries: &[(&str, bool)]) -> String {
    let entries: Vec<String> = entries
        .iter()
        .map(|(s, untested)| {
            let mut e = format!(
                "### Capability: {s}\nOne can {s}.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n"
            );
            if *untested {
                e.push_str("\n#### Untested\nNo program sees it.\n");
            }
            e
        })
        .collect();
    format!("## Capabilities\n\n{}", entries.join("\n"))
}

/// Root, node `n` with rules `x`, `y` and `z`, the given architecture entries, and
/// capabilities `b`, `c` and `d` (untested).
fn repo(arch_entries: &[String]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(
        d,
        "specs/n/SPEC.md",
        &spec("[\"src/**\"]", &["x", "y", "z"]),
    );
    write(d, "ARCHITECTURE.md", &architecture(arch_entries));
    write(
        d,
        "CAPABILITIES.md",
        &capabilities(&[("b", false), ("c", false), ("d", true)]),
    );
    dir
}

fn line(graph: &clm_model::Graph, markers: &[clm_coverage::Marker], id: &str) -> String {
    report(graph, markers)
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("no line for {id}"))
        .to_string()
}

#[test]
fn capabilities_cited_like_rules() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = repo(&[arch("a", &["n/x"], false)]);
    let graph = graph(dir.path());
    let markers = [
        marker("capability/b", Some("01234567"), "tests/t.rs", 2),
        current(&graph, "n/x", "tests/t.rs", 3),
    ];
    let lines: Vec<(String, State)> = report(&graph, &markers)
        .into_iter()
        .map(|c| (c.id, c.state))
        .collect();
    let ids: Vec<&str> = lines.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "architecture/a",
            "capability/b",
            "capability/c",
            "capability/d",
            "n/x",
            "n/y",
            "n/z"
        ]
    );
    assert_eq!(
        line(&graph, &markers, "capability/b"),
        "capability/b behind tests/t.rs:2"
    );
    assert_eq!(
        line(&graph, &markers, "capability/c"),
        "capability/c unbuilt"
    );
    let c = report(&graph, &markers);
    let version = graph.requirement("capability/c").unwrap().version();
    assert_eq!(c[2].version.as_deref(), Some(version));
}

#[test]
fn architecture_entries() {
    // req:check.coverage/coverage-report@1fec41ce
    let dir = repo(&[
        arch("a", &["n/x", "n/y"], false),
        arch("b", &["n/x", "n/z"], false),
        arch("c", &[], true),
        arch("e", &[], false),
        arch("f", &["m/q", "n/x", "m/p"], true),
    ]);
    write(
        dir.path(),
        "specs/m/SPEC.md",
        &spec("[\"m/**\"]", &["p", "q"]),
    );
    let graph = graph(dir.path());
    // A marker citing an entry proves nothing: `check/dangling-markers` reports it.
    let markers = [
        current(&graph, "n/x", "tests/t.rs", 1),
        current(&graph, "n/y", "tests/t.rs", 2),
        current(&graph, "architecture/c", "tests/t.rs", 3),
        current(&graph, "capability/d", "tests/t.rs", 4),
    ];
    assert_eq!(
        line(&graph, &markers, "architecture/a"),
        "architecture/a current"
    );
    assert_eq!(
        line(&graph, &markers, "architecture/b"),
        "architecture/b pending n/z"
    );
    assert_eq!(
        line(&graph, &markers, "architecture/c"),
        "architecture/c untested"
    );
    assert_eq!(
        line(&graph, &markers, "architecture/e"),
        "architecture/e unenforced"
    );
    // Both blocks: its rules decide, named in block order.
    assert_eq!(
        line(&graph, &markers, "architecture/f"),
        "architecture/f pending m/q,m/p"
    );
    assert_eq!(
        line(&graph, &markers, "capability/d"),
        "capability/d untested"
    );

    let c = report(&graph, &markers);
    let get = |id: &str| c.iter().find(|c| c.id == id).unwrap();
    assert_eq!(get("architecture/a").proof, Some(Proof::Enforced));
    assert_eq!(get("architecture/a").state, State::Current);
    assert_eq!(
        get("architecture/b").proof,
        Some(Proof::Pending(vec!["n/z".into()]))
    );
    assert_eq!(get("architecture/c").proof, Some(Proof::Untested));
    assert_eq!(get("architecture/e").proof, Some(Proof::Unenforced));
    assert_eq!(get("capability/d").proof, Some(Proof::Untested));
    assert_eq!(get("capability/b").proof, None);
    assert!(get("architecture/c").markers.is_empty());
}

#[test]
fn entries_that_are_never_pending() {
    // req:check.coverage/pending-report@0ca7a745
    let dir = repo(&[arch("a", &["n/x"], false), arch("u", &[], true)]);
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [architecture/a, capability/d]\n---\n",
    );
    let graph = graph(dir.path());
    let markers = [
        current(&graph, "n/y", "tests/t.rs", 1),
        current(&graph, "n/z", "tests/t.rs", 2),
    ];
    let found: Vec<(String, String)> = pending(&graph, &markers)
        .into_iter()
        .map(|p| (p.node, p.id))
        .collect();
    assert_eq!(
        found,
        [
            ("clm".into(), "capability/b".into()),
            ("clm".into(), "capability/c".into()),
            ("n".into(), "n/x".into()),
        ]
    );
    let x = &pending(&graph, &markers)[2];
    let version = graph.requirement("n/x").unwrap().version();
    assert_eq!(x.to_string(), format!("n/x {version} unbuilt"));
}

#[test]
fn entry_blocked_by_item() {
    // req:check.coverage/pending-report@0ca7a745
    let dir = repo(&[arch("a", &["n/x"], false)]);
    write(
        dir.path(),
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [capability/c]\n---\n",
    );
    let graph = graph(dir.path());
    let c = pending(&graph, &[])
        .into_iter()
        .find(|p| p.id == "capability/c")
        .unwrap();
    assert_eq!(c.node, "clm");
    assert_eq!(c.blocked_by, ["q"]);
    let version = graph.requirement("capability/c").unwrap().version();
    assert_eq!(
        c.to_string(),
        format!("capability/c {version} unbuilt blocked backlog/q")
    );
}

/// The `capability` node: frontmatter, a Contract and a Design, no requirements.
const CAPABILITY_NODE: &str = "---\ntitle: Capabilities\nstatus: active\nowners: []\npaths: [\"tests/capabilities/**\"]\n---\n\n## Contract\nThe capabilities' tests.\n\n## Design\nThe harness.\n";

#[test]
fn capabilities_pending_on_capability_node() {
    // req:check.coverage/pending-report@0ca7a745
    let dir = repo(&[arch("a", &["n/x"], false)]);
    write(dir.path(), "specs/capability/SPEC.md", CAPABILITY_NODE);
    let graph = graph(dir.path());
    assert!(graph.capability_node().is_some());
    let markers = [current(&graph, "capability/b", "tests/t.rs", 1)];
    let list = pending(&graph, &markers);
    let on = |node: &str| -> Vec<String> {
        list.iter()
            .filter(|p| p.node == node)
            .map(|p| p.id.clone())
            .collect()
    };
    assert_eq!(on("capability"), ["capability/c"]);
    assert!(on("clm").is_empty(), "{list:?}");
    let version = graph.requirement("capability/c").unwrap().version();
    let c = list.iter().find(|p| p.id == "capability/c").unwrap();
    assert_eq!(c.to_string(), format!("capability/c {version} unbuilt"));
}

#[test]
fn entries_on_the_roots_row() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/n/SPEC.md", &spec("[\"src/**\"]", &["x"]));
    write(
        d,
        "ARCHITECTURE.md",
        &architecture(&[arch("a", &["n/x"], false)]),
    );
    write(
        d,
        "CAPABILITIES.md",
        &capabilities(&[("b", false), ("c", false), ("u", true)]),
    );
    let graph = graph(d);
    let markers = [
        current(&graph, "n/x", "tests/t.rs", 1),
        current(&graph, "capability/c", "tests/t.rs", 2),
        current(&graph, "capability/u", "tests/t.rs", 3),
    ];
    let report = status_report(&graph, &markers);
    let row = |id: &str| report.nodes.iter().find(|r| r[0] == id).unwrap()[2].clone();
    assert_eq!(row("clm"), "1/2");
    assert_eq!(row("n"), "1/1");
}

#[test]
fn capabilities_on_the_capability_nodes_row() {
    // req:check.coverage/status-report@47ccc387
    let dir = repo(&[arch("a", &["n/x"], false)]);
    write(dir.path(), "specs/capability/SPEC.md", CAPABILITY_NODE);
    let graph = graph(dir.path());
    let markers = [current(&graph, "capability/b", "tests/t.rs", 1)];
    let report = status_report(&graph, &markers);
    let row = |id: &str| report.nodes.iter().find(|r| r[0] == id).unwrap()[2].clone();
    assert_eq!(row("capability"), "1/2");
    assert_eq!(row("clm"), "0/0");
    assert_eq!(row("n"), "0/3");
}
