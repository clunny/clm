//! The node queue without git: `queue_from_refs` on a graph and markers in memory.

mod common;

use clm_coverage::Marker;
use clm_model::{Graph, Priority};
use clm_orchestrator::{ClaimRef, Queue, State, queue_from_refs};
use common::*;

const ROOT: (&str, &str) = (
    "specs/SPEC.md",
    "---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
);

/// A marker citing `id` at `version` (`None` for a bare one) at `t.rs:<line>`.
fn marker(id: &str, version: Option<&str>, line: usize) -> Marker {
    Marker {
        id: id.into(),
        version: version.map(String::from),
        path: std::path::PathBuf::from("t.rs"),
        line,
    }
}

/// A marker citing `id` at its current version in `graph`.
fn current(graph: &Graph, id: &str) -> Marker {
    marker(id, Some(graph.requirement(id).unwrap().version()), 1)
}

fn queue(graph: &Graph, markers: &[Marker], refs: &[ClaimRef]) -> Queue {
    queue_from_refs(graph, markers, refs, 1_000_000, 4)
}

fn state<'a>(q: &'a Queue, node: &str) -> &'a State {
    &q.nodes
        .iter()
        .find(|n| n.node == node)
        .unwrap_or_else(|| panic!("no node {node} in {q:?}"))
        .state
}

fn priority(q: &Queue, node: &str) -> Priority {
    q.nodes.iter().find(|n| n.node == node).unwrap().priority
}

fn claim(node: &str, hours_ago: u64) -> ClaimRef {
    ClaimRef {
        node: node.into(),
        tip_time: 1_000_000 - hours_ago * 3600,
        merged: false,
    }
}

/// The graph of the *same as next* scenario: `a` depends on `b`, `c` on `d`, plus `e` and
/// `f`; `backlog/h.md` blocks `f/x`.
fn same_as_next() -> Graph {
    graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("b", &["x"])),
        ("specs/b/SPEC.md", &node("", &["x"])),
        ("specs/c/SPEC.md", &node("d", &["x"])),
        ("specs/d/SPEC.md", &node("", &["x"])),
        ("specs/e/SPEC.md", &node("", &["x"])),
        ("specs/f/SPEC.md", &node("", &["x"])),
        (
            "backlog/h.md",
            "---\ntitle: H\nkind: human-task\nblocks: [f/x]\n---\n",
        ),
    ])
}

#[test]
fn scenario_same_as_next() {
    // req:orchestrator/queue-from-refs@cbcfd325 req:orchestrator/next@fd7e1025
    let g = same_as_next();
    let q = queue(&g, &[current(&g, "b/x")], &[claim("e", 1)]);
    assert_eq!(state(&q, "b"), &State::Converged);
    assert!(matches!(state(&q, "a"), State::Ready { .. }));
    assert!(matches!(state(&q, "d"), State::Ready { .. }));
    assert!(matches!(state(&q, "c"), State::Ready { .. }), "{q:?}");
    assert_eq!(
        state(&q, "e"),
        &State::Claimed {
            lapses_at: 1_000_000 + 3 * 3600
        }
    );
    match state(&q, "f") {
        State::Blocked { by } => {
            assert_eq!(by.len(), 1);
            assert_eq!(by[0].item.as_deref(), Some("backlog/h"));
            assert_eq!(by[0].requirement, None);
            assert_eq!(by[0].requirements, ["f/x"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(q.next(), ["a", "c", "d"]);
}

#[test]
fn ready_carries_its_pending_requirements() {
    // req:orchestrator/queue-pending-entries@78481951
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["x", "y", "z"])),
        (
            "changes/c/change.md",
            "---\nstatus: applied\n---\n## REMOVED Requirements\n",
        ),
        (
            "changes/c/deltas/a.md",
            "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0000000000000000 -->\n",
        ),
    ]);
    let markers = [
        current(&g, "a/x"),
        marker("a/y", Some("11111111"), 3),
        marker("a/y", None, 4),
        marker("a/gone", None, 9),
    ];
    let q = queue(&g, &markers, &[]);
    let State::Ready { pending } = state(&q, "a") else {
        panic!("{q:?}");
    };
    let ids: Vec<&str> = pending.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["a/gone", "a/y", "a/z"]);
    assert_eq!(pending[0].state, "obsolete");
    assert_eq!(pending[0].markers, ["t.rs:9"]);
    assert_eq!(pending[1].state, "behind");
    assert_eq!(pending[1].cited, ["11111111", "none"]);
    assert_eq!(
        pending[1].version.as_deref(),
        Some(g.requirement("a/y").unwrap().version())
    );
    assert_eq!(pending[2].state, "unbuilt");
}

