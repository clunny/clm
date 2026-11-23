//! The `spec`, `flight` and `blockers` models and their charts, as library calls.

mod common;

use std::collections::BTreeSet;

use clm_orchestrator::DEFAULT_EXPIRY_HOURS as EXPIRY;
use clm_view::{
    At, Checkout, Format, Head, View, blockers_view, flight_view, queue_states, render, spec_view,
    status_text,
};
use common::*;
use serde_json::Value;

fn json<T: serde::Serialize>(model: &T) -> Value {
    serde_json::to_value(model).unwrap()
}

fn text(view: &View) -> String {
    render(view, Format::Text, 100, None)
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

// ---- spec ----

/// `clm` (root), `check`, `check.drift` depending on `model`, owned by `@a`, with 2
/// requirements, `x` current and `y` cited only at an older version; `model` with none.
fn nested() -> (clm_model::Graph, Vec<clm_coverage::Marker>) {
    let g = graph(&[
        ("specs/SPEC.md", &node("active", "", "", &[])),
        ("specs/check/SPEC.md", &node("active", "", "", &["c"])),
        (
            "specs/check/drift/SPEC.md",
            &node("specd", "model", "\"@a\"", &["x", "y"]),
        ),
        ("specs/model/SPEC.md", &node("active", "", "", &["m"])),
    ]);
    let markers = vec![
        current(&g, "check.drift/x"),
        marker("check.drift/y", Some("00000000"), 2),
        current(&g, "check/c"),
    ];
    (g, markers)
}

/// The spec view's chart, with the queue states the flight model gives for no claims.
fn spec_chart(g: &clm_model::Graph, markers: &[clm_coverage::Marker]) -> View {
    spec_with(g, markers, &[])
}

fn spec_with(
    g: &clm_model::Graph,
    markers: &[clm_coverage::Marker],
    refs: &[clm_orchestrator::ClaimRef],
) -> View {
    View::Spec(
        spec_view(g, markers, None),
        queue_states(&flight_view(g, markers, refs, NOW, EXPIRY)),
    )
}

#[test]
fn scenario_nested_nodes() {
    // req:view/spec-view@249ab038
    let (g, markers) = nested();
    let v = json(&spec_view(&g, &markers, None));
    assert_eq!(v["view"], "spec");
    let nodes = v["nodes"].as_array().unwrap();
    let find = |id: &str| nodes.iter().find(|n| n["node"] == id).unwrap();
    assert_eq!(find("check.drift")["parent"], "check");
    assert_eq!(find("check")["parent"], "clm");
    assert_eq!(find("clm")["parent"], Value::Null);
    let drift = find("check.drift");
    assert_eq!(drift["owners"], json(&["@a"]));
    assert_eq!(drift["depends_on"], json(&["model"]));
    for (key, n) in [
        ("current", 1),
        ("behind", 1),
        ("unbuilt", 0),
        ("obsolete", 0),
        ("total", 2),
    ] {
        assert_eq!(drift[key], n, "{key} in {drift:#}");
    }
    assert_eq!(drift.get("cited"), None, "{drift:#}");
    assert_eq!(drift.get("specd"), None, "{drift:#}");
    assert_eq!(drift["status"], "unbuilt");
    assert_eq!(drift["title"], "T");
}

#[test]
fn spec_view_holds_every_node_once_in_tree_order() {
    // req:view/spec-view@249ab038
    let (g, markers) = nested();
    let ids: Vec<String> = spec_view(&g, &markers, None)
        .nodes
        .iter()
        .map(|n| n.node.clone())
        .collect();
    assert_eq!(ids, ["clm", "check", "check.drift", "model"]);
}

#[test]
fn scenario_tree_chart() {
    // req:view/spec-view@249ab038
    let (g, markers) = nested();
    let out = text(&spec_chart(&g, &markers));
    let lines = lines(&out);
    let at = |id: &str| {
        lines
            .iter()
            .position(|l| l.contains(&format!("{id} ")) || l.ends_with(id))
            .unwrap_or_else(|| panic!("no {id} in {out}"))
    };
    let (check, drift) = (at("check"), at("check.drift"));
    assert!(drift > check, "{out}");
    let indent = |l: &str| l.find(|c: char| c.is_alphanumeric()).unwrap();
    assert!(indent(lines[drift]) > indent(lines[check]), "{out}");
    let line = lines[drift];
    for fact in ["1/2", "@a", "model", "unbuilt"] {
        assert!(line.contains(fact), "{fact} missing from {line:?}");
    }
    assert!(line.contains("█████▓▓▓▓▓ 1/2"), "{line:?}");
    // `check` depends on nothing: no arrow.
    assert!(!lines[check].contains('→'), "{out}");
    assert!(lines[drift].contains("→ model"), "{out}");
}

/// The chart line of node `id` in `out`.
fn line_of<'a>(out: &'a str, id: &str) -> &'a str {
    out.lines()
        .find(|l| {
            l.trim_start_matches(|c: char| !c.is_alphanumeric())
                .starts_with(&format!("{id} "))
        })
        .unwrap_or_else(|| panic!("no line for {id} in {out}"))
}

/// Node `a` with requirements `r0`..`r<n>`, the first `current` cited at their version and
/// the next `behind` at an older one.
fn counted(
    n: usize,
    current_n: usize,
    behind: usize,
) -> (clm_model::Graph, Vec<clm_coverage::Marker>) {
    let names: Vec<String> = (0..n).map(|i| format!("r{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &refs)),
    ]);
    let mut markers: Vec<_> = names[..current_n]
        .iter()
        .map(|r| current(&g, &format!("a/{r}")))
        .collect();
    markers.extend(
        names[current_n..current_n + behind]
            .iter()
            .map(|r| marker(&format!("a/{r}"), Some("00000000"), 2)),
    );
    (g, markers)
}

#[test]
fn scenario_a_node_behind_its_spec_is_not_full() {
    // req:view/spec-view@249ab038
    let (g, markers) = counted(4, 3, 1);
    let out = text(&spec_chart(&g, &markers));
    let line = line_of(&out, "a");
    assert!(line.contains("3/4"), "{line:?}");
    assert!(line.contains('▓'), "{line:?}");
    assert!(!line.contains('░'), "{line:?}");
}

#[test]
fn bar_gives_every_state_a_cell_and_ties_to_current_first() {
    // req:view/spec-view@249ab038
    for (n, cur, behind, bar) in [
        (3, 1, 1, "████▓▓▓░░░ 1/3"),
        (100, 98, 1, "████████▓░ 98/100"),
        (2, 0, 0, "░░░░░░░░░░ 0/2"),
        (0, 0, 0, "·········· 0/0"),
    ] {
        let (g, markers) = counted(n, cur, behind);
        let out = text(&spec_chart(&g, &markers));
        let line = line_of(&out, "a");
        assert!(line.contains(bar), "{bar} not in {line:?}");
    }
}

#[test]
fn scenario_queue_states_on_the_chart() {
    // req:view/spec-view@249ab038
    // req:view/view-command@c4a4df22
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd("", &["x"])),
        ("specs/c/SPEC.md", &specd("", &["x"])),
        ("backlog/q.md", &item("question", "b/x")),
    ]);
    let markers = [current(&g, "c/x")];
    let view = spec_with(&g, &markers, &[claim("a", 1)]);
    let out = text(&view);
    assert!(line_of(&out, "a").contains("claimed"), "{out}");
    assert!(line_of(&out, "b").contains("blocked"), "{out}");
    let c = line_of(&out, "c");
    for state in ["ready", "claimed", "blocked", "converged"] {
        assert!(!c.contains(state), "{state} in {c:?}");
    }
    // Coloured as the flight chart colours them: claimed amber, blocked rose.
    let ansi = render(&view, Format::Ansi, 100, None);
    assert!(ansi.contains("\u{1b}[33mclaimed\u{1b}[0m"), "{ansi}");
    assert!(ansi.contains("\u{1b}[31mblocked\u{1b}[0m"), "{ansi}");
    // And outlined in those colours in the SVG.
    let svg = render(&view, Format::Svg, 100, None);
    assert!(svg.contains("<rect class=\"node blocked\""), "{svg}");
    let style = &svg[svg.find("<style>").unwrap()..svg.find("</style>").unwrap()];
    for (state, colour) in [("claimed", "#b7791f"), ("blocked", "#dc3558")] {
        let rule = style
            .split('}')
            .find(|r| r.contains(&format!("rect.node.{state}")) && r.contains("stroke:"))
            .unwrap_or_else(|| panic!("no outline rule for {state} in {style}"));
        assert!(rule.contains(colour), "{state}: {rule}");
    }
}

/// Node `a` with `a/x` current, and an applied change whose delta removed `a/old`, which a
/// test still cites.
fn obsolete() -> (clm_model::Graph, Vec<clm_coverage::Marker>) {
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("applied", "")),
        (
            "changes/d/deltas/a.md",
            "---\ncomponent: a\n---\n## REMOVED Requirements\n### Requirement: old\n",
        ),
    ]);
    let markers = vec![current(&g, "a/x"), marker("a/old", None, 3)];
    (g, markers)
}

#[test]
fn scenario_obsolete_markers() {
    // req:view/spec-view@249ab038
    let (g, markers) = obsolete();
    let v = json(&spec_view(&g, &markers, None));
    let a = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "a")
        .unwrap();
    assert_eq!(
        (&a["current"], &a["total"], &a["obsolete"]),
        (&json(&1), &json(&1), &json(&1)),
        "{a:#}"
    );
    let out = text(&spec_chart(&g, &markers));
    let line = line_of(&out, "a");
    assert!(line.contains("1/1 ✕1"), "{line:?}");
}

/// Nodes `a` (paths `src/a/**`) and `b` (paths `src/b/**`), and an applied change whose
/// delta removed node `gone` with its requirement `gone/x`.
fn removed_node() -> clm_model::Graph {
    let with_paths = |paths: &str| {
        let spec = specd("", &["x"]);
        assert_eq!(spec.matches("\ndepends_on").count(), 1, "{spec}");
        spec.replace(
            "\ndepends_on",
            &format!("\npaths: [\"{paths}\"]\ndepends_on"),
        )
    };
    graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &with_paths("src/a/**")),
        ("specs/b/SPEC.md", &with_paths("src/b/**")),
        ("changes/d/change.md", &change("applied", "")),
        (
            "changes/d/deltas/gone.md",
            "---\ncomponent: gone\nremove: true\n---\n## REMOVED Requirements\n\n\
             ### Requirement: x <!-- base: 0123456789abcdef -->\n",
        ),
    ])
}

