//! The crates that judge a branch read only git, and never what `clm health`
//! reports. Both are read from the workspace's dependency graph, as
//! `cargo metadata` gives it, counting normal and build dependencies and
//! leaving out dev-dependencies.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::{Value, json};

/// The crates whose verdicts no `clm health` finding may reach.
const HEALTH_JUDGES: [&str; 3] = ["clm-check", "clm-gate", "clm-land"];

/// The crates that judge a branch.
const JUDGES: [&str; 7] = [
    "clm-model",
    "clm-coverage",
    "clm-check",
    "clm-apply",
    "clm-gate",
    "clm-ledger",
    "clm-land",
];

/// HTTP clients and database crates a judge must not reach.
const NOT_GIT: [&str; 10] = [
    "reqwest", "ureq", "hyper", "isahc", "curl", "rusqlite", "sqlx", "diesel", "redis", "postgres",
];

/// The workspace's dependency graph, from `cargo metadata`, read once.
fn workspace_metadata() -> &'static Value {
    static METADATA: OnceLock<Value> = OnceLock::new();
    METADATA.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = Command::new(env!("CARGO"))
            .args(["metadata", "--format-version", "1", "--locked"])
            .current_dir(&root)
            .output()
            .expect("cargo metadata runs");
        assert!(
            out.status.success(),
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("cargo metadata prints JSON")
    })
}

/// The resolved graph by package id, since two versions of a crate share a
/// name: each id's name, and the ids of its dependencies that are normal or
/// build dependencies on some target. One that is only a dev-dependency is
/// left out.
struct Graph {
    names: HashMap<String, String>,
    deps: HashMap<String, Vec<String>>,
}