/// Nodes `a` with requirements `p` and `q`, and an `applied` change `x` whose delta adds
/// `a/p` and removes `a/q`, at `status`.
fn change_x(status: &str) -> Graph {
    graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["p"])),
        ("changes/x/change.md", &change(status)),
        (
            "changes/x/deltas/a.md",
            "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: p\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n## REMOVED Requirements\n\n### Requirement: q <!-- base: 0000000000000000 -->\n",
        ),
    ])
}

#[test]
fn scenario_complete_change() {
    // req:orchestrator/queue-complete-changes@334fce17
    let g = change_x("applied");
    let q = queue(&g, &[current(&g, "a/p")], &[]);
    assert_eq!(q.complete, ["x"]);
    assert_eq!(state(&q, "a"), &State::Converged);
}

#[test]
fn scenario_incomplete_change() {
    // req:orchestrator/queue-complete-changes@334fce17
    let g = change_x("applied");
    let q = queue(&g, &[current(&g, "a/p"), marker("a/q", None, 5)], &[]);
    assert_eq!(q.complete, Vec::<String>::new());
    let State::Ready { pending } = state(&q, "a") else {
        panic!("{q:?}");
    };
    assert_eq!(pending.len(), 1);
    assert_eq!(
        (pending[0].id.as_str(), pending[0].state.as_str()),
        ("a/q", "obsolete")
    );
}

#[test]
fn scenario_unapplied_change() {
    // req:orchestrator/queue-complete-changes@334fce17
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["p"])),
        ("changes/y/change.md", &change("proposed")),
        (
            "changes/y/deltas/a.md",
            &format!(
                "---\ncomponent: a\n---\n## MODIFIED Requirements\n\n### Requirement: p <!-- base: {} -->\nIt SHALL more.\n\n#### Scenario: s\n- ok\n",
                graph(&[ROOT, ("specs/a/SPEC.md", &node("", &["p"]))])
                    .requirement("a/p")
                    .unwrap()
                    .hash
            ),
        ),
    ]);
    let q = queue(&g, &[current(&g, "a/p")], &[]);
    assert_eq!(q.complete, Vec::<String>::new());
}

#[test]
fn a_change_whose_addition_a_later_change_removed_completes() {
    // req:orchestrator/queue-complete-changes@334fce17
    // Completeness is `check/status-report`'s: a requirement the spec no longer holds
    // doesn't keep the change that added it from completing.
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["q"])),
        ("changes/x/change.md", &change("applied")),
        (
            "changes/x/deltas/a.md",
            "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: p\nIt SHALL.\n\n#### Scenario: s\n- ok\n",
        ),
        ("changes/z/change.md", &change("applied")),
        (
            "changes/z/deltas/a.md",
            "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: p <!-- base: 0000000000000000 -->\n",
        ),
    ]);
    let q = queue(&g, &[current(&g, "a/q")], &[]);
    assert_eq!(q.complete, ["x", "z"]);
}

#[test]
fn lapsed_and_merged_claims_are_released() {
    // req:orchestrator/next@fd7e1025 req:orchestrator/queue-from-refs@cbcfd325
    let g = graph(&[
        ROOT,
        ("specs/b/SPEC.md", &node("", &["x"])),
        ("specs/d/SPEC.md", &node("", &["x"])),
        ("specs/m/SPEC.md", &node("", &["x"])),
    ]);
    let mut merged = claim("m", 1);
    merged.merged = true;
    let q = queue(&g, &[], &[claim("b", 1), claim("d", 5), merged]);
    assert_eq!(q.next(), ["d", "m"]);
    assert!(matches!(state(&q, "b"), State::Claimed { .. }));
}

#[test]
fn nested_refs_claim_no_node() {
    // req:orchestrator/queue-from-refs@cbcfd325 req:orchestrator/nested-branches-not-claims@a7ec57b2
    let g = graph(&[ROOT, ("specs/b/SPEC.md", &node("", &["x"]))]);
    let q = queue(&g, &[], &[claim("x/b", 1), claim("b/x", 1)]);
    assert_eq!(q.next(), ["b"]);
}

#[test]
fn scenario_archive_branch_in_the_queue() {
    // req:orchestrator/nested-branches-not-claims@a7ec57b2
    let g = graph(&[
        ROOT,
        ("specs/archive/SPEC.md", &node("", &["x"])),
        ("specs/b/SPEC.md", &node("", &["x"])),
    ]);
    let q = queue(&g, &[], &[claim("archive/20261026-x", 1), claim("b", 1)]);
    assert!(matches!(state(&q, "b"), State::Claimed { .. }));
    assert!(matches!(state(&q, "archive"), State::Ready { .. }));
}