/// A bare marker citing `id` in `path` at `line`.
fn cite_in(id: &str, path: &str, line: usize) -> clm_coverage::Marker {
    clm_coverage::Marker {
        path: path.into(),
        ..marker(id, None, line)
    }
}

/// The `obsolete` count of each node in the spec model, by ID.
fn obsolete_by_node(
    g: &clm_model::Graph,
    markers: &[clm_coverage::Marker],
) -> std::collections::BTreeMap<String, usize> {
    spec_view(g, markers, None)
        .nodes
        .into_iter()
        .map(|n| (n.node, n.obsolete))
        .collect()
}

#[test]
fn scenario_a_removed_nodes_markers() {
    // req:view/spec-view@249ab038
    let g = removed_node();
    let markers = [current(&g, "a/x"), cite_in("gone/x", "src/a/t.rs", 3)];
    let v = json(&spec_view(&g, &markers, None));
    let nodes = v["nodes"].as_array().unwrap();
    let a = nodes.iter().find(|n| n["node"] == "a").unwrap();
    assert_eq!(a["obsolete"], 1, "{a:#}");
    assert!(nodes.iter().all(|n| n["node"] != "gone"), "{v:#}");
    let view = spec_chart(&g, &markers);
    let out = text(&view);
    assert!(line_of(&out, "a").contains("✕1"), "{out}");
    assert!(!line_of(&out, "b").contains('✕'), "{out}");
    // The SVG draws the same count, once: on `a`'s box.
    let svg = render(&view, Format::Svg, 100, None);
    assert_eq!(svg.matches("✕1").count(), 1, "{svg}");
}

#[test]
fn a_removed_nodes_requirement_counts_once_on_each_owner() {
    // req:view/spec-view@249ab038
    let g = removed_node();
    let markers = [
        cite_in("gone/x", "src/a/t.rs", 3),
        cite_in("gone/x", "src/a/u.rs", 5),
        cite_in("gone/x", "src/b/t.rs", 7),
    ];
    let counts = obsolete_by_node(&g, &markers);
    assert_eq!(
        (counts["a"], counts["b"], counts["clm"]),
        (1, 1, 0),
        "{counts:?}"
    );
}

#[test]
fn a_removed_nodes_marker_no_node_owns_counts_on_the_root() {
    // req:view/spec-view@249ab038
    let g = removed_node();
    let markers = [cite_in("gone/x", "tests/t.rs", 3)];
    let counts = obsolete_by_node(&g, &markers);
    assert_eq!(
        (counts["a"], counts["b"], counts["clm"]),
        (0, 0, 1),
        "{counts:?}"
    );
    let out = text(&spec_chart(&g, &markers));
    assert!(line_of(&out, "clm").contains("✕1"), "{out}");
}

#[test]
fn a_requirement_removed_from_a_node_that_no_longer_exists_counts_on_its_owner() {
    // req:view/spec-view@249ab038
    // The delta removes `gone/old` without `remove: true`, and no node `gone` exists.
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("applied", "")),
        (
            "changes/d/deltas/gone.md",
            "---\ncomponent: gone\n---\n## REMOVED Requirements\n### Requirement: old\n",
        ),
    ]);
    let markers = [current(&g, "a/x"), marker("gone/old", None, 3)];
    let counts = obsolete_by_node(&g, &markers);
    assert_eq!((counts["a"], counts["clm"]), (0, 1), "{counts:?}");
}

#[test]
fn scenario_oversized_without_sizes() {
    // req:view/spec-view@249ab038
    let (g, markers) = nested();
    let v = json(&spec_view(&g, &markers, None));
    for n in v["nodes"].as_array().unwrap() {
        assert_eq!(n.get("oversized"), None, "{n:#}");
    }
    let view = spec_chart(&g, &markers);
    let out = text(&view);
    assert!(!out.contains('⚠') && !out.contains("oversized"), "{out}");
    let out = render(&view, Format::Svg, 100, None);
    assert!(!out.contains('⚠'), "{out}");
    assert!(!out.contains("node oversized"), "{out}");
}

/// `nested()` with `check.drift` and `model` oversized, and `check.drift` claimed.
fn oversized_view() -> (View, clm_view::SpecView) {
    let (g, mut markers) = nested();
    markers.push(current(&g, "model/m"));
    let big: BTreeSet<String> = ["check.drift", "model"].map(String::from).into();
    let model = spec_view(&g, &markers, Some(&big));
    let states = queue_states(&flight_view(
        &g,
        &markers,
        &[claim("check.drift", 1)],
        NOW,
        EXPIRY,
    ));
    (View::Spec(model.clone(), states), model)
}

#[test]
fn oversized_marks_the_model_and_the_chart_line() {
    // req:view/spec-view@249ab038
    let (view, model) = oversized_view();
    let v = json(&model);
    let find = |id: &str| {
        v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["node"] == id)
            .unwrap()
            .clone()
    };
    assert_eq!(find("model")["oversized"], true);
    assert_eq!(find("check.drift")["oversized"], true);
    assert_eq!(find("check")["oversized"], false);
    let out = text(&view);
    assert!(line_of(&out, "model").contains("⚠ oversized"), "{out}");
    // After the queue state.
    let drift = line_of(&out, "check.drift");
    let (claimed, warn) = (drift.find("claimed"), drift.find("⚠ oversized"));
    assert!(claimed.is_some() && warn > claimed, "{drift:?}");
    assert!(!line_of(&out, "check").contains('⚠'), "{out}");
    let ansi = render(&view, Format::Ansi, 100, None);
    assert_eq!(strip_ansi(&ansi), out);
}

#[test]
fn oversized_marks_the_svg_label_and_outline() {
    // req:view/spec-view@249ab038
    let (view, _) = oversized_view();
    let out = render(&view, Format::Svg, 100, None);
    let rect = |id: &str| {
        let lines: Vec<&str> = out.lines().collect();
        let label = lines
            .iter()
            .position(|l| l.contains(&format!(">{id}  ")))
            .unwrap_or_else(|| panic!("no label for {id} in {out}"));
        (lines[label - 1].to_string(), lines[label].to_string())
    };
    // A converged oversized node: amber outline, `⚠` in its label.
    let (model_rect, model_label) = rect("model");
    assert!(
        model_rect.contains("class=\"node oversized\""),
        "{model_rect}"
    );
    assert!(model_label.contains('⚠'), "{model_label}");
    assert!(out.contains("rect.node.oversized{stroke:#b7791f}"), "{out}");
    // A claimed one keeps its queue state's outline.
    let (drift_rect, drift_label) = rect("check.drift");
    assert!(
        drift_rect.contains("class=\"node claimed\""),
        "{drift_rect}"
    );
    assert!(drift_label.contains('⚠'), "{drift_label}");
    // Within its budgets: no mark.
    let (check_rect, check_label) = rect("check");
    assert!(check_rect.contains("class=\"node active\""), "{check_rect}");
    assert!(!check_label.contains('⚠'), "{check_label}");
}

// ---- flight ----

#[test]
fn scenario_claimed_node() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["p", "q"])),
    ]);
    let v = json(&flight_view(&g, &[], &[claim("a", 1)], NOW, EXPIRY));
    assert_eq!(v["view"], "flight");
    let a = &v["nodes"][0];
    assert_eq!(a["node"], "a");
    assert_eq!(a["state"], "claimed");
    assert_eq!(a["claim"]["pushed_hours_ago"], 1);
    assert_eq!(a["claim"]["lapses_in_hours"], 3);
    assert_eq!(a["claim"]["lapses_at"], NOW + 3 * 3600);
    let pending = a["pending"].as_array().unwrap();
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().all(|p| p["state"] == "unbuilt"), "{a:#}");
    // Only `interactive` nodes filled `paths`, and that state is gone.
    assert!(a.get("paths").is_none(), "{a:#}");
    assert_eq!(v["totals"]["pending_nodes"], 1);
    assert_eq!(v["totals"]["by_state"]["claimed"], 1);
}

#[test]
fn hours_are_rounded_to_the_nearest() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["p"])),
    ]);
    let refs = [clm_orchestrator::ClaimRef {
        node: "a".into(),
        tip_time: NOW - 3600 - 20,
        merged: false,
    }];
    let v = json(&flight_view(&g, &[], &refs, NOW, EXPIRY));
    assert_eq!(v["nodes"][0]["claim"]["pushed_hours_ago"], 1);
    assert_eq!(v["nodes"][0]["claim"]["lapses_in_hours"], 3);
    // A half rounds up.
    let refs = [clm_orchestrator::ClaimRef {
        node: "a".into(),
        tip_time: NOW - 1800,
        merged: false,
    }];
    let v = json(&flight_view(&g, &[], &refs, NOW, EXPIRY));
    assert_eq!(v["nodes"][0]["claim"]["pushed_hours_ago"], 1);
    assert_eq!(v["nodes"][0]["claim"]["lapses_in_hours"], 4);
    // A push after the current time was 0 hours ago.
    let refs = [clm_orchestrator::ClaimRef {
        node: "a".into(),
        tip_time: NOW + 60,
        merged: false,
    }];
    let v = json(&flight_view(&g, &[], &refs, NOW, EXPIRY));
    assert_eq!(v["nodes"][0]["state"], "claimed", "{v:#}");
    assert_eq!(v["nodes"][0]["claim"]["pushed_hours_ago"], 0);
    assert_eq!(v["nodes"][0]["claim"]["lapses_in_hours"], 4);
}

#[test]
fn scenario_change_progress() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x", "y"])),
        ("changes/d/change.md", &change("applied", "")),
        ("changes/d/deltas/n.md", &delta("n", &["x", "y"])),
    ]);
    let markers = [current(&g, "n/x"), marker("n/y", Some("00000000"), 2)];
    let v = json(&flight_view(&g, &markers, &[], NOW, EXPIRY));
    let group = &v["changes"][0];
    assert_eq!(group["status"], "applied");
    let d = &group["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!(d["current"], 1);
    assert_eq!(d["total"], 2);
    assert_eq!(d["complete"], false);
    assert_eq!(v["totals"]["requirements_pending"], 1);
    assert_eq!(v["totals"]["live_changes"], 1);
    // The node is pending too, with `y` behind.
    assert_eq!(v["nodes"][0]["pending"][0]["state"], "behind");
}

