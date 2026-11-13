//! The spine-scoped context bundle (`spine-scoped-context`).

use std::fs;
use std::path::Path;

use clm_model::{Graph, Layer, Node, Requirement, Tier};

/// The first line of the carried section.
const CARRIED_INTRO: &str = "These are the constraints CLM carries to every project, from the `clm` that runs; `clm guide constraints` explains their tiers. Names in backticks that this section doesn't list are CLM's own decisions and constraints, kept in CLM's repository.";

/// The bundle for node `id` in the checkout at `root`, or `None` if no such node exists.
pub fn bundle(graph: &Graph, root: &Path, id: &str) -> Option<String> {
    let architecture = fs::read_to_string(root.join("ARCHITECTURE.md")).ok();
    bundle_from(graph, architecture.as_deref(), id)
}

/// The bundle for node `id`, given the text of `ARCHITECTURE.md` (`None` when there's no
/// such file), or `None` if no such node exists. Reads no files: the capabilities it serves
/// come from the graph.
pub fn bundle_from(graph: &Graph, architecture: Option<&str>, id: &str) -> Option<String> {
    let target = graph.nodes.get(id)?;
    let mut parts: Vec<String> = Vec::new();

    if let Some(architecture) = architecture {
        parts.push("# ARCHITECTURE.md".into());
        parts.push(architecture.into());
    }

    if !graph.carried.source {
        parts.push("# CLM's constraints".into());
        parts.push(CARRIED_INTRO.into());
        for entry in &graph.carried.in_force {
            let tier = match entry.tier {
                Tier::Method => "method",
                Tier::Replaceable => "replaceable",
                Tier::Default => "default",
            };
            parts.push(format!("### Constraint: {} ({tier})", entry.slug));
            parts.push(entry.statement.clone());
            if let Some(guarantee) = &entry.guarantee {
                parts.push(format!("Guarantee: {guarantee}"));
            }
        }
    }

    let served = served_capabilities(graph, target);
    if !served.is_empty() {
        parts.push("# Capabilities served".into());
        parts.extend(served.iter().map(|c| c.text.clone()));
    }

    parts.push("# Ancestors".into());
    for node in spine(graph, target) {
        parts.push(format!("## {}: {}", node.id, node.frontmatter.title));
        if node.id == target.id {
            let yaml = clm_model::split_frontmatter(&node.source).map_or("", |(fm, _)| fm);
            parts.push("### Frontmatter".into());
            parts.push(format!("```yaml\n{}\n```", yaml.trim_end()));
        }
        parts.push("### Contract".into());
        parts.push(node.contract.clone());
        parts.push("### Design".into());
        parts.push(node.design.clone());
        if node.id == target.id {
            parts.push("### Requirements".into());
            parts.extend(node.requirements.iter().map(|r| r.text.clone()));
            if !node.assets.is_empty() {
                parts.push("### Assets".into());
                parts.push(assets(node));
            }
        }
    }

    let deps = dependencies(graph, target);
    if !deps.is_empty() {
        parts.push("# Dependencies".into());
        for node in deps {
            parts.push(format!("## {}: {}", node.id, node.frontmatter.title));
            parts.push("### Contract".into());
            parts.push(node.contract.clone());
        }
    }

    let parts: Vec<String> = parts.iter().map(|p| trim(p)).collect();
    Some(parts.join("\n\n") + "\n")
}

/// The bundle's content hash, which plans record (`hash-based-staleness`).
pub fn hash(bundle: &str) -> String {
    clm_model::content_hash(bundle)
}

/// The IDs of the nodes whose specs make up `id`'s bundle: its spine from the root down,
/// then the dependencies off the spine, sorted. Empty if no such node exists.
pub fn bundle_nodes(graph: &Graph, id: &str) -> Vec<String> {
    let Some(target) = graph.nodes.get(id) else {
        return Vec::new();
    };
    spine(graph, target)
        .into_iter()
        .chain(dependencies(graph, target))
        .map(|n| n.id.clone())
        .collect()
}

/// The capability entries `node`'s requirements serve (each one's effective serves), each
/// once, in `CAPABILITIES.md`'s order. Architecture IDs add nothing: the whole of
/// `ARCHITECTURE.md` is in the bundle. The node that builds the capability tests (the
/// capability node, else the root) gets every capability (`model/capability-tests-component`).
pub fn served_capabilities<'g>(graph: &'g Graph, node: &Node) -> Vec<&'g Requirement> {
    let capabilities = graph.layers.get(&Layer::Capability).into_iter().flatten();
    if node.id == graph.capability_tests_node() {
        return capabilities.collect();
    }
    let named: Vec<&String> = node
        .requirements
        .iter()
        .flat_map(|r| graph.effective_serves(&r.id))
        .collect();
    capabilities.filter(|c| named.contains(&&c.id)).collect()
}

/// `target`'s direct dependencies, `depends_on`, that aren't on its
/// spine, sorted and deduplicated.
fn dependencies<'g>(graph: &'g Graph, target: &'g Node) -> Vec<&'g Node> {
    let on_spine: Vec<&str> = spine(graph, target).iter().map(|n| n.id.as_str()).collect();
    let mut deps: Vec<&Node> = target
        .frontmatter
        .depends_on
        .iter()
        .filter(|d| !on_spine.contains(&d.as_str()))
        .filter_map(|d| graph.nodes.get(d))
        .collect();
    deps.sort_by(|a, b| a.id.cmp(&b.id));
    deps.dedup_by(|a, b| a.id == b.id);
    deps
}

/// One line per asset, `- specs/<node path>/assets/<path> <hash>`, sorted by path. Lists
/// rather than includes them: they may be large or binary (`node-assets`).
fn assets(node: &Node) -> String {
    let dir: Vec<String> = node
        .path
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let dir = dir.join("/");
    let lines: Vec<String> = node
        .assets
        .iter()
        .map(|(path, hash)| format!("- {dir}/assets/{path} {hash}"))
        .collect();
    lines.join("\n")
}

/// Nodes from the root down to `target`.
fn spine<'g>(graph: &'g Graph, target: &'g Node) -> Vec<&'g Node> {
    let mut chain = vec![target];
    let mut current = target;
    while let Some(parent) = current.parent.as_ref().and_then(|p| graph.nodes.get(p)) {
        chain.push(parent);
        current = parent;
    }
    chain.reverse();
    chain
}

/// Trailing whitespace off every line and the whole text, LF line endings.
fn trim(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    lines.join("\n").trim_end().to_string()
}