#[test]
fn scenario_behind_requirement() {
    // req:orchestrator/next@fd7e1025
    let g = graph(&[ROOT, ("specs/b/SPEC.md", &node("", &["x"]))]);
    let q = queue(&g, &[marker("b/x", Some("11111111"), 1)], &[]);
    assert_eq!(q.next(), ["b"]);
}

#[test]
fn scenario_dependency_doesnt_order() {
    // req:orchestrator/next@fd7e1025
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("b", &["x"])),
        ("specs/b/SPEC.md", &node("", &["x"])),
    ]);
    assert_eq!(queue(&g, &[], &[]).next(), ["a", "b"]);
}

/// A live change `id` at `priority` (empty for none) adding `node/slug`.
fn adds(id: &str, priority: &str, node: &str, slug: &str) -> Vec<(String, String)> {
    let extra = if priority.is_empty() {
        String::new()
    } else {
        format!("priority: {priority}\n")
    };
    vec![
        (
            format!("changes/{id}/change.md"),
            change_md("applied", &extra),
        ),
        (
            format!("changes/{id}/deltas/{node}.md"),
            delta(node, &[slug]),
        ),
    ]
}

fn with_changes(nodes: &[(&str, &str)], changes: Vec<Vec<(String, String)>>) -> Graph {
    let mut files: Vec<(String, String)> = vec![(ROOT.0.into(), ROOT.1.into())];
    files.extend(nodes.iter().map(|(p, t)| (p.to_string(), t.to_string())));
    files.extend(changes.into_iter().flatten());
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    graph(&refs)
}

#[test]
fn scenario_higher_priority_first() {
    // req:orchestrator/next@fd7e1025 req:orchestrator/priority-orders-components@147ea0bf
    let g = with_changes(
        &[
            ("specs/a/SPEC.md", &node("", &["x"])),
            ("specs/b/SPEC.md", &node("", &["x"])),
        ],
        vec![
            adds("20260101-a", "", "a", "x"),
            adds("20260101-b", "P2", "b", "x"),
        ],
    );
    let q = queue(&g, &[], &[]);
    assert_eq!(q.next(), ["b", "a"]);
    assert_eq!(priority(&q, "b"), Priority::P2);
    assert_eq!(priority(&q, "a"), Priority::P5);
}

#[test]
fn scenario_urgent_work_pulls_the_node_it_needs_forward() {
    // req:orchestrator/priority-orders-components@147ea0bf
    let g = with_changes(
        &[
            ("specs/a/SPEC.md", &needing("", "b/y", &[])),
            ("specs/b/SPEC.md", &node("", &["y"])),
            ("specs/c/SPEC.md", &node("", &["x"])),
        ],
        vec![
            adds("20260101-a", "P1", "a", "x"),
            adds("20260101-b", "", "b", "y"),
            adds("20260101-c", "P3", "c", "x"),
        ],
    );
    let q = queue(&g, &[], &[]);
    assert_eq!(q.next(), ["b", "c"]);
    assert_eq!(priority(&q, "b"), Priority::P1);
}

#[test]
fn scenario_passes_through_a_claimed_node() {
    // req:orchestrator/priority-orders-components@147ea0bf
    let g = with_changes(
        &[
            ("specs/a/SPEC.md", &node("", &["x"])),
            (
                "specs/b/SPEC.md",
                &node_with("", "", &[("y", "It SHALL.\n\n#### Needs\n- a/x")]),
            ),
            (
                "specs/c/SPEC.md",
                &node_with("", "", &[("z", "It SHALL.\n\n#### Needs\n- b/y")]),
            ),
            ("specs/d/SPEC.md", &node("", &["x"])),
        ],
        vec![
            adds("20260101-a", "", "a", "x"),
            adds("20260101-c", "P1", "c", "z"),
            adds("20260101-d", "P3", "d", "x"),
        ],
    );
    let q = queue(&g, &[], &[claim("b", 1)]);
    assert_eq!(q.next(), ["a", "d"]);
    assert_eq!(priority(&q, "a"), Priority::P1);
    assert_eq!(priority(&q, "b"), Priority::P1);
}

#[test]
fn scenario_dependency_lends_no_priority() {
    // req:orchestrator/priority-orders-components@147ea0bf req:orchestrator/next@fd7e1025
    let g = with_changes(
        &[
            ("specs/a/SPEC.md", &node("b", &["x"])),
            ("specs/b/SPEC.md", &node("", &["x"])),
            ("specs/c/SPEC.md", &node("", &["x"])),
        ],
        vec![
            adds("20260101-a", "P1", "a", "x"),
            adds("20260101-b", "", "b", "x"),
            adds("20260101-c", "P3", "c", "x"),
        ],
    );
    let q = queue(&g, &[], &[]);
    assert_eq!(q.next(), ["a", "c", "b"]);
    assert_eq!(priority(&q, "b"), Priority::P5);
}