#[test]
fn a_complete_change_and_a_marker_citing_a_removed_requirement() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("implementing", "")),
        (
            "changes/d/deltas/n.md",
            "---\ncomponent: n\n---\n## ADDED Requirements\n### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n## REMOVED Requirements\n### Requirement: gone\n",
        ),
    ]);
    let done = [current(&g, "n/x")];
    let v = json(&flight_view(&g, &done, &[], NOW, EXPIRY));
    assert_eq!(v["changes"][0]["changes"][0]["complete"], true);
    assert_eq!(
        v["changes"][0]["changes"][0]["obsolete"],
        json(&Vec::<String>::new())
    );
    assert_eq!(
        v["nodes"],
        json(&Vec::<String>::new()),
        "converged nodes are left out"
    );
    let stale = [current(&g, "n/x"), marker("n/gone", None, 3)];
    let v = json(&flight_view(&g, &stale, &[], NOW, EXPIRY));
    assert_eq!(v["changes"][0]["changes"][0]["complete"], false);
    assert_eq!(v["changes"][0]["changes"][0]["obsolete"], json(&["t.rs:3"]));
}

#[test]
fn each_change_is_exactly_its_change_progress() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("implementing", "")),
        ("changes/d/deltas/n.md", &delta("n", &["x", "y"])),
        ("changes/e/change.md", &change("applied", "")),
        (
            "changes/e/deltas/n.md",
            "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: y\n",
        ),
        ("changes/f/change.md", &change("proposed", "")),
        ("changes/f/deltas/n.md", &delta("n", &["z"])),
        ("changes/g/change.md", &change("done", "")),
        ("changes/g/deltas/n.md", &delta("n", &["w"])),
    ]);
    let markers = [current(&g, "n/x"), marker("n/y", None, 2)];
    let view = flight_view(&g, &markers, &[], NOW, EXPIRY);
    let listed: Vec<_> = view.changes.iter().flat_map(|g| &g.changes).collect();
    let progress: Vec<_> = clm_coverage::change_progress(&g, &markers)
        .into_iter()
        .filter(|p| p.change != "g")
        .collect();
    assert_eq!(listed.len(), progress.len(), "done changes are left out");
    for p in progress {
        let c = listed.iter().find(|c| c.change == p.change).unwrap();
        assert_eq!(
            (c.current, c.total, &c.obsolete, c.complete),
            (p.current, p.total, &p.obsolete, p.complete),
            "{}",
            p.change
        );
    }
}

#[test]
fn scenario_requirement_removed_later() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("applied", "")),
        ("changes/d/deltas/n.md", &delta("n", &["x", "y"])),
    ]);
    let markers = [current(&g, "n/x")];
    let v = json(&flight_view(&g, &markers, &[], NOW, EXPIRY));
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!(d["current"], 1);
    assert_eq!(d["total"], 1);
    assert_eq!(d["obsolete"], json(&Vec::<String>::new()));
    assert_eq!(d["complete"], true);
}

#[test]
fn an_implementing_change_leaves_out_a_requirement_a_later_change_removed() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("implementing", "")),
        ("changes/d/deltas/n.md", &delta("n", &["x", "y"])),
        ("changes/e/change.md", &change("applied", "")),
        (
            "changes/e/deltas/n.md",
            "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: y\n",
        ),
    ]);
    // `n/y` is obsolete: a marker cites it and live change `e` removed it.
    let markers = [current(&g, "n/x"), marker("n/y", None, 2)];
    let v = json(&flight_view(&g, &markers, &[], NOW, EXPIRY));
    let d = &v["changes"][0]["changes"][0];
    assert_eq!(d["change"], "d");
    assert_eq!((&d["current"], &d["total"]), (&json(&1), &json(&1)));
    assert_eq!(d["complete"], true);
    let e = &v["changes"][1]["changes"][0];
    assert_eq!(e["change"], "e");
    assert_eq!(e["obsolete"], json(&["t.rs:2"]));
    assert_eq!(e["complete"], false);
}

#[test]
fn an_unapplied_change_counts_requirements_the_spec_does_not_hold_yet() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/p/change.md", &change("proposed", "")),
        ("changes/p/deltas/n.md", &delta("n", &["z"])),
    ]);
    let v = json(&flight_view(&g, &[], &[], NOW, EXPIRY));
    let p = &v["changes"][0]["changes"][0];
    assert_eq!((&p["current"], &p["total"]), (&json(&0), &json(&1)));
    assert_eq!(p["complete"], false);
}

#[test]
fn a_change_lists_no_marker_for_a_removed_requirement_the_spec_still_holds() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/p/change.md", &change("proposed", "")),
        (
            "changes/p/deltas/n.md",
            "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: x\n",
        ),
    ]);
    let markers = [current(&g, "n/x")];
    let v = json(&flight_view(&g, &markers, &[], NOW, EXPIRY));
    let p = &v["changes"][0]["changes"][0];
    assert_eq!(p["change"], "p");
    assert_eq!(p["obsolete"], json(&Vec::<String>::new()));
}

#[test]
fn flight_leaves_out_done_changes_and_the_archive() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x"])),
        ("changes/d/change.md", &change("done", "")),
        ("changes/d/deltas/n.md", &delta("n", &["x"])),
        ("changes/e/change.md", &change("abandoned", "")),
        ("changes/archive/f/change.md", &change("applied", "")),
        ("changes/archive/f/deltas/n.md", &delta("n", &["x"])),
        ("changes/p/change.md", &change("proposed", "")),
        ("changes/p/deltas/n.md", &delta("n", &["z"])),
    ]);
    let v = json(&flight_view(&g, &[], &[], NOW, EXPIRY));
    let groups: Vec<&str> = v["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["status"].as_str().unwrap())
        .collect();
    assert_eq!(groups, ["proposed"]);
    assert_eq!(v["totals"]["live_changes"], 1);
}

/// Pending nodes `a` at `P5` and `z` at `P2`; `a1` applied, `z9` implementing at `P2`.
fn ordered() -> clm_model::Graph {
    graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/z/SPEC.md", &specd("", &["x"])),
        ("changes/a1/change.md", &change("applied", "")),
        ("changes/a1/deltas/a.md", &delta("a", &["x"])),
        (
            "changes/z9/change.md",
            &change("implementing", "priority: P2\n"),
        ),
        ("changes/z9/deltas/z.md", &delta("z", &["x"])),
    ])
}

#[test]
fn scenario_order() {
    // req:view/flight-view@86ea7638
    let g = ordered();
    let out = text(&View::Flight(flight_view(&g, &[], &[], NOW, EXPIRY)));
    let pos = |s: &str| out.find(s).unwrap_or_else(|| panic!("no {s} in {out}"));
    assert!(pos("\n  z ") < pos("\n  a "), "{out}");
    assert!(pos("z9") < pos("a1"), "{out}");
    let v = json(&flight_view(&g, &[], &[], NOW, EXPIRY));
    assert_eq!(v["nodes"][0]["node"], "z");
    assert_eq!(v["nodes"][0]["priority"], "P2");
    assert_eq!(v["nodes"][1]["node"], "a");
}

#[test]
fn changes_in_a_group_sort_by_priority_then_id() {
    // req:view/flight-view@86ea7638
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x", "y", "z"])),
        ("changes/b/change.md", &change("applied", "priority: P3\n")),
        ("changes/b/deltas/n.md", &delta("n", &["x"])),
        ("changes/a/change.md", &change("applied", "")),
        ("changes/a/deltas/n.md", &delta("n", &["y"])),
        ("changes/c/change.md", &change("applied", "priority: P3\n")),
        ("changes/c/deltas/n.md", &delta("n", &["z"])),
    ]);
    let v = json(&flight_view(&g, &[], &[], NOW, EXPIRY));
    let ids: Vec<&str> = v["changes"][0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["change"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["b", "c", "a"]);
}

#[test]
fn flight_chart_names_every_state_and_the_claim() {
    // req:view/flight-view@86ea7638
    // req:view/view-command@c4a4df22
    let twelve: Vec<String> = (0..12).map(|i| format!("s{i:02}")).collect();
    let twelve: Vec<&str> = twelve.iter().map(String::as_str).collect();
    let d = specd("", &twelve).replace(
        "depends_on: []\n",
        "depends_on: []\npaths: [\"src/d/**\"]\n",
    );
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["p"])),
        ("specs/b/SPEC.md", &needs(&[("q", &["a/p"])])),
        ("specs/c/SPEC.md", &specd("", &["r"])),
        ("specs/d/SPEC.md", &d),
        ("backlog/q.md", &item("question", "c/r")),
        ("changes/e/change.md", &change("applied", "")),
        (
            "changes/e/deltas/n.md",
            "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: x\n",
        ),
    ]);
    // `n/x` is held by no component, and a test in a file `d` owns still cites it.
    let markers = [cite_in("n/x", "src/d/t.rs", 3)];
    let view = View::Flight(flight_view(&g, &markers, &[claim("a", 1)], NOW, EXPIRY));
    let out = text(&view);
    let line = |id: &str| {
        out.lines()
            .find(|l| l.starts_with(&format!("  {id} ")))
            .unwrap_or_else(|| panic!("no {id} in {out}"))
            .to_string()
    };
    assert!(line("a").contains("claimed"), "{out}");
    assert!(line("a").contains("pushed 1h ago, lapses in 3h"), "{out}");
    assert!(
        line("b").contains("blocked") && line("b").contains("a/p"),
        "{out}"
    );
    assert!(
        line("c").contains("blocked") && line("c").contains("backlog/q"),
        "{out}"
    );
    assert!(line("b").contains("by a/p"), "{out}");
    assert!(line("c").contains("by backlog/q"), "{out}");
    assert!(line("d").contains("ready"), "{out}");
    // Ten cells, then `…`, then the count: twelve unbuilt and the obsolete marker.
    let bar: String = line("d")
        .chars()
        .skip_while(|c| !"▓░✕".contains(*c))
        .take_while(|c| *c != ' ')
        .collect();
    assert_eq!(bar.chars().count(), 11, "{bar:?} in {out}");
    assert!(bar.ends_with('…'), "{bar:?} in {out}");
    assert!(line("d").contains("… 13 pending"), "{out}");
    let first = out.lines().next().unwrap();
    assert!(first.starts_with("flight"), "{out}");
    assert!(
        first.contains("4 components pending (1 claimed, 1 ready, 2 blocked)"),
        "{out}"
    );
    let e = out
        .lines()
        .find(|l| l.starts_with("    e "))
        .unwrap_or_else(|| panic!("no e in {out}"));
    assert!(e.contains("1 obsolete"), "{out}");
    assert!(!out.contains('\u{1b}'), "{out}");
}