fn graph(meta: &Value) -> Graph {
    let names = meta["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .map(|p| (text(&p["id"]), text(&p["name"])))
        .collect();
    let deps = meta["resolve"]["nodes"]
        .as_array()
        .expect("resolve.nodes")
        .iter()
        .map(|node| {
            let deps = node["deps"]
                .as_array()
                .expect("deps")
                .iter()
                .filter(|dep| {
                    let kinds = dep["dep_kinds"].as_array().expect("dep_kinds");
                    assert!(!kinds.is_empty(), "no dep_kinds: {dep}");
                    kinds.iter().any(|k| k["kind"].as_str() != Some("dev"))
                })
                .map(|dep| text(&dep["pkg"]))
                .collect();
            (text(&node["id"]), deps)
        })
        .collect();
    Graph { names, deps }
}

fn text(value: &Value) -> String {
    value.as_str().expect("a string").to_string()
}

/// The shortest path of package names from a package named `from` to a
/// package named in `forbidden`, following normal and build dependencies
/// only, or `None` when none is reached. Panics when no package is named
/// `from`, so a renamed judge can't pass unchecked.
fn path_in(graph: &Graph, from: &str, forbidden: &[&str]) -> Option<Vec<String>> {
    let starts: Vec<&str> = graph
        .names
        .iter()
        .filter(|(_, name)| *name == from)
        .map(|(id, _)| id.as_str())
        .collect();
    assert!(!starts.is_empty(), "no package {from} in the graph");
    let mut came_from: HashMap<&str, &str> = HashMap::new();
    let mut seen: HashSet<&str> = starts.iter().copied().collect();
    let mut queue: VecDeque<&str> = starts.into_iter().collect();
    while let Some(id) = queue.pop_front() {
        let name = graph.names[id].as_str();
        if name != from && forbidden.contains(&name) {
            let mut path = vec![name.to_string()];
            let mut at = id;
            while let Some(prev) = came_from.get(at) {
                path.push(graph.names[*prev].clone());
                at = prev;
            }
            path.reverse();
            return Some(path);
        }
        for next in graph.deps.get(id).into_iter().flatten() {
            if seen.insert(next) {
                came_from.insert(next, id);
                queue.push_back(next);
            }
        }
    }
    None
}

fn path_to(meta: &Value, from: &str, forbidden: &[&str]) -> Option<Vec<String>> {
    path_in(&graph(meta), from, forbidden)
}

/// One line per judge that reaches a forbidden package:
/// `<judge> reaches <package>: <judge> -> ... -> <package>`.
fn violations(meta: &Value, judges: &[&str], forbidden: &[&str]) -> Vec<String> {
    let graph = graph(meta);
    judges
        .iter()
        .filter_map(|judge| {
            let path = path_in(&graph, judge, forbidden)?;
            Some(format!(
                "{judge} reaches {}: {}",
                path.last().unwrap(),
                path.join(" -> ")
            ))
        })
        .collect()
}

/// A metadata fixture: each edge is `(from, to, kinds)`, a kind being `null`
/// for a normal dependency, `"build"` or `"dev"`, optionally with a target.
/// A package written `name@version` is that version of `name`; any other is
/// version `1.0.0`.
fn fixture(edges: &[(&str, &str, Value)]) -> Value {
    let mut names: Vec<&str> = edges.iter().flat_map(|(a, b, _)| [*a, *b]).collect();
    names.sort_unstable();
    names.dedup();
    let id = |n: &str| match n.contains('@') {
        true => format!("registry+fixture#{n}"),
        false => format!("registry+fixture#{n}@1.0.0"),
    };
    let name = |n: &str| n.split('@').next().unwrap().to_string();
    let packages: Vec<Value> = names
        .iter()
        .map(|n| json!({ "name": name(n), "id": id(n) }))
        .collect();
    let nodes: Vec<Value> = names
        .iter()
        .map(|n| {
            let deps: Vec<Value> = edges
                .iter()
                .filter(|(a, _, _)| a == n)
                .map(|(_, b, kinds)| {
                    json!({ "name": name(b).replace('-', "_"), "pkg": id(b), "dep_kinds": kinds })
                })
                .collect();
            json!({ "id": id(n), "deps": deps })
        })
        .collect();
    json!({ "packages": packages, "resolve": { "nodes": nodes } })
}

fn normal() -> Value {
    json!([{ "kind": null, "target": null }])
}

fn kind(k: &str) -> Value {
    json!([{ "kind": k, "target": null }])
}

// req:clm/health-isolated@82b4324b
#[test]
fn health_reached_through_a_dependency_fails() {
    let meta = fixture(&[
        ("clm-gate", "clm-mid", normal()),
        ("clm-mid", "clm-health", normal()),
        ("clm-check", "clm-model", normal()),
        ("clm-land", "clm-model", normal()),
    ]);
    assert_eq!(
        violations(&meta, &HEALTH_JUDGES, &["clm-health"]),
        ["clm-gate reaches clm-health: clm-gate -> clm-mid -> clm-health"]
    );
}

// req:clm/health-isolated@82b4324b
#[test]
fn dev_dependencies_do_not_count() {
    let meta = fixture(&[
        ("clm-gate", "clm-health", kind("dev")),
        ("clm-gate", "clm-mid", normal()),
        ("clm-mid", "clm-health", kind("dev")),
    ]);
    assert_eq!(path_to(&meta, "clm-gate", &["clm-health"]), None);
}

// req:clm/health-isolated@82b4324b
#[test]
fn build_target_and_mixed_dependencies_count() {
    let build = fixture(&[("clm-land", "clm-health", kind("build"))]);
    assert!(path_to(&build, "clm-land", &["clm-health"]).is_some());
    let target = fixture(&[(
        "clm-land",
        "clm-health",
        json!([{ "kind": null, "target": "cfg(unix)" }]),
    )]);
    assert!(path_to(&target, "clm-land", &["clm-health"]).is_some());
    let mixed = fixture(&[(
        "clm-land",
        "clm-health",
        json!([{ "kind": "dev", "target": null }, { "kind": null, "target": null }]),
    )]);
    assert!(path_to(&mixed, "clm-land", &["clm-health"]).is_some());
}

// req:clm/health-isolated@82b4324b
#[test]
fn each_version_of_a_crate_keeps_its_own_dependencies() {
    let meta = fixture(&[
        ("clm-gate", "shared@1", normal()),
        ("shared@1", "clm-health", normal()),
        ("clm-ledger", "shared@2", normal()),
    ]);
    assert_eq!(
        path_to(&meta, "clm-gate", &["clm-health"]),
        Some(vec![
            "clm-gate".into(),
            "shared".into(),
            "clm-health".into()
        ])
    );
}

// req:clm/health-isolated@82b4324b
#[test]
#[should_panic(expected = "dep_kinds")]
fn a_dependency_without_kinds_fails() {
    let meta = fixture(&[("clm-gate", "clm-health", json!([]))]);
    path_to(&meta, "clm-gate", &["clm-health"]);
}

// req:clm/health-isolated@82b4324b
#[test]
#[should_panic(expected = "clm-gone")]
fn a_judge_missing_from_the_graph_fails() {
    let meta = fixture(&[("clm-gate", "clm-mid", normal())]);
    path_to(&meta, "clm-gone", &["clm-health"]);
}

// req:clm/health-isolated@82b4324b
#[test]
fn todays_workspace_keeps_health_out_of_the_judges() {
    let found = violations(workspace_metadata(), &HEALTH_JUDGES, &["clm-health"]);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

// req:clm/judges-read-only-git@3f09968f
#[test]
fn judge_reaching_an_http_client_fails() {
    let meta = fixture(&[
        ("clm-land", "ureq", normal()),
        ("clm-gate", "hyper-util", normal()),
        ("clm-model", "serde", normal()),
        ("clm-coverage", "clm-model", normal()),
        ("clm-check", "clm-coverage", normal()),
        ("clm-apply", "clm-model", normal()),
        ("clm-ledger", "clm-model", normal()),
    ]);
    assert_eq!(
        violations(&meta, &JUDGES, &NOT_GIT),
        ["clm-land reaches ureq: clm-land -> ureq"]
    );
}

// req:clm/judges-read-only-git@3f09968f
#[test]
fn todays_workspace_judges_reach_no_http_client_or_database() {
    let found = violations(workspace_metadata(), &JUDGES, &NOT_GIT);
    assert!(found.is_empty(), "{}", found.join("\n"));
}