#[test]
fn scenario_capability_pulls_its_serving_rules_forward() {
    // req:orchestrator/priority-orders-components@147ea0bf
    let capability_node = "---\ntitle: Capabilities\nstatus: active\nowners: []\npaths: [\"tests/capabilities/**\"]\n---\n\n## Contract\nThe tests.\n\n## Design\nThe harness.\n";
    let capabilities =
        "## Capabilities\n\n### Capability: c\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
    let w = node_with("", "serves: [capability/c]\n", &[("x", "It SHALL.")]);
    let mut files: Vec<(String, String)> = vec![
        (ROOT.0.into(), ROOT.1.into()),
        ("specs/capability/SPEC.md".into(), capability_node.into()),
        ("CAPABILITIES.md".into(), capabilities.into()),
        ("ARCHITECTURE.md".into(), "## Requirements\n".into()),
        ("specs/w/SPEC.md".into(), w),
        ("specs/d/SPEC.md".into(), node("", &["x"])),
    ];
    files.extend(adds("20260101-c", "P1", "capability", "c"));
    files.extend(adds("20260101-d", "P3", "d", "x"));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let g = graph(&refs);
    let q = queue(&g, &[], &[]);
    assert_eq!(q.next(), ["w", "d"]);
    assert_eq!(priority(&q, "w"), Priority::P1);
    match state(&q, "capability") {
        State::Blocked { by } => assert_eq!(by[0].requirement.as_deref(), Some("w/x")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn scenario_capability_pulls_a_changed_serving_rules_needs_forward() {
    // req:orchestrator/priority-orders-components@147ea0bf
    let capability_node = "---\ntitle: Capabilities\nstatus: active\nowners: []\npaths: [\"tests/capabilities/**\"]\n---\n\n## Contract\nThe tests.\n\n## Design\nThe harness.\n";
    let capabilities =
        "## Capabilities\n\n### Capability: c\nOne can.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
    let w = node_with(
        "",
        "serves: [capability/c]\n",
        &[("x", "It SHALL.\n\n#### Needs\n- b/y")],
    );
    let mut files: Vec<(String, String)> = vec![
        (ROOT.0.into(), ROOT.1.into()),
        ("specs/capability/SPEC.md".into(), capability_node.into()),
        ("CAPABILITIES.md".into(), capabilities.into()),
        ("ARCHITECTURE.md".into(), "## Requirements\n".into()),
        ("specs/w/SPEC.md".into(), w),
        ("specs/b/SPEC.md".into(), node("", &["y"])),
        ("specs/d/SPEC.md".into(), node("", &["x"])),
    ];
    files.extend(adds("20260101-c", "P1", "capability", "c"));
    files.extend(adds("20260101-w", "", "w", "x"));
    files.extend(adds("20260101-b", "", "b", "y"));
    files.extend(adds("20260101-d", "P3", "d", "x"));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let g = graph(&refs);
    let markers = [
        marker("capability/c", Some("11111111"), 1),
        marker("w/x", Some("11111111"), 2),
    ];
    let q = queue(&g, &markers, &[]);
    assert_eq!(q.next(), ["b", "d"]);
    assert_eq!(priority(&q, "b"), Priority::P1);
    assert_eq!(priority(&q, "w"), Priority::P1);
}

#[test]
fn scenario_blocked_requirement_does_not_lend_its_priority() {
    // req:orchestrator/priority-orders-components@147ea0bf
    let mut files = vec![
        ("specs/b/SPEC.md".to_string(), node("", &["x", "y"])),
        ("specs/c/SPEC.md".to_string(), node("", &["x"])),
        (
            "backlog/q.md".to_string(),
            "---\ntitle: Q\nkind: human-task\nblocks: [b/x]\n---\n".to_string(),
        ),
    ];
    files.extend(adds("20260101-b", "P1", "b", "x"));
    files.extend(adds("20260101-c", "P3", "c", "x"));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let g = with_changes(&refs, vec![]);
    let q = queue(&g, &[], &[]);
    assert_eq!(q.next(), ["c", "b"]);
    assert_eq!(priority(&q, "b"), Priority::P5);
}

#[test]
fn priority_never_makes_a_node_ready() {
    // req:orchestrator/next@fd7e1025
    let g = with_changes(
        &[
            ("specs/a/SPEC.md", &needing("", "b/y", &[])),
            ("specs/b/SPEC.md", &node("", &["y"])),
        ],
        vec![
            adds("20260101-a", "P1", "a", "x"),
            adds("20260101-b", "", "b", "y"),
        ],
    );
    assert_eq!(queue(&g, &[], &[]).next(), ["b"]);
}

#[test]
fn a_node_that_writes_a_protected_path_is_ready() {
    // req:orchestrator/queue-from-refs@cbcfd325 req:orchestrator/next@fd7e1025
    let g = graph(&[
        ROOT,
        (
            "specs/w/SPEC.md",
            &node_with(
                "",
                "paths: [\"web/**\"]\n",
                &[("x", "It SHALL write `web/.npmrc`.")],
            ),
        ),
    ]);
    let q = queue(&g, &[], &[]);
    assert!(matches!(state(&q, "w"), State::Ready { .. }), "{q:?}");
    assert_eq!(q.next(), ["w"]);
}

#[test]
fn nodes_serialise_with_a_state_tag() {
    // req:orchestrator/queue-from-refs@cbcfd325
    let g = same_as_next();
    let q = queue(&g, &[current(&g, "b/x")], &[claim("e", 1)]);
    let json = serde_json::to_value(&q.nodes).unwrap();
    let find = |id: &str| {
        json.as_array()
            .unwrap()
            .iter()
            .find(|n| n["node"] == id)
            .unwrap()
            .clone()
    };
    assert_eq!(find("b")["state"], "converged");
    assert_eq!(find("c")["state"], "ready");
    assert!(find("c").get("on").is_none());
    assert_eq!(find("e")["lapses_at"], 1_000_000 + 3 * 3600);
    assert_eq!(find("f")["by"][0]["item"], "backlog/h");
    assert_eq!(find("a")["priority"], "P5");
    assert_eq!(find("a")["pending"][0]["id"], "a/x");
}

#[test]
fn priority_reaches_no_node_it_doesnt_need() {
    // req:orchestrator/priority-orders-components@147ea0bf
    // `a` (P1) depends on `b`, whose only pending requirement is blocked by an item, and on
    // `c`, which is converged: it needs neither, so neither is pulled forward.
    let mut files = vec![
        ("specs/a/SPEC.md".to_string(), node("b, c", &["x"])),
        ("specs/b/SPEC.md".to_string(), node("", &["y"])),
        ("specs/c/SPEC.md".to_string(), node("", &["z"])),
        (
            "backlog/q.md".to_string(),
            "---\ntitle: Q\nkind: human-task\nblocks: [b/y]\n---\n".to_string(),
        ),
    ];
    files.extend(adds("20260101-a", "P1", "a", "x"));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let g = with_changes(&refs, vec![]);
    let q = queue(&g, &[current(&g, "c/z")], &[]);
    assert_eq!(q.next(), ["a"]);
    assert_eq!(priority(&q, "b"), Priority::P5);
    assert_eq!(priority(&q, "c"), Priority::P5);
}

#[test]
fn a_need_blocked_requirements_path_does_not_count() {
    // req:orchestrator/queue-from-refs@cbcfd325
    let g = graph(&[
        ROOT,
        (
            "specs/w/SPEC.md",
            &node_with(
                "",
                "paths: [\"web/**\"]\n",
                &[
                    ("x", "It SHALL write `web/.npmrc`.\n\n#### Needs\n- b/y"),
                    ("z", "It SHALL."),
                ],
            ),
        ),
        ("specs/b/SPEC.md", &node("", &["y"])),
    ]);
    assert_eq!(queue(&g, &[], &[]).next(), ["b", "w"]);
}

#[test]
fn a_needed_requirement_blocked_by_an_item_still_blocks() {
    // req:orchestrator/queue-from-refs@cbcfd325
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &node("", &["y"])),
        (
            "backlog/q.md",
            "---\ntitle: Q\nkind: human-task\nblocks: [b/y]\n---\n",
        ),
    ]);
    let q = queue(&g, &[], &[]);
    match state(&q, "a") {
        State::Blocked { by } => assert_eq!(by[0].requirement.as_deref(), Some("b/y")),
        other => panic!("{other:?}"),
    }
    assert!(matches!(state(&q, "b"), State::Blocked { .. }));
    assert_eq!(q.next(), Vec::<String>::new());
}