/// The lines of `out` between the line `components` and the line `changes`, and those after
/// `changes`.
fn node_and_change_lines(out: &str) -> (Vec<&str>, Vec<&str>) {
    let lines: Vec<&str> = out.lines().collect();
    let nodes = lines.iter().position(|l| *l == "components").expect(out);
    let changes = lines.iter().position(|l| *l == "changes").expect(out);
    assert!(nodes < changes, "{out}");
    (
        lines[nodes + 1..changes].to_vec(),
        lines[changes + 1..].to_vec(),
    )
}

#[test]
fn scenario_node_lines() {
    // req:view/flight-component-lines@92827b9d
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd("", &[])),
        ("specs/b/c/SPEC.md", &specd("", &["x"])),
        ("changes/20260101-d/change.md", &change("applied", "")),
        ("changes/20260101-d/deltas/a.md", &delta("a", &["x"])),
    ]);
    let view = View::Flight(flight_view(&g, &[], &[claim("a", 1)], NOW, EXPIRY));
    let ansi = render(&view, Format::Ansi, 100, None);
    let out = strip_ansi(&ansi);
    assert_eq!(out, text(&view));
    let (nodes, changes) = node_and_change_lines(&out);
    assert_eq!(nodes.len(), 2, "{out}");
    assert!(nodes[0].starts_with("  a "), "{out}");
    assert!(nodes[1].starts_with("  b.c "), "{out}");
    assert_eq!(changes[0], "  applied", "{out}");
    let change = changes
        .iter()
        .find(|l| l.contains("20260101-d"))
        .expect(&out);
    assert!(change.starts_with("    "), "{out}");
    for l in &changes {
        assert!(
            l.starts_with("  ")
                && (*l == "  none" || l.starts_with("   ") || !l[2..].starts_with(' ')),
            "{l:?} in {out}"
        );
    }
}

#[test]
fn scenario_long_id() {
    // req:view/flight-component-lines@92827b9d
    let long = "a".repeat(60);
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        (&format!("specs/{long}/SPEC.md"), &specd("", &["x"])),
    ]);
    let view = View::Flight(flight_view(&g, &[], &[], NOW, EXPIRY));
    for format in [Format::Text, Format::Ansi] {
        let out = strip_ansi(&render(&view, format, 40, None));
        for line in out.lines() {
            assert!(line.chars().count() <= 40, "{line:?} in {out}");
        }
        let (nodes, _) = node_and_change_lines(&out);
        assert_eq!(nodes.len(), 1, "{out}");
        let rest = nodes[0].strip_prefix("  ").expect(&out);
        let prefix = rest.strip_suffix('…').expect(&out);
        assert!(!prefix.is_empty() && long.starts_with(prefix), "{out}");
    }
}

#[test]
fn scenario_nothing_pending() {
    // req:view/flight-component-lines@92827b9d
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
    ]);
    let markers = [current(&g, "a/x")];
    let out = text(&View::Flight(flight_view(&g, &markers, &[], NOW, EXPIRY)));
    let (nodes, changes) = node_and_change_lines(&out);
    assert_eq!(nodes, ["  none"], "{out}");
    assert_eq!(changes, ["  none"], "{out}");
}

// ---- blockers ----

#[test]
fn scenario_chain_weight() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &needs(&[("y", &["a/x"])])),
        ("specs/c/SPEC.md", &needs(&[("x", &["b/y"])])),
    ]);
    let v = json(&blockers_view(&g, &[], &[claim("a", 1)], NOW, EXPIRY));
    assert_eq!(v["view"], "blockers");
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1, "{v:#}");
    let a = &roots[0];
    assert_eq!(a["root"], "a");
    assert_eq!(a["kind"], "node");
    assert_eq!(a["state"], "claimed");
    assert_eq!(a["waits_on_human"], false);
    assert_eq!(a["weight"], 2);
    assert_eq!(a["holds"][0]["node"], "b");
    assert_eq!(a["holds"][0]["via"], json(&Vec::<String>::new()));
    assert_eq!(a["holds"][1]["node"], "c");
    // `b/y` is blocked, so `c` waits on its blocker `a/x` directly.
    assert_eq!(a["holds"][1]["via"], json(&Vec::<String>::new()));
}

/// A root with `wiring: []`, and nodes `ids`, each owning `src/<id>/**` with no requirement,
/// and an applied change that removed `b/gone`.
fn removed_on(ids: &[&str]) -> clm_model::Graph {
    let root = specd("", &[]).replace("depends_on: []\n", "depends_on: []\nwiring: []\n");
    let specs: Vec<(String, String)> = ids
        .iter()
        .map(|id| {
            (
                format!("specs/{id}/SPEC.md"),
                specd("", &[]).replace(
                    "depends_on: []\n",
                    &format!("depends_on: []\npaths: [\"src/{id}/**\"]\n"),
                ),
            )
        })
        .collect();
    let mut files: Vec<(&str, &str)> = vec![
        ("specs/SPEC.md", &root),
        ("changes/d/change.md", "---\nstatus: applied\n---\n"),
        (
            "changes/d/deltas/b.md",
            "---\ncomponent: b\n---\n## REMOVED Requirements\n### Requirement: gone\n",
        ),
    ];
    files.extend(specs.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    graph(&files)
}

#[test]
fn scenario_an_obsolete_requirement_pending_on_two_components() {
    // req:view/blockers-view@e774ca77
    let g = removed_on(&["a", "b"]);
    let markers = [
        cite_in("b/gone", "src/b/t.rs", 3),
        cite_in("b/gone", "src/a/t.rs", 3),
    ];
    let view = blockers_view(&g, &markers, &[], NOW, EXPIRY);
    assert_eq!(
        roots_of(&view),
        [("a".into(), 1, vec!["b".into()])],
        "{:#}",
        json(&view)
    );
}

#[test]
fn scenario_an_obsolete_requirement_pending_on_three_components() {
    // req:view/blockers-view@e774ca77
    let g = removed_on(&["a", "b", "c"]);
    let markers = [
        cite_in("b/gone", "src/a/t.rs", 3),
        cite_in("b/gone", "src/b/t.rs", 3),
        cite_in("b/gone", "src/c/t.rs", 3),
    ];
    let view = blockers_view(&g, &markers, &[], NOW, EXPIRY);
    assert_eq!(
        roots_of(&view),
        [
            ("a".into(), 1, vec!["b".into()]),
            ("c".into(), 1, vec!["b".into()]),
        ],
        "{:#}",
        json(&view)
    );
}

/// `a`'s only pending requirement is blocked by `backlog/q` (a question); `e`..`h` each
/// need `d/x`, uncited on claimed `d`.
fn human_first() -> clm_model::Graph {
    graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/d/SPEC.md", &specd("", &["x"])),
        ("specs/e/SPEC.md", &needs(&[("x", &["d/x"])])),
        ("specs/f/SPEC.md", &needs(&[("x", &["d/x"])])),
        ("specs/g/SPEC.md", &needs(&[("x", &["d/x"])])),
        ("specs/h/SPEC.md", &needs(&[("x", &["d/x"])])),
        ("backlog/q.md", &item("question", "a/x")),
    ])
}

#[test]
fn scenario_human_first() {
    // req:view/blockers-view@e774ca77
    // req:view/blockers-root-lines@31794d8d
    let g = human_first();
    let view = blockers_view(&g, &[], &[claim("d", 1)], NOW, EXPIRY);
    let v = json(&view);
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots[0]["root"], "backlog/q", "{v:#}");
    assert_eq!(roots[0]["kind"], "backlog");
    assert_eq!(roots[0]["state"], "question");
    assert_eq!(roots[0]["waits_on_human"], true);
    assert_eq!(roots[0]["weight"], 1);
    assert_eq!(roots[1]["root"], "d");
    assert_eq!(roots[1]["weight"], 4);
    let out = text(&View::Blockers(view));
    let pos = |s: &str| out.find(s).unwrap_or_else(|| panic!("no {s} in {out}"));
    assert!(pos("backlog/q") < pos("\nd "), "{out}");
    assert!(out.contains("waits on you"), "{out}");
    assert!(out.contains("holds up 4"), "{out}");
    assert!(out.contains("question"), "{out}");
}

#[test]
fn scenario_dependency_holds_nothing() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd("a", &["x"])),
        ("backlog/q.md", &item("question", "a/x")),
    ]);
    let v = json(&blockers_view(&g, &[], &[], NOW, EXPIRY));
    let held: Vec<&str> = v["roots"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["holds"].as_array().unwrap())
        .map(|h| h["node"].as_str().unwrap())
        .collect();
    // `b` depends on `a`, but its own requirement is unblocked, so it is ready.
    assert_eq!(held, ["a"], "{v:#}");
    let flight = json(&flight_view(&g, &[], &[], NOW, EXPIRY));
    let b = flight["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == "b")
        .unwrap();
    assert_eq!(b["state"], "ready", "{flight:#}");
}

#[test]
fn scenario_nothing_blocked() {
    // req:view/blockers-root-lines@31794d8d
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd("", &["x"])),
        ("specs/c/SPEC.md", &specd("", &["x"])),
    ]);
    let markers = [current(&g, "c/x")];
    let view = blockers_view(&g, &markers, &[claim("b", 1)], NOW, EXPIRY);
    assert!(view.roots.is_empty());
    let out = text(&View::Blockers(view));
    assert!(out.contains("nothing is blocked"), "{out}");
}

#[test]
fn a_node_waiting_on_two_chains_is_under_each_root() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        (
            "specs/a/SPEC.md",
            &(node("specd", "", "", &[]).replace(
                "depends_on: []\n",
                "depends_on: []\npaths: [\".claude/**\"]\n",
            ) + "\n### Requirement: w\nWrites `.claude/settings.json`.\n\n#### Scenario: s\n- ok\n"),
        ),
        ("specs/b/SPEC.md", &specd("", &["x"])),
        ("specs/c/SPEC.md", &needs(&[("x", &["a/w", "b/x"])])),
        ("specs/d/SPEC.md", &needs(&[("x", &["c/x"])])),
    ]);
    let v = json(&blockers_view(&g, &[], &[], NOW, EXPIRY));
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2, "{v:#}");
    assert_eq!(roots[0]["root"], "a");
    // A node that writes a protected path is ready like any other (`builds-in-subagents`).
    assert_eq!(roots[0]["state"], "ready");
    assert_eq!(roots[0]["waits_on_human"], false);
    assert_eq!(roots[0]["weight"], 2);
    assert_eq!(roots[1]["root"], "b");
    assert_eq!(roots[1]["weight"], 2);
    // `c/x` is blocked, so `d` waits on its blockers directly.
    assert_eq!(roots[1]["holds"][1]["node"], "d");
    assert_eq!(roots[1]["holds"][1]["via"], json(&Vec::<String>::new()));
}

