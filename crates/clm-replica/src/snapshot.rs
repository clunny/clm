//! The snapshot: what the views show for one commit of the default branch, built with the
//! same model and coverage code as `clm status`, `clm coverage` and `clm backlog`
//! (`replica/snapshot-matches-clm`, `replica/snapshot-size`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_check::{Budgets, Coverage as Cited, Length, Marker};
use clm_model::{Files, Graph};
use serde::Serialize;
use serde_json::Value;

use crate::{State, regular};

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum Coverage {
    Complete,
    /// The marker index is still being built; `done` is the share scanned.
    Indexing {
        done: f64,
    },
}

#[derive(Serialize)]
struct Snapshot {
    sha: String,
    coverage: Coverage,
    /// `checked` when the `spec` model's components carry `oversized`, else why not:
    /// `indexing` or `clm.toml: <error>`.
    sizes: String,
    spec: Value,
    nodes: Vec<NodeView>,
    changes: Vec<ChangeView>,
    backlog: Vec<BacklogView>,
    /// Every slug a change, under `changes/` or `changes/archive/`, names in `resolves`.
    resolved: Vec<String>,
    decisions: Vec<DecisionView>,
    violations: Vec<String>,
}

#[derive(Serialize)]
struct NodeView {
    id: String,
    title: String,
    status: String,
    parent: Option<String>,
    depends_on: Vec<String>,
    contract: String,
    requirements: Vec<RequirementView>,
    assets: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct RequirementView {
    id: String,
    text: String,
    version: String,
    state: Option<String>,
    markers: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ChangeView {
    id: String,
    status: String,
    from: Option<String>,
    resolves: Vec<String>,
    nodes: Vec<String>,
    current: Option<usize>,
    total: usize,
    complete: Option<bool>,
}

#[derive(Serialize)]
struct BacklogView {
    slug: String,
    kind: String,
    title: String,
    nodes: Vec<String>,
    origin: Option<String>,
    blocks: Vec<String>,
}

#[derive(Serialize)]
struct DecisionView {
    name: String,
    title: String,
    date: String,
    status: String,
}

fn lower(d: &dyn std::fmt::Debug) -> String {
    format!("{d:?}").to_lowercase()
}

/// The IDs of the components over their budgets, as `clm size` would mark them on a
/// checkout of the commit, or why sizes can't be checked.
fn oversized(
    state: &State,
    graph: &Graph,
    files: &Files,
    markers: &[Marker],
) -> Result<BTreeSet<String>, String> {
    if !state.index_complete() {
        return Err("indexing".into());
    }
    let budgets = match files.get(Path::new("clm.toml")) {
        None => clm_config::NodeSize::default(),
        Some(bytes) => std::str::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .and_then(|text| clm_config::node_size_from(text).map_err(|e| e.message))
            .map_err(|e| format!("clm.toml: {e}"))?,
    };
    let attributes: Vec<(&str, &str)> = files
        .iter()
        .filter_map(|(path, bytes)| {
            let path = path.to_str()?;
            let dir = match path {
                ".gitattributes" => "",
                _ => path.strip_suffix("/.gitattributes")?,
            };
            Some((dir, std::str::from_utf8(bytes).ok()?))
        })
        .collect();
    let binary = |path: &str| state.held.get(path).is_some_and(|h| h.binary);
    // Regular files only: a symlink or a submodule never counts.
    let sizes: BTreeMap<&str, u64> = state
        .tree
        .iter()
        .filter(|(_, b)| regular(&b.mode))
        .map(|(p, b)| (p.as_str(), b.size))
        .collect();
    let mut lengths: BTreeMap<PathBuf, Length> = sizes
        .iter()
        .filter(|(path, _)| !binary(path) && !clm_check::linguist_excluded(&attributes, path))
        .map(|(path, size)| (PathBuf::from(path), Length::from(*size)))
        .collect();
    // `clm size` counts each component's `SPEC.md` whatever its attributes say.
    for node in graph.nodes.values() {
        let size = node.path.to_str().and_then(|p| sizes.get(p));
        if let Some(size) = size {
            lengths.insert(node.path.clone(), Length::from(*size));
        }
    }
    let defaults = Budgets {
        spec_tokens: budgets.spec_tokens,
        code_tokens: budgets.code_tokens,
    };
    Ok(clm_check::size_of(graph, &lengths, markers, defaults)
        .into_iter()
        .filter(|s| s.oversized())
        .map(|s| s.node)
        .collect())
}

/// The snapshot of the tip, as a JSON value.
pub(crate) fn build(state: &State, graph: &Graph, violations: &[String], files: &Files) -> Value {
    let complete = state.index_complete();
    // An incomplete index would give partial counts, so until it completes the views get
    // the model's shape with nothing cited.
    let markers = if complete {
        state.markers()
    } else {
        Vec::new()
    };
    let coverage = clm_check::report(graph, &markers);
    let cited: BTreeMap<&str, &Cited> = coverage.iter().map(|c| (c.id.as_str(), c)).collect();
    let oversized = oversized(state, graph, files, &markers);
    let spec = clm_view::spec_view(graph, &markers, oversized.as_ref().ok());
    let snapshot = Snapshot {
        sha: state.tip.clone().unwrap_or_default(),
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Indexing {
                done: state.index_done(),
            }
        },
        sizes: oversized.err().unwrap_or_else(|| "checked".into()),
        spec: serde_json::to_value(spec).unwrap_or(Value::Null),
        nodes: nodes(graph, complete.then_some(&cited)),
        changes: changes(graph, &markers, complete),
        backlog: backlog(graph),
        resolved: graph
            .changes
            .values()
            .flat_map(|c| c.frontmatter.resolves.iter().cloned())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect(),
        decisions: graph
            .decisions
            .values()
            .map(|d| DecisionView {
                name: d.name.clone(),
                title: d.frontmatter.title.clone(),
                date: d.frontmatter.date.clone(),
                status: lower(&d.frontmatter.status),
            })
            .collect(),
        violations: violations.to_vec(),
    };
    serde_json::to_value(snapshot).unwrap_or(Value::Null)
}

fn nodes(graph: &Graph, cited: Option<&BTreeMap<&str, &Cited>>) -> Vec<NodeView> {
    graph
        .nodes
        .values()
        .map(|n| NodeView {
            id: n.id.clone(),
            title: n.frontmatter.title.clone(),
            status: lower(&n.frontmatter.status),
            parent: n.parent.clone(),
            depends_on: n.frontmatter.depends_on.clone(),
            contract: n.contract.clone(),
            requirements: n
                .requirements
                .iter()
                .map(|r| {
                    let c = cited.and_then(|c| c.get(r.id.as_str()));
                    RequirementView {
                        id: r.id.clone(),
                        text: r.text.clone(),
                        version: r.hash[..8].to_string(),
                        state: c.map(|c| c.state.to_string()),
                        markers: c.map(|c| c.markers.iter().map(ToString::to_string).collect()),
                    }
                })
                .collect(),
            assets: n.assets.clone(),
        })
        .collect()
}

/// Each change outside the archive with its progress exactly as `clm status` gives it.
fn changes(graph: &Graph, markers: &[Marker], complete: bool) -> Vec<ChangeView> {
    clm_check::change_progress(graph, markers)
        .into_iter()
        .filter_map(|p| {
            let c = graph.changes.get(&p.change)?;
            Some(ChangeView {
                id: c.id.clone(),
                status: lower(&c.frontmatter.status),
                from: c.frontmatter.from.clone(),
                resolves: c.frontmatter.resolves.clone(),
                nodes: c.deltas.iter().map(|d| d.node.clone()).collect(),
                current: complete.then_some(p.current),
                total: p.total,
                complete: complete.then_some(p.complete),
            })
        })
        .collect()
}

/// In `clm backlog`'s order.
fn backlog(graph: &Graph) -> Vec<BacklogView> {
    let mut items: Vec<_> = graph.backlog.values().collect();
    items.sort_by(|a, b| (a.frontmatter.kind, &a.slug).cmp(&(b.frontmatter.kind, &b.slug)));
    items
        .into_iter()
        .map(|i| BacklogView {
            slug: i.slug.clone(),
            kind: i.frontmatter.kind.to_string(),
            title: i.frontmatter.title.clone(),
            nodes: i.frontmatter.nodes.clone(),
            origin: i.frontmatter.origin.clone(),
            blocks: i.frontmatter.blocks.clone(),
        })
        .collect()
}