/// A node depending on `deps` whose requirement `x` needs `needs`, plus requirements `more`.
fn needing(deps: &str, needs: &str, more: &[&str]) -> String {
    let x = format!("It SHALL.\n\n#### Needs\n- {needs}");
    let mut reqs = vec![("x", x.as_str())];
    reqs.extend(more.iter().map(|r| (*r, "It SHALL.")));
    node_with(deps, "", &reqs)
}

#[test]
fn scenario_blocked_by_a_need() {
    // req:orchestrator/queue-from-refs@cbcfd325
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &node("", &["y"])),
    ]);
    let q = queue(&g, &[], &[]);
    match state(&q, "a") {
        State::Blocked { by } => {
            assert_eq!(by.len(), 1);
            assert_eq!(by[0].item, None);
            assert_eq!(by[0].requirement.as_deref(), Some("b/y"));
            assert_eq!(by[0].components.as_deref(), Some(&["b".into()][..]));
            assert_eq!(by[0].requirements, ["a/x"]);
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(state(&q, "b"), State::Ready { .. }));
    let json = serde_json::to_value(&q.nodes).unwrap();
    let a = json
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "a")
        .unwrap();
    assert_eq!(a["by"][0]["requirement"], "b/y");
    assert_eq!(a["by"][0]["requirements"], serde_json::json!(["a/x"]));
    assert_eq!(a["by"][0]["components"], serde_json::json!(["b"]));
    assert!(a["by"][0].get("item").is_none(), "{a}");
}