#[test]
fn roots_of_equal_weight_sort_by_name_after_weight() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd("", &["x"])),
        ("specs/c/SPEC.md", &specd("", &["x"])),
        ("specs/d/SPEC.md", &needs(&[("x", &["c/x"])])),
        ("specs/e/SPEC.md", &needs(&[("x", &["c/x"])])),
        ("specs/f/SPEC.md", &needs(&[("x", &["a/x"])])),
        ("specs/g/SPEC.md", &needs(&[("x", &["b/x"])])),
    ]);
    let v = json(&blockers_view(&g, &[], &[], NOW, EXPIRY));
    let roots: Vec<&str> = v["roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["root"].as_str().unwrap())
        .collect();
    assert_eq!(roots, ["c", "a", "b"]);
}

/// A specd node depending on `deps` whose requirement `x` needs `needs`, plus `more`.
fn needing(deps: &str, needs: &str, more: &[&str]) -> String {
    specd(deps, more).replace(
        "## Requirements\n",
        &format!(
            "## Requirements\n\n### Requirement: x\nIt SHALL.\n\n#### Needs\n- {needs}\n\n\
             #### Scenario: s\n- ok\n"
        ),
    )
}

/// A specd node depending on nothing, each requirement `(slug, needs)` needing those listed.
fn needs(reqs: &[(&str, &[&str])]) -> String {
    let mut s = specd("", &[]);
    for (r, needed) in reqs {
        let list: String = needed.iter().map(|n| format!("- {n}\n")).collect();
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Needs\n{list}\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// The roots of a blockers view as `(root, weight, held nodes)`.
fn roots_of(view: &clm_view::BlockersView) -> Vec<(String, usize, Vec<String>)> {
    view.roots
        .iter()
        .map(|r| {
            (
                r.root.clone(),
                r.weight,
                r.holds.iter().map(|h| h.node.clone()).collect(),
            )
        })
        .collect()
}

#[test]
fn scenario_need_reaches_its_node() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &specd("", &["y"])),
    ]);
    let view = blockers_view(&g, &[], &[claim("b", 1)], NOW, EXPIRY);
    assert_eq!(roots_of(&view), [("b".into(), 1, vec!["a".into()])]);
    assert_eq!(view.roots[0].state, "claimed");
}

#[test]
fn scenario_need_reaches_the_needed_requirements_item() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &specd("", &["y", "z"])),
        ("backlog/q.md", &item("question", "b/y")),
    ]);
    let view = blockers_view(&g, &[], &[], NOW, EXPIRY);
    assert_eq!(
        roots_of(&view),
        [("backlog/q".into(), 1, vec!["a".into()])],
        "{:#}",
        json(&view)
    );
}

#[test]
fn a_need_on_a_node_with_an_unsettled_dependency_stops_at_that_node() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &needing("", "b/y", &[])),
        ("specs/b/SPEC.md", &specd("c", &["y"])),
        ("specs/c/SPEC.md", &specd("", &["w"])),
    ]);
    let view = blockers_view(&g, &[], &[], NOW, EXPIRY);
    // `b/y` is unblocked, so `b` is ready whatever `c` holds (`needs-alone-order`).
    assert_eq!(
        roots_of(&view),
        [("b".into(), 1, vec!["a".into()])],
        "{:#}",
        json(&view)
    );
    assert_eq!(view.roots[0].state, "ready");
}

#[test]
fn a_need_on_a_requirement_that_needs_another_reaches_the_last_ones_node() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &needing("", "b/x", &[])),
        ("specs/b/SPEC.md", &needing("", "c/y", &[])),
        ("specs/c/SPEC.md", &specd("", &["y"])),
    ]);
    let view = blockers_view(&g, &[], &[], NOW, EXPIRY);
    // `b/x` is blocked, so `a` waits on its blocker `c/y`, whose node is `c`, directly.
    assert_eq!(
        roots_of(&view),
        [("c".into(), 2, vec!["a".into(), "b".into()])],
        "{:#}",
        json(&view)
    );
    assert!(view.roots[0].holds.iter().all(|h| h.via.is_empty()));
}

#[test]
fn a_cycle_of_needs_in_a_graph_with_violations_terminates() {
    // req:view/blockers-view@e774ca77
    let files: clm_model::Files = [
        ("specs/SPEC.md", specd("", &[])),
        ("specs/a/SPEC.md", needing("", "b/x", &[])),
        ("specs/b/SPEC.md", needing("", "a/x", &[])),
        ("specs/c/SPEC.md", needing("", "a/x", &[])),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t.into_bytes()))
    .collect();
    let (g, violations) = clm_model::load_files(&files);
    assert!(!violations.is_empty());
    let view = blockers_view(&g, &[], &[], NOW, EXPIRY);
    // The cycle is one root, named by its first ID, holding its other component and `c`.
    assert_eq!(
        roots_of(&view),
        [("a".into(), 2, vec!["b".into(), "c".into()])],
        "{:#}",
        json(&view)
    );
    assert_eq!(view.roots[0].state, "blocked");
    assert_eq!(view.roots[0].kind, "node");
    assert!(!view.roots[0].waits_on_human);
    assert!(view.roots[0].holds.iter().all(|h| h.via.is_empty()));
}

#[test]
fn a_cycle_is_named_by_its_first_id_whatever_reaches_it() {
    // req:view/blockers-view@e774ca77
    // `d` needs `c/x`, which is in the cycle `c` -> `b` -> `c`; `e` needs `d/x`.
    let files: clm_model::Files = [
        ("specs/SPEC.md", specd("", &[])),
        ("specs/b/SPEC.md", needing("", "c/x", &[])),
        ("specs/c/SPEC.md", needing("", "b/x", &[])),
        ("specs/d/SPEC.md", needing("", "c/x", &[])),
        ("specs/e/SPEC.md", needing("", "d/x", &[])),
    ]
    .into_iter()
    .map(|(p, t)| (p.into(), t.into_bytes()))
    .collect();
    let (g, _) = clm_model::load_files(&files);
    let view = blockers_view(&g, &[], &[], NOW, EXPIRY);
    assert_eq!(
        roots_of(&view),
        [("b".into(), 3, vec!["c".into(), "d".into(), "e".into()])],
        "{:#}",
        json(&view)
    );
    assert_eq!(view.roots[0].state, "blocked");
}

#[test]
fn a_need_blocked_by_an_item_and_a_need_reaches_both() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &needing("", "b/x", &[])),
        ("specs/b/SPEC.md", &needing("", "c/y", &[])),
        ("specs/c/SPEC.md", &specd("", &["y"])),
        ("backlog/q.md", &item("question", "b/x")),
    ]);
    let view = blockers_view(&g, &[], &[], NOW, EXPIRY);
    assert_eq!(
        roots_of(&view),
        [
            ("backlog/q".into(), 2, vec!["a".into(), "b".into()]),
            ("c".into(), 2, vec!["a".into(), "b".into()]),
        ],
        "{:#}",
        json(&view)
    );
}

// ---- rendering ----

#[test]
fn ansi_adds_colour_and_nothing_else() {
    // req:view/view-command@c4a4df22
    let (g, markers) = nested();
    let view = spec_with(&g, &markers, &[claim("check.drift", 1)]);
    let plain = render(&view, Format::Text, 100, None);
    let ansi = render(&view, Format::Ansi, 100, None);
    assert_ne!(plain, ansi);
    assert!(ansi.contains("\u{1b}["), "{ansi}");
    assert_eq!(strip_ansi(&ansi), plain);
}

#[test]
fn lines_are_cut_to_the_width_in_text_and_ansi() {
    // req:view/view-command@c4a4df22
    let long = "a".repeat(60);
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/b/SPEC.md", &specd("", &["x"])),
        (&format!("specs/{long}/SPEC.md"), &needs(&[("x", &["b/x"])])),
    ]);
    let views = [
        spec_chart(&g, &[]),
        View::Flight(flight_view(&g, &[], &[], NOW, EXPIRY)),
        View::Blockers(blockers_view(&g, &[], &[claim("b", 1)], NOW, EXPIRY)),
    ];
    for view in &views {
        let plain = render(view, Format::Text, 40, None);
        for line in plain.lines() {
            assert!(line.chars().count() <= 40, "{line:?} in {plain}");
        }
        assert!(plain.contains('…'), "{plain}");
        let ansi = render(view, Format::Ansi, 40, None);
        assert_eq!(strip_ansi(&ansi), plain);
    }
}

#[test]
fn json_is_the_model_with_the_view_named() {
    // req:view/view-library@2bb194e8
    let g = ordered();
    let model = flight_view(&g, &[], &[], NOW, EXPIRY);
    let out = render(&View::Flight(model.clone()), Format::Json, 100, None);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v, json(&model));
    assert_eq!(v["view"], "flight");
}

#[test]
fn a_merged_ref_is_not_a_claim() {
    // req:view/view-library@2bb194e8
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
    ]);
    let merged = [clm_orchestrator::ClaimRef {
        node: "a".into(),
        tip_time: NOW - 60,
        merged: true,
    }];
    let v = json(&flight_view(&g, &[], &merged, NOW, EXPIRY));
    assert_eq!(v["nodes"][0]["state"], "ready");
    assert_eq!(v["nodes"][0]["claim"], Value::Null);
    let lapsed = [claim("a", EXPIRY + 1)];
    let v = json(&flight_view(&g, &[], &lapsed, NOW, EXPIRY));
    assert_eq!(v["nodes"][0]["state"], "ready");
}

#[test]
fn svg_is_a_document_with_the_nodes_and_their_edges() {
    // req:view/view-command@c4a4df22
    let (g, markers) = nested();
    let out = render(&spec_chart(&g, &markers), Format::Svg, 100, None);
    assert!(out.starts_with("<svg"), "{out}");
    assert!(out.trim_end().ends_with("</svg>"), "{out}");
    assert!(out.contains("check.drift"), "{out}");
    assert!(out.contains("<style>"), "{out}");
    // The edge from `check.drift` to `model`.
    assert!(out.contains("class=\"edge\""), "{out}");
    for view in [
        View::Flight(flight_view(&g, &markers, &[], NOW, EXPIRY)),
        View::Blockers(blockers_view(&g, &markers, &[], NOW, EXPIRY)),
    ] {
        let out = render(&view, Format::Svg, 100, None);
        assert!(
            out.starts_with("<svg") && out.trim_end().ends_with("</svg>"),
            "{out}"
        );
    }
}

#[test]
fn svg_escapes_text() {
    // req:view/view-command@c4a4df22
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &node("specd", "", "\"<o&o>\"", &["x"])),
    ]);
    let out = render(&spec_chart(&g, &[]), Format::Svg, 100, None);
    let doc = roxmltree::Document::parse(&out).unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(doc.root_element().tag_name().name(), "svg");
    assert!(
        doc.descendants()
            .filter_map(|n| n.text())
            .any(|t| t.contains("<o&o>")),
        "{out}"
    );
    // A character XML 1.0 forbids, here in a title, still leaves a document that parses.
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        (
            "specs/a/SPEC.md",
            &specd("", &["x"]).replace("title: T", "title: \"T\\x01\""),
        ),
    ]);
    assert!(g.nodes["a"].frontmatter.title.contains('\u{1}'));
    let out = render(&spec_chart(&g, &[]), Format::Svg, 100, None);
    roxmltree::Document::parse(&out).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    // Nor does one in the text the SVG draws: an owner and the checkout's line.
    let mut model = spec_view(&g, &[], None);
    for n in &mut model.nodes {
        n.owners.push("o\u{1}\u{b}".into());
    }
    let out = render(
        &View::Spec(model, Default::default()),
        Format::Svg,
        100,
        Some(&Head::Checkout(Checkout {
            commit: "abc\u{1}".into(),
            behind: None,
        })),
    );
    roxmltree::Document::parse(&out).unwrap_or_else(|e| panic!("{e}: {out:?}"));
    let flight = View::Flight(flight_view(&g, &[], &[], NOW, EXPIRY));
    let out = render(
        &flight,
        Format::Svg,
        100,
        Some(&Head::Checkout(Checkout {
            commit: "abc\u{1}\u{fffe}".into(),
            behind: None,
        })),
    );
    roxmltree::Document::parse(&out).unwrap_or_else(|e| panic!("{e}: {out:?}"));
}

#[test]
fn scenario_no_width() {
    // req:view/view-command@c4a4df22
    let long = "a".repeat(60);
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        (&format!("specs/{long}/SPEC.md"), &specd("", &["x"])),
    ]);
    let view = spec_chart(&g, &[]);
    for format in [Format::Text, Format::Ansi] {
        let out = strip_ansi(&render(&view, format, 0, None));
        assert!(
            out.lines().any(|l| l.contains(&format!("{long} "))),
            "{out}"
        );
        assert!(!out.contains('…'), "{out}");
    }
}

#[test]
fn scenario_colours() {
    // req:view/view-command@c4a4df22
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &node("active", "", "", &["x"])),
        ("specs/b/SPEC.md", &specd("", &["x"])),
        ("specs/c/SPEC.md", &specd("", &["x"])),
        ("backlog/q.md", &item("question", "b/x")),
    ]);
    let view = spec_with(&g, &[], &[claim("a", 1)]);
    let behind = head(Some(3));
    let ansi = render(&view, Format::Ansi, 100, Some(&behind));
    for coloured in [
        "\u{1b}[33mclaimed\u{1b}[0m",
        "\u{1b}[33m⚠ 3 behind origin/main\u{1b}[0m",
        "\u{1b}[31mblocked\u{1b}[0m",
        "\u{1b}[36mready\u{1b}[0m",
        "\u{1b}[32mactive\u{1b}[0m",
    ] {
        assert!(ansi.contains(coloured), "{coloured:?} not in {ansi}");
    }
    let flight = View::Flight(flight_view(&g, &[], &[claim("a", 1)], NOW, EXPIRY));
    let ansi = render(&flight, Format::Ansi, 100, None);
    assert!(
        ansi.lines().any(|l| l == "\u{1b}[34mcomponents\u{1b}[0m"),
        "{ansi}"
    );
    // The SVG fills `claimed` and the `⚠` part amber and `blocked` rose.
    let svg = render(&view, Format::Svg, 100, Some(&behind));
    let style = &svg[svg.find("<style>").unwrap()..svg.find("</style>").unwrap()];
    let fill = |class: &str| {
        let at = style
            .find(&format!(".{class}{{fill:"))
            .unwrap_or_else(|| panic!("no fill for {class} in {style}"));
        &style[at + class.len() + 7..at + class.len() + 14]
    };
    assert_eq!(fill("amber"), "#b7791f");
    assert_eq!(fill("rose"), "#dc3558");
    for drawn in [
        "<tspan class=\"amber\">claimed</tspan>",
        "<tspan class=\"amber\">⚠ 3 behind origin/main</tspan>",
        "<tspan class=\"rose\">blocked</tspan>",
    ] {
        assert!(svg.contains(drawn), "{drawn} not in {svg}");
    }
}

#[test]
fn svg_draws_the_bar_and_outlines_a_pending_node_in_its_state() {
    // req:view/spec-view@249ab038
    let (g, markers) = nested();
    let out = render(
        &spec_with(&g, &markers, &[claim("check.drift", 1)]),
        Format::Svg,
        100,
        None,
    );
    let rect = out
        .lines()
        .find(|l| l.starts_with("<rect class=\"node") && l.contains("claimed"))
        .unwrap_or_else(|| panic!("no claimed outline in {out}"));
    assert!(rect.contains("class=\"node claimed\""), "{rect}");
    // The label names the state, in its colour.
    assert!(
        out.contains("<tspan class=\"amber\">claimed</tspan>"),
        "{out}"
    );
    // Its bar: five cells' width of green then five of amber, then `1/2`.
    let lines: Vec<&str> = out.lines().collect();
    let amber = lines
        .iter()
        .position(|l| l.contains("class=\"seg amber\""))
        .unwrap_or_else(|| panic!("no amber segment in {out}"));
    let w = |l: &str| {
        let at = l.find("width=\"").unwrap() + 7;
        l[at..at + l[at..].find('"').unwrap()]
            .parse::<f64>()
            .unwrap()
    };
    assert!(lines[amber - 1].contains("class=\"seg green\""), "{out}");
    assert_eq!(w(lines[amber - 1]), w(lines[amber]), "{out}");
    assert!(lines[amber + 1].contains("1/2"), "{out}");
    // A converged node keeps the outline of its status.
    assert!(out.contains("<rect class=\"node active\""), "{out}");
    // Obsolete markers are counted beside the bar.
    let (g, markers) = obsolete();
    let out = render(&spec_chart(&g, &markers), Format::Svg, 100, None);
    assert!(out.contains("✕1"), "{out}");
}

#[test]
fn the_flight_bar_writes_each_pending_state_as_its_own_symbol() {
    // req:view/flight-view@86ea7638
    // req:view/view-command@c4a4df22
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/n/SPEC.md", &specd("", &["x", "y"])),
        ("changes/d/change.md", &change("implementing", "")),
        (
            "changes/d/deltas/n.md",
            "---\ncomponent: n\n---\n## REMOVED Requirements\n### Requirement: gone\n",
        ),
    ]);
    // `gone` obsolete, `x` behind, `y` unbuilt, in ID order.
    let markers = [
        marker("n/x", Some("00000000"), 1),
        marker("n/gone", None, 2),
    ];
    let out = text(&View::Flight(flight_view(&g, &markers, &[], NOW, EXPIRY)));
    let line = out
        .lines()
        .find(|l| l.starts_with("  n "))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(line.contains("✕▓░ 3 pending"), "{out}");
}

// ---- checkout ----

fn checkout(behind: Option<u64>) -> Checkout {
    Checkout {
        commit: "abc1234".into(),
        behind,
    }
}

fn head(behind: Option<u64>) -> Head {
    Head::Checkout(checkout(behind))
}

/// `origin/main` at `abc1234`, fetched `ago` seconds before `NOW` when given, on `branch`.
fn at(ago: Option<u64>, branch: Option<&str>) -> Head {
    Head::At(At {
        rev: "origin/main".into(),
        commit: "abc1234".into(),
        fetched: ago.map(|a| NOW - a),
        branch: branch.map(str::to_string),
        now: NOW,
    })
}

#[test]
fn the_commit_line_heads_every_chart_at_a_commit() {
    // req:view/view-command@c4a4df22
    let (g, markers) = nested();
    let view = View::Flight(flight_view(&g, &markers, &[], NOW, EXPIRY));
    let first = |head: Head| {
        render(&view, Format::Text, 100, Some(&head))
            .lines()
            .next()
            .unwrap()
            .to_string()
    };
    assert_eq!(
        first(at(Some(200), Some("impl/a"))),
        "origin/main abc1234 · fetched 3m ago · checkout impl/a"
    );
    assert_eq!(
        first(at(Some(30), None)),
        "origin/main abc1234 · fetched 0m ago · checkout detached"
    );
    assert_eq!(
        first(at(Some(3 * 3600 + 59 * 60), None)),
        "origin/main abc1234 · fetched 3h ago · checkout detached"
    );
    assert_eq!(
        first(at(None, None)),
        "origin/main abc1234 · checkout detached"
    );
    // A fetch later than the time it is measured to is `0m` ago.
    assert_eq!(
        first(Head::At(At {
            rev: "origin/main".into(),
            commit: "abc1234".into(),
            fetched: Some(NOW + 10),
            branch: None,
            now: NOW,
        })),
        "origin/main abc1234 · fetched 0m ago · checkout detached"
    );
    let head = at(Some(200), Some("impl/a"));
    let ansi = render(&view, Format::Ansi, 100, Some(&head));
    assert!(!ansi.lines().next().unwrap().contains('⚠'), "{ansi}");
    let svg = render(&view, Format::Svg, 100, Some(&head));
    assert!(
        svg.contains("origin/main abc1234 · fetched 3m ago · checkout impl/a"),
        "{svg}"
    );
    let json: Value = serde_json::from_str(&render(&view, Format::Json, 100, Some(&head))).unwrap();
    assert_eq!(json.get("checkout"), None, "{json:#}");
    assert_eq!(json["at"]["rev"], "origin/main");
    assert_eq!(json["at"]["commit"], "abc1234");
    assert_eq!(json["at"]["fetched"], NOW - 200);
    assert_eq!(json["at"]["branch"], "impl/a");
    assert_eq!(json["at"].get("now"), None, "{json:#}");
    let json: Value =
        serde_json::from_str(&render(&view, Format::Json, 100, Some(&at(None, None)))).unwrap();
    assert_eq!(json["at"].get("fetched"), None, "{json:#}");
    assert_eq!(json["at"].get("branch"), None, "{json:#}");
}