#[test]
fn scenario_needed_requirement_first_then_current() {
    // req:orchestrator/next@fd7e1025
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &node("", &["y"])),
    ]);
    assert_eq!(queue(&g, &[], &[]).next(), ["b"]);
    assert_eq!(queue(&g, &[current(&g, "b/y")], &[]).next(), ["a"]);
}

#[test]
fn scenario_need_against_the_node_graph() {
    // req:orchestrator/next@fd7e1025
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &needing("", "b/y", &["w"])),
        ("specs/b/SPEC.md", &node("a", &["y"])),
    ]);
    let q = queue(&g, &[], &[]);
    assert_eq!(q.next(), ["a", "b"]);
    let State::Ready { pending } = state(&q, "a") else {
        panic!("{q:?}");
    };
    let x = pending.iter().find(|p| p.id == "a/x").unwrap();
    assert_eq!(x.waits_on, ["b/y"]);
    let json = serde_json::to_value(x).unwrap();
    assert_eq!(json["waits_on"], serde_json::json!(["b/y"]));
    assert_eq!(json["blocked_by"], serde_json::json!([]));
    assert_eq!(queue(&g, &[current(&g, "a/w")], &[]).next(), ["b"]);
}

#[test]
fn scenario_blocked_node_holds_nothing_up() {
    // req:orchestrator/next@fd7e1025
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &node("", &["y"])),
        ("specs/c/SPEC.md", &node("a", &["x"])),
    ]);
    assert_eq!(queue(&g, &[], &[]).next(), ["b", "c"]);
}

#[test]
fn scenario_need_pulls_priority() {
    // req:orchestrator/priority-orders-components@147ea0bf req:orchestrator/next@fd7e1025
    let g = with_changes(
        &[
            ("specs/a/SPEC.md", &needing("", "b/y", &[])),
            ("specs/b/SPEC.md", &node("", &["y"])),
            ("specs/c/SPEC.md", &node("", &["x"])),
        ],
        vec![
            adds("20260101-a", "P2", "a", "x"),
            adds("20260101-c", "P3", "c", "x"),
        ],
    );
    let q = queue(&g, &[], &[]);
    assert_eq!(priority(&q, "b"), Priority::P2);
    assert_eq!(q.next(), ["b", "c"]);
}

#[test]
fn a_need_blocked_by_an_item_too_still_seeds_and_lists_both() {
    // req:orchestrator/queue-from-refs@cbcfd325 req:orchestrator/priority-orders-components@147ea0bf
    // `a/x` is blocked by `backlog/q` and needs `b/y`, which needs `d/z`, so the seeded
    // priority passes on along the needs.
    let b = node_with("", "", &[("y", "It SHALL.\n\n#### Needs\n- d/z")]);
    let mut files = vec![
        ("specs/a/SPEC.md".to_string(), needing("", "b/y", &[])),
        ("specs/b/SPEC.md".to_string(), b),
        ("specs/d/SPEC.md".to_string(), node("", &["z"])),
        (
            "backlog/q.md".to_string(),
            "---\ntitle: Q\nkind: human-task\nblocks: [a/x]\n---\n".to_string(),
        ),
    ];
    files.extend(adds("20260101-a", "P2", "a", "x"));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let g = with_changes(&refs, vec![]);
    let q = queue(&g, &[], &[]);
    match state(&q, "a") {
        State::Blocked { by } => {
            let names: Vec<&str> = by.iter().map(|b| b.blocker()).collect();
            assert_eq!(names, ["backlog/q", "b/y"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(priority(&q, "b"), Priority::P2);
    assert_eq!(priority(&q, "d"), Priority::P2);
    assert_eq!(q.next(), ["d"]);
}

#[test]
fn scenario_split_in_flight() {
    // req:orchestrator/next@fd7e1025
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["w"])),
        ("specs/a/b/SPEC.md", &node("", &["x"])),
        ("changes/s/change.md", &change("applied")),
        (
            "changes/s/deltas/a.md",
            "---\ncomponent: a\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0000000000000000 -->\n",
        ),
        ("changes/s/deltas/a.b.md", &delta("a.b", &["x"])),
    ]);
    let q = queue(&g, &[current(&g, "a/w"), marker("a/x", None, 5)], &[]);
    assert!(matches!(state(&q, "a"), State::Blocked { .. }), "{q:?}");
    assert_eq!(q.next(), ["a.b"]);
}

#[test]
fn scenario_waits_serialised() {
    // req:orchestrator/queue-pending-entries@78481951
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &needing("", "b/y", &["w"])),
        ("specs/b/SPEC.md", &node("", &["y"])),
    ]);
    let q = queue(&g, &[], &[]);
    let json = serde_json::to_value(&q.nodes).unwrap();
    let a = json
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "a")
        .unwrap();
    assert_eq!(a["state"], "ready");
    let x = a["pending"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "a/x")
        .unwrap();
    assert!(x.get("needs_pending").is_none());
    assert_eq!(x["waits_on"], serde_json::json!(["b/y"]));
    assert_eq!(x["blocked_by"], serde_json::json!([]));
}

/// A marker citing `id` with no version at `crates/<component>/t.rs:1`.
fn marker_in(component: &str, id: &str) -> Marker {
    Marker {
        id: id.into(),
        version: None,
        path: std::path::PathBuf::from(format!("crates/{component}/t.rs")),
        line: 1,
    }
}

/// A root with `wiring: []`, a component for each of `owned` with `paths`
/// `["crates/<id>/**"]` and the requirements given, an `applied` change at `priority`
/// removing `b/gone`, and `more` files.
fn removed_gone(priority: &str, owned: &[(&str, &[&str])], more: &[(&str, &str)]) -> Graph {
    let root = "---\ntitle: R\nstatus: active\nowners: []\nwiring: []\n---\n\n## Contract\nr\n";
    let extra = if priority.is_empty() {
        String::new()
    } else {
        format!("priority: {priority}\n")
    };
    let mut files: Vec<(String, String)> = vec![
        ("specs/SPEC.md".into(), root.into()),
        ("changes/s/change.md".into(), change_md("applied", &extra)),
        (
            "changes/s/deltas/b.md".into(),
            "---\ncomponent: b\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0000000000000000 -->\n".into(),
        ),
    ];
    for (id, reqs) in owned {
        let reqs: Vec<(&str, &str)> = reqs.iter().map(|r| (*r, "It SHALL.")).collect();
        files.push((
            format!("specs/{id}/SPEC.md"),
            node_with("", &format!("paths: [\"crates/{id}/**\"]\n"), &reqs),
        ));
    }
    files.extend(more.iter().map(|(p, t)| (p.to_string(), t.to_string())));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    graph(&refs)
}

#[test]
fn scenario_an_obsolete_requirement_waiting_on_its_other_lines() {
    // req:orchestrator/queue-from-refs@cbcfd325
    let g = removed_gone("", &[("b", &[]), ("c", &[]), ("d", &[])], &[]);
    let markers = [
        marker_in("b", "b/gone"),
        marker_in("c", "b/gone"),
        marker_in("d", "b/gone"),
    ];
    let q = queue(&g, &markers, &[]);
    match state(&q, "b") {
        State::Blocked { by } => {
            assert_eq!(by.len(), 1, "{by:?}");
            assert_eq!(by[0].requirement.as_deref(), Some("b/gone"));
            assert_eq!(
                by[0].components.as_deref(),
                Some(&["c".into(), "d".into()][..])
            );
            assert_eq!(by[0].requirements, ["b/gone"]);
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(state(&q, "c"), State::Ready { .. }), "{q:?}");
    assert!(matches!(state(&q, "d"), State::Ready { .. }), "{q:?}");
    let json = serde_json::to_value(&q.nodes).unwrap();
    let b = json
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "b")
        .unwrap();
    assert_eq!(b["by"][0]["components"], serde_json::json!(["c", "d"]));
}

#[test]
fn scenario_an_item_blocker_names_no_components() {
    // req:orchestrator/queue-from-refs@cbcfd325
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["x"])),
        (
            "backlog/q.md",
            "---\ntitle: Q\nkind: question\nblocks: [a/x]\n---\n",
        ),
    ]);
    let q = queue(&g, &[], &[]);
    match state(&q, "a") {
        State::Blocked { by } => assert_eq!(by[0].components, None),
        other => panic!("{other:?}"),
    }
    let json = serde_json::to_value(&q.nodes).unwrap();
    let a = json
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "a")
        .unwrap();
    assert_eq!(a["by"][0]["item"], "backlog/q");
    assert!(a["by"][0].get("components").is_none(), "{a}");
}