#[test]
fn the_checkout_line_heads_every_chart() {
    // req:view/view-command@c4a4df22
    let (g, markers) = nested();
    let views = [
        spec_chart(&g, &markers),
        View::Flight(flight_view(&g, &markers, &[], NOW, EXPIRY)),
        View::Blockers(blockers_view(&g, &markers, &[], NOW, EXPIRY)),
    ];
    for view in &views {
        let behind = head(Some(3));
        let plain = render(view, Format::Text, 100, Some(&behind));
        assert_eq!(
            plain.lines().next(),
            Some("commit abc1234  ⚠ 3 behind origin/main"),
            "{plain}"
        );
        // The rest is the chart as it was.
        let bare = render(view, Format::Text, 100, None);
        assert_eq!(plain.split_once('\n').unwrap().1, bare);
        let ansi = render(view, Format::Ansi, 100, Some(&behind));
        assert_eq!(strip_ansi(&ansi), plain);
        assert!(
            ansi.starts_with("commit abc1234  \u{1b}[33m⚠ 3 behind origin/main\u{1b}[0m\n"),
            "{ansi}"
        );
        let svg = render(view, Format::Svg, 100, Some(&behind));
        assert!(svg.contains("commit abc1234"), "{svg}");
        assert!(
            svg.contains("<tspan class=\"amber\">⚠ 3 behind origin/main</tspan>"),
            "{svg}"
        );
        for current in [Some(0), None] {
            let text = render(view, Format::Text, 100, Some(&head(current)));
            assert_eq!(text.lines().next(), Some("commit abc1234"), "{text}");
        }
        // Cut to the width like any other line.
        let narrow = render(view, Format::Text, 20, Some(&behind));
        assert_eq!(
            narrow.lines().next(),
            Some("commit abc1234  ⚠ 3…"),
            "{narrow}"
        );
    }
}

#[test]
fn the_spec_svg_puts_the_checkout_above_the_boxes() {
    // req:view/view-command@c4a4df22
    let (g, markers) = nested();
    let view = spec_chart(&g, &markers);
    let svg = render(&view, Format::Svg, 100, Some(&head(Some(1))));
    let first_text = svg.find("<text").unwrap();
    let first_box = svg.find("<rect class=\"node").unwrap();
    assert!(first_text < first_box, "{svg}");
    assert!(
        svg[first_text..].starts_with("<text x=\"10\" y=\"24.4\""),
        "{svg}"
    );
    let height = |s: &str| {
        let at = s.find("height=\"").unwrap() + 8;
        s[at..at + s[at..].find('"').unwrap()]
            .parse::<f64>()
            .unwrap()
    };
    let bare = render(&view, Format::Svg, 100, None);
    assert_eq!(height(&svg), height(&bare) + 18.0, "{svg}");
}

#[test]
fn the_json_carries_the_checkout_beside_the_model() {
    // req:view/view-library@2bb194e8
    // req:view/view-command@c4a4df22
    let g = ordered();
    let model = flight_view(&g, &[], &[], NOW, EXPIRY);
    let view = View::Flight(model.clone());
    let v: Value =
        serde_json::from_str(&render(&view, Format::Json, 100, Some(&head(Some(3))))).unwrap();
    assert_eq!(
        v["checkout"],
        serde_json::json!({"commit": "abc1234", "behind": 3})
    );
    let mut without = v.clone();
    without.as_object_mut().unwrap().remove("checkout");
    assert_eq!(without, json(&model));
    let v: Value =
        serde_json::from_str(&render(&view, Format::Json, 100, Some(&head(None)))).unwrap();
    assert_eq!(v["checkout"], serde_json::json!({"commit": "abc1234"}));
    let v: Value = serde_json::from_str(&render(&view, Format::Json, 100, None)).unwrap();
    assert_eq!(v.get("checkout"), None, "{v:#}");
    // The models themselves carry none.
    assert_eq!(json(&model).get("checkout"), None);
}

#[test]
fn scenario_checkout_behind_on_the_chart() {
    // req:view/view-command@c4a4df22
    let (g, markers) = nested();
    let flight = View::Flight(flight_view(&g, &markers, &[], NOW, EXPIRY));
    let behind = head(Some(3));
    let chart = render(&flight, Format::Text, 100, Some(&behind));
    let first = chart.lines().next().unwrap();
    assert!(first.starts_with("commit "), "{chart}");
    assert!(first.ends_with("⚠ 3 behind origin/main"), "{chart}");
    let svg = render(&flight, Format::Svg, 100, Some(&behind));
    assert!(svg.contains("3 behind origin/main"), "{svg}");
    let v: Value =
        serde_json::from_str(&render(&flight, Format::Json, 100, Some(&behind))).unwrap();
    assert_eq!(v["checkout"]["behind"], 3, "{v:#}");
    assert_eq!(v["checkout"]["commit"], "abc1234", "{v:#}");
    let spec = spec_chart(&g, &markers);
    let v: Value =
        serde_json::from_str(&render(&spec, Format::Json, 100, Some(&head(Some(0))))).unwrap();
    assert_eq!(v["checkout"]["behind"], 0, "{v:#}");
}

#[test]
fn scenario_the_model_carries_no_checkout() {
    // req:view/view-library@2bb194e8
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
    ]);
    let model = flight_view(&g, &[], &[claim("a", 1)], NOW, EXPIRY);
    let bare = json(&model);
    assert_eq!(bare["nodes"][0]["state"], "claimed", "{bare:#}");
    assert_eq!(bare.get("checkout"), None, "{bare:#}");
    let out = render(
        &View::Flight(model),
        Format::Json,
        100,
        Some(&head(Some(2))),
    );
    let rendered: Value = serde_json::from_str(&out).unwrap();
    let mut expected = bare.clone();
    expected
        .as_object_mut()
        .unwrap()
        .insert("checkout".into(), rendered["checkout"].clone());
    assert_eq!(rendered, expected);
    assert_eq!(
        rendered["checkout"],
        serde_json::json!({"commit": "abc1234", "behind": 2})
    );
}

#[test]
fn status_names_the_checkout_after_its_title() {
    // req:view/status-report@cddca121
    let g = ordered();
    let status = |c: Option<&Checkout>| status_text(&g, &[], &[], &[], NOW, None, c);
    let behind = status(Some(&checkout(Some(2))));
    let lines: Vec<&str> = behind.lines().collect();
    assert_eq!(lines[0], "CLM status");
    assert_eq!(lines[1], "  commit abc1234  ⚠ 2 behind origin/main");
    assert!(lines[2].starts_with("  components "), "{behind}");
    assert_eq!(
        status(Some(&checkout(Some(1)))).lines().nth(1),
        Some("  commit abc1234  ⚠ 1 behind origin/main")
    );
    assert_eq!(
        status(Some(&checkout(Some(0)))).lines().nth(1),
        Some("  commit abc1234")
    );
    assert_eq!(
        status(Some(&checkout(None))).lines().nth(1),
        Some("  commit abc1234")
    );
    assert!(
        status(None)
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("  components ")
    );
}

// ---- pinned line shapes: spec and blockers ----

/// How many steps `node` is below the root, walking its parents.
fn depth(view: &clm_view::SpecView, node: &str) -> usize {
    let parent = |id: &str| {
        view.nodes
            .iter()
            .find(|n| n.node == id)
            .and_then(|n| n.parent.clone())
    };
    std::iter::successors(parent(node), |p| parent(p)).count()
}

#[test]
fn scenario_spec_lines() {
    // req:view/spec-component-lines@fcbd78c9
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/a/c/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd("", &[])),
    ]);
    let view = spec_with(&g, &[], &[claim("a", 1)]);
    let c = head(Some(2));
    let out = strip_ansi(&render(&view, Format::Ansi, 100, Some(&c)));
    assert_eq!(out, render(&view, Format::Text, 100, Some(&c)));
    let lines = lines(&out);
    assert!(lines[0].starts_with("commit "), "{out}");
    let rest = &lines[1..];
    assert_eq!(rest.len(), 4, "{out}");
    for (line, start) in rest.iter().zip(["clm ", "├─ a ", "│  └─ a.c ", "└─ b "]) {
        assert!(line.starts_with(start), "{line:?} in {out}");
    }
    // The nth line is the model's nth node, its prefix three columns per level.
    let model = spec_view(&g, &[], None);
    assert_eq!(model.nodes.len(), rest.len());
    for (line, n) in rest.iter().zip(&model.nodes) {
        let cols = 3 * depth(&model, &n.node);
        let prefix: String = line.chars().take(cols).collect();
        assert!(
            prefix
                .chars()
                .all(|c| matches!(c, '│' | '├' | '└' | '─' | ' ')),
            "{line:?}"
        );
        assert!(
            line[prefix.len()..].starts_with(&format!("{} ", n.node)),
            "{line:?}"
        );
    }
    // Without a checkout there is no first line to skip.
    let bare = render(&view, Format::Text, 100, None);
    assert_eq!(bare.lines().collect::<Vec<_>>(), rest, "{bare}");
    // With `--at`, the line naming the commit takes the checkout line's place.
    let at_out = strip_ansi(&render(&view, Format::Ansi, 100, Some(&at(Some(60), None))));
    let at_lines: Vec<&str> = at_out.lines().collect();
    assert!(at_lines[0].starts_with("origin/main "), "{at_out}");
    assert_eq!(&at_lines[1..], rest, "{at_out}");
}

#[test]
fn scenario_spec_long_id() {
    // req:view/spec-component-lines@fcbd78c9
    // Distinct letters, so a cut taken from anywhere but the ID's start shows.
    let long = "abcdefghij".repeat(6);
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        (&format!("specs/{long}/SPEC.md"), &specd("", &["x"])),
        ("specs/b/SPEC.md", &specd(&long, &["x"])),
    ]);
    let view = spec_chart(&g, &[]);
    for format in [Format::Text, Format::Ansi] {
        let out = strip_ansi(&render(&view, format, 40, None));
        for line in out.lines() {
            assert!(line.chars().count() <= 40, "{line:?} in {out}");
        }
        let line = out.lines().nth(1).expect(&out);
        let rest = line
            .strip_prefix("├─ ")
            .or_else(|| line.strip_prefix("└─ "))
            .expect(&out);
        let cut = rest.strip_suffix('…').expect(&out);
        // The cut takes every column the width leaves: 40 less the prefix and `…`.
        assert_eq!(cut, &long[..36], "{out}");
        assert!(out.lines().nth(2).unwrap().starts_with("└─ b "), "{out}");
    }
}