#[test]
fn scenario_an_obsolete_requirement_pulls_the_owner_of_its_other_markers_forward() {
    // req:orchestrator/priority-orders-components@147ea0bf
    let mut more: Vec<(String, String)> = vec![
        (
            "backlog/q.md".into(),
            "---\ntitle: Q\nkind: question\nblocks: [b/gone]\n---\n".into(),
        ),
        ("specs/d/SPEC.md".into(), node("", &["x"])),
    ];
    more.extend(adds("20260101-c", "", "c", "y"));
    more.extend(adds("20260101-d", "P3", "d", "x"));
    let more: Vec<(&str, &str)> = more.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
    let g = removed_gone("P1", &[("b", &[]), ("c", &["y"])], &more);
    let markers = [marker_in("b", "b/gone"), marker_in("c", "b/gone")];
    let q = queue(&g, &markers, &[]);
    assert_eq!(q.next(), ["c", "d"]);
    assert_eq!(priority(&q, "c"), Priority::P1);
    assert_eq!(priority(&q, "b"), Priority::P5);
}

#[test]
fn a_requirement_blocker_names_only_its_own_lines_once() {
    // req:orchestrator/queue-from-refs@cbcfd325
    // `a/x` waits on `b/y` and `c/z`, `a/w` on `b/y` too: each blocker names the components
    // of its own ID's lines only, each once, over every requirement it blocks.
    let a = node_with(
        "",
        "",
        &[
            ("x", "It SHALL.\n\n#### Needs\n- b/y\n- c/z"),
            ("w", "It SHALL.\n\n#### Needs\n- b/y"),
        ],
    );
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &a),
        ("specs/b/SPEC.md", &node("", &["y"])),
        ("specs/c/SPEC.md", &node("", &["z"])),
    ]);
    let q = queue(&g, &[], &[]);
    let State::Blocked { by } = state(&q, "a") else {
        panic!("{q:?}");
    };
    let got: Vec<(&str, Vec<String>, Vec<String>)> = by
        .iter()
        .map(|b| {
            (
                b.blocker(),
                b.components.clone().unwrap(),
                b.requirements.clone(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("b/y", vec!["b".into()], vec!["a/w".into(), "a/x".into()]),
            ("c/z", vec!["c".into()], vec!["a/x".into()]),
        ]
    );
}

#[test]
fn an_obsolete_requirement_seeds_every_component_of_its_other_lines() {
    // req:orchestrator/priority-orders-components@147ea0bf
    // `b/gone`'s line on `b` waits on its lines on `c` and `d`, both of one ID: each is
    // followed, by its component and ID together, so both are seeded.
    let mut more: Vec<(String, String)> = vec![(
        "backlog/q.md".into(),
        "---\ntitle: Q\nkind: question\nblocks: [b/gone]\n---\n".into(),
    )];
    more.extend(adds("20260101-c", "", "c", "y"));
    more.extend(adds("20260101-d", "", "d", "y"));
    let more: Vec<(&str, &str)> = more.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
    let g = removed_gone("P1", &[("b", &[]), ("c", &["y"]), ("d", &["y"])], &more);
    let markers = [
        marker_in("b", "b/gone"),
        marker_in("c", "b/gone"),
        marker_in("d", "b/gone"),
    ];
    let q = queue(&g, &markers, &[]);
    assert_eq!(priority(&q, "c"), Priority::P1);
    assert_eq!(priority(&q, "d"), Priority::P1);
    assert_eq!(priority(&q, "b"), Priority::P5);
}

#[test]
fn a_ready_entry_carries_its_item_blockers_and_priority() {
    // req:orchestrator/queue-pending-entries@78481951
    let g = graph(&[
        ROOT,
        ("specs/a/SPEC.md", &node("", &["x", "w"])),
        (
            "backlog/q.md",
            "---\ntitle: Q\nkind: question\nblocks: [a/x]\n---\n",
        ),
    ]);
    let q = queue(&g, &[], &[]);
    let json = serde_json::to_value(&q.nodes).unwrap();
    let a = json
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "a")
        .unwrap();
    assert_eq!(a["state"], "ready");
    let entry = |id: &str| {
        a["pending"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .unwrap()
    };
    assert_eq!(entry("a/x")["blocked_by"], serde_json::json!(["q"]));
    assert_eq!(entry("a/w")["blocked_by"], serde_json::json!([]));
    assert_eq!(entry("a/x")["priority"], "P5");
}