/// The lines of `out` after the line `blockers`.
fn after_blockers(out: &str) -> Vec<&str> {
    let lines: Vec<&str> = out.lines().collect();
    let at = lines.iter().position(|l| *l == "blockers").expect(out);
    lines[at + 1..].to_vec()
}

#[test]
fn scenario_root_lines() {
    // req:view/blockers-root-lines@31794d8d
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/b/SPEC.md", &needs(&[("y", &["a/x"])])),
        ("specs/c/SPEC.md", &needs(&[("x", &["b/y"])])),
        ("specs/d/SPEC.md", &specd("", &["x"])),
        ("backlog/q.md", &item("question", "d/x")),
    ]);
    let view = View::Blockers(blockers_view(&g, &[], &[claim("a", 1)], NOW, EXPIRY));
    let c = head(None);
    let out = strip_ansi(&render(&view, Format::Ansi, 100, Some(&c)));
    assert_eq!(out, render(&view, Format::Text, 100, Some(&c)));
    let rest = after_blockers(&out);
    assert_eq!(rest.len(), 5, "{out}");
    assert!(rest[0].starts_with("backlog/q "), "{out}");
    assert!(rest[1].starts_with('└') && rest[1].ends_with(" d"), "{out}");
    assert!(rest[2].starts_with("a "), "{out}");
    // A root's cells, after its name: its state, `waits on you` when it waits on the human,
    // then its weight, at least two spaces apart.
    let cells = |line: &str| -> Vec<String> {
        line.split("  ")
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(String::from)
            .collect()
    };
    assert_eq!(
        cells(rest[0]),
        ["backlog/q", "question", "waits on you", "holds up 1"],
        "{out}"
    );
    assert_eq!(cells(rest[2]), ["a", "claimed", "holds up 2"], "{out}");
    // `c` reaches `a` directly (`b/y` is blocked), so both hang off `a`.
    assert_eq!(rest[3..], ["├─ b", "└─ c"], "{out}");
}

#[test]
fn blockers_root_line_is_cut_from_the_first_column() {
    // req:view/blockers-root-lines@31794d8d
    let long = "abcdefghij".repeat(6);
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        (&format!("specs/{long}/SPEC.md"), &specd("", &["x"])),
        ("specs/b/SPEC.md", &needs(&[("x", &[&format!("{long}/x")])])),
    ]);
    let view = View::Blockers(blockers_view(&g, &[], &[claim(&long, 1)], NOW, EXPIRY));
    for format in [Format::Text, Format::Ansi] {
        let out = strip_ansi(&render(&view, format, 40, None));
        let rest = after_blockers(&out);
        assert_eq!(rest.len(), 2, "{out}");
        let cut = rest[0].strip_suffix('…').expect(&out);
        assert_eq!(cut, &long[..39], "{out}");
        assert!(rest[1].starts_with('└'), "{out}");
    }
}

#[test]
fn scenario_blockers_nothing_blocked_line() {
    // req:view/blockers-root-lines@31794d8d
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
    ]);
    let view = View::Blockers(blockers_view(&g, &[], &[], NOW, EXPIRY));
    for format in [Format::Text, Format::Ansi] {
        let out = strip_ansi(&render(&view, format, 100, Some(&head(Some(1)))));
        assert_eq!(after_blockers(&out), ["nothing is blocked"], "{out}");
    }
}

// ---- node detail ----

/// Node `a` whose Contract reads `Alpha does X.`, with requirements `x` (current) and `y`;
/// an `applied` change `20260101-c` adding both, an archived change and a `done` one with
/// deltas for `a`; items `q` (question, `Ask`) and `b` (bug, `Fix`) naming `a`, and `o`
/// naming only `b`.
fn detailed() -> (clm_model::Graph, Vec<clm_coverage::Marker>) {
    let a = "---\ntitle: A\nstatus: active\nowners: []\ndepends_on: []\n---\n\n\
             ## Contract\nAlpha does X.\n\n\n## Design\nd\n\n## Requirements\n\n\
             ### Requirement: x\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n\
             ### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- ok\n";
    let item = |kind: &str, title: &str, nodes: &str| {
        format!("---\ntitle: {title}\nkind: {kind}\ncomponents: [{nodes}]\n---\n")
    };
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", a),
        ("specs/b/SPEC.md", &specd("", &["z"])),
        ("changes/20260101-c/change.md", &change("applied", "")),
        ("changes/20260101-c/deltas/a.md", &delta("a", &["x", "y"])),
        (
            "changes/archive/20250101-z/change.md",
            &change("applied", ""),
        ),
        (
            "changes/archive/20250101-z/deltas/a.md",
            &delta("a", &["x"]),
        ),
        ("changes/20260102-d/change.md", &change("done", "")),
        ("changes/20260102-d/deltas/a.md", &delta("a", &["y"])),
        ("backlog/q.md", &item("question", "Ask", "a")),
        ("backlog/b.md", &item("bug", "Fix", "a")),
        ("backlog/o.md", &item("idea", "Other", "b")),
    ]);
    let markers = vec![current(&g, "a/x")];
    (g, markers)
}

fn spec_node(v: &Value, id: &str) -> Value {
    v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node"] == id)
        .unwrap_or_else(|| panic!("no {id} in {v:#}"))
        .clone()
}

#[test]
fn scenario_node_detail() {
    // req:view/component-detail@d0431adf
    let (g, markers) = detailed();
    let a = spec_node(&json(&spec_view(&g, &markers, None)), "a");
    assert_eq!(a["contract"], "Alpha does X.");
    assert_eq!(
        a["changes"],
        serde_json::json!([{
            "change": "20260101-c",
            "status": "applied",
            "current": 1,
            "total": 2,
            "complete": false,
        }])
    );
    assert_eq!(
        a["backlog"],
        serde_json::json!([
            { "slug": "b", "kind": "bug", "title": "Fix" },
            { "slug": "q", "kind": "question", "title": "Ask" },
        ])
    );
}

#[test]
fn scenario_nothing_touches_it() {
    // req:view/component-detail@d0431adf
    let (g, markers) = detailed();
    let v = json(&spec_view(&g, &markers, None));
    let root = spec_node(&v, "clm");
    assert_eq!(root["changes"], serde_json::json!([]));
    assert_eq!(root["backlog"], serde_json::json!([]));
    assert_eq!(root["contract"], "c");
    // `b` is named only by `o`.
    let b = spec_node(&v, "b");
    assert_eq!(b["changes"], serde_json::json!([]));
    assert_eq!(b["backlog"][0]["slug"], "o");
}

#[test]
fn node_detail_leaves_out_abandoned_changes_and_sorts_by_id() {
    // req:view/component-detail@d0431adf
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x", "y"])),
        ("changes/f/change.md", &change("proposed", "")),
        ("changes/f/deltas/a.md", &delta("a", &["y"])),
        ("changes/c/change.md", &change("implementing", "")),
        ("changes/c/deltas/a.md", &delta("a", &["x"])),
        ("changes/e/change.md", &change("abandoned", "")),
        ("changes/e/deltas/a.md", &delta("a", &["x"])),
    ]);
    let a = spec_node(&json(&spec_view(&g, &[], None)), "a");
    let ids: Vec<(&str, &str)> = a["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["change"].as_str().unwrap(), c["status"].as_str().unwrap()))
        .collect();
    assert_eq!(ids, [("c", "implementing"), ("f", "proposed")], "{a:#}");
}

#[test]
fn node_detail_names_a_change_once_however_many_deltas_name_the_node() {
    // req:view/component-detail@d0431adf
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x", "y"])),
        ("changes/c/change.md", &change("proposed", "")),
        ("changes/c/deltas/a.md", &delta("a", &["x"])),
        ("changes/c/deltas/a-more.md", &delta("a", &["y"])),
    ]);
    let a = spec_node(&json(&spec_view(&g, &[], None)), "a");
    assert_eq!(a["changes"].as_array().unwrap().len(), 1, "{a:#}");
}

#[test]
fn node_backlog_goes_by_kind_before_slug() {
    // req:view/component-detail@d0431adf
    let item = |kind: &str| format!("---\ntitle: T\nkind: {kind}\ncomponents: [a, a]\n---\n");
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("backlog/a-human.md", &item("human-task")),
        ("backlog/b-question.md", &item("question")),
        ("backlog/c-idea.md", &item("idea")),
        ("backlog/d-deferred.md", &item("deferred")),
        ("backlog/e-bug.md", &item("bug")),
        ("backlog/f-bug.md", &item("bug")),
    ]);
    let a = spec_node(&json(&spec_view(&g, &[], None)), "a");
    let slugs: Vec<&str> = a["backlog"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["slug"].as_str().unwrap())
        .collect();
    assert_eq!(
        slugs,
        [
            "e-bug",
            "f-bug",
            "d-deferred",
            "c-idea",
            "b-question",
            "a-human"
        ]
    );
}

#[test]
fn scenario_charts_unchanged() {
    // req:view/component-detail@d0431adf
    let (g, markers) = detailed();
    let view = spec_chart(&g, &markers);
    for format in [Format::Text, Format::Ansi, Format::Svg] {
        let out = render(&view, format, 100, None);
        for absent in ["Alpha does X.", "20260101-c", "backlog/q", "Ask", "Fix"] {
            assert!(!out.contains(absent), "{absent} in {out}");
        }
    }
}

#[test]
fn a_human_task_item_waits_on_the_human_and_comes_first() {
    // req:view/blockers-view@e774ca77
    let g = graph(&[
        ("specs/SPEC.md", &specd("", &[])),
        ("specs/a/SPEC.md", &specd("", &["x"])),
        ("specs/d/SPEC.md", &specd("", &["x"])),
        ("specs/e/SPEC.md", &needs(&[("x", &["d/x"])])),
        ("specs/f/SPEC.md", &needs(&[("x", &["d/x"])])),
        ("backlog/t.md", &item("human-task", "a/x")),
    ]);
    let v = json(&blockers_view(&g, &[], &[claim("d", 1)], NOW, EXPIRY));
    let roots = v["roots"].as_array().unwrap();
    assert_eq!(roots[0]["root"], "backlog/t", "{v:#}");
    assert_eq!(roots[0]["state"], "human-task");
    assert_eq!(roots[0]["waits_on_human"], true);
    assert_eq!(roots[1]["root"], "d");
    assert_eq!(roots[1]["weight"], 2);
}
