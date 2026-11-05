//! Applies a change's deltas to `specs/` (see specs/apply/SPEC.md).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use clm_model::outline::{requirement_spans, section_spans};
use clm_model::{
    BacklogKind, Change, Delta, DeltaRequirement, Graph, Layer, Node, ROOT_ID, Violation,
    check_node_frontmatter, split_frontmatter,
};

/// `(element, hash)` for every element of `node`, in `clm hashes` order.
pub fn hashes(node: &Node) -> Vec<(String, String)> {
    let slug = |id: &str| id.rsplit_once('/').map_or(id, |(_, s)| s).to_string();
    [
        ("frontmatter".into(), node.hashes.frontmatter.clone()),
        ("contract".into(), node.hashes.contract.clone()),
        ("design".into(), node.hashes.design.clone()),
    ]
    .into_iter()
    .chain(
        node.requirements
            .iter()
            .map(|r| (format!("requirement:{}", slug(&r.id)), r.hash.clone())),
    )
    .chain(
        node.assets
            .iter()
            .map(|(path, hash)| (format!("asset:{path}"), hash.clone())),
    )
    .collect()
}

/// `(element, hash)` for every line `clm hashes <layer>` prints: the layer's node's
/// elements first, when it loads (the capability node), then each entry in file order,
/// named with the word its heading uses (`hash-command`).
pub fn layer_hashes(graph: &Graph, layer: Layer) -> Vec<(String, String)> {
    let word = match layer {
        Layer::Architecture => "requirement",
        Layer::Capability => "capability",
    };
    let node = match layer {
        Layer::Capability => graph.capability_node().map(hashes).unwrap_or_default(),
        Layer::Architecture => Vec::new(),
    };
    let entries = graph.layers.get(&layer).into_iter().flatten().map(|e| {
        let slug = e.id.split_once('/').map_or("", |(_, s)| s);
        (format!("{word}:{slug}"), e.hash.clone())
    });
    node.into_iter().chain(entries).collect()
}

/// Every reason `change` cannot be applied to `graph`.
pub fn validate(graph: &Graph, change: &Change) -> Vec<Violation> {
    plan(graph, change, Vec::new()).err().unwrap_or_default()
}

/// Applies change `id` in the repo at `root`, all or nothing. Returns the files written and
/// removed.
pub fn apply(root: &Path, id: &str) -> Result<Vec<FileOp>, Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    let Some(change) = graph.changes.get(id) else {
        let mut refused = vec![violation(
            Path::new("changes"),
            "apply/change-status",
            format!("no change `{id}`"),
        )];
        refused.extend(violations);
        return Err(refused);
    };
    let ops = plan(&graph, change, violations)?;
    for op in &ops {
        op.perform(root)
            .map_err(|e| vec![io_violation(op.path(), e)])?;
    }
    Ok(ops)
}

/// One file `apply` writes or removes, by repo-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOp {
    Write(PathBuf, Vec<u8>),
    Remove(PathBuf),
}

impl FileOp {
    pub fn path(&self) -> &Path {
        match self {
            FileOp::Write(path, _) | FileOp::Remove(path) => path,
        }
    }

    /// Writes (creating directories) or removes the file under `root`.
    pub fn perform(&self, root: &Path) -> std::io::Result<()> {
        let full = root.join(self.path());
        match self {
            FileOp::Write(_, bytes) => {
                if let Some(dir) = full.parent() {
                    fs::create_dir_all(dir)?;
                }
                fs::write(&full, bytes)
            }
            FileOp::Remove(_) => {
                fs::remove_file(&full)?;
                // Drop the directories it empties below `specs/`, as `clm sync`'s regenerated
                // tree has none: a removed node's directory goes once nothing else is in it.
                for dir in self.path().ancestors().skip(1) {
                    if dir.parent().is_none_or(|p| p.as_os_str().is_empty())
                        || fs::remove_dir(root.join(dir)).is_err()
                    {
                        break;
                    }
                }
                Ok(())
            }
        }
    }
}

/// Moves a done or abandoned change to `changes/archive/<id>/`. Returns the new directory.
pub fn archive(root: &Path, id: &str) -> Result<PathBuf, Vec<Violation>> {
    use clm_model::ChangeStatus::{Abandoned, Done};
    let (graph, _) = clm_model::load(root);
    let refuse = |message: String| {
        Err(vec![violation(
            Path::new("changes").join(id).as_path(),
            "apply/archive-command",
            message,
        )])
    };
    let Some(change) = graph.changes.get(id).filter(|c| !c.archived) else {
        return refuse(format!("no unarchived change `{id}`"));
    };
    if !matches!(change.frontmatter.status, Done | Abandoned) {
        return refuse(format!(
            "change `{id}` is {:?}; only done or abandoned changes can be archived",
            change.frontmatter.status
        ));
    }
    let to = PathBuf::from("changes")
        .join(clm_model::ARCHIVE_DIR)
        .join(id);
    if root.join(&to).exists() {
        return refuse(format!("{} already exists", to.display()));
    }
    fs::create_dir_all(root.join("changes").join(clm_model::ARCHIVE_DIR))
        .and_then(|_| fs::rename(root.join("changes").join(id), root.join(&to)))
        .map_err(|e| vec![io_violation(&to, e)])?;
    Ok(to)
}

fn violation(path: &Path, rule: &'static str, message: String) -> Violation {
    Violation {
        path: path.to_path_buf(),
        rule,
        message,
    }
}

fn io_violation(path: &Path, e: std::io::Error) -> Violation {
    violation(path, "apply/atomic", format!("write failed: {e}"))
}

/// The `specs/` files an already applied change writes or removes on `graph`, or every
/// violation: `apply` without its status check, leaving `change.md` as it is. `clm sync`
/// uses it to rebuild `specs/` on a newer `main` (`regenerate-applied-specs`).
pub fn reapply(graph: &Graph, change: &Change) -> Result<Vec<FileOp>, Vec<Violation>> {
    let Writes {
        ops, violations, ..
    } = node_writes(graph, change);
    if violations.is_empty() {
        Ok(ops)
    } else {
        Err(violations)
    }
}

/// Every file the change writes or removes, or every violation, the loaded graph's
/// (`loaded`) among them as `loaded-graph` counts them.
fn plan(
    graph: &Graph,
    change: &Change,
    loaded: Vec<Violation>,
) -> Result<Vec<FileOp>, Vec<Violation>> {
    let mut violations = Vec::new();
    if !change.frontmatter.status.is_pending() {
        violations.push(violation(
            &change.path,
            "apply/change-status",
            format!(
                "change `{}` is {:?}; only proposed or approved changes can be applied",
                change.id, change.frontmatter.status
            ),
        ));
    }
    let Writes {
        mut ops,
        specs,
        violations: errs,
    } = node_writes(graph, change);
    // Every delta applies, so `resulting` gives the graph the change leaves.
    if let (true, Some(after)) = (errs.is_empty(), resulting(graph, change)) {
        let refused = !violations.is_empty();
        let (current, found) = before_and_after(graph, after, &specs);
        violations.extend(resulting_graph(change, &current, &found));
        // What a change that can't apply would fix can't be judged (`loaded-graph`).
        if refused {
            violations.extend(loaded);
        } else {
            violations.extend(left_behind(loaded, &current, &found));
        }
    } else {
        // What a change that can't apply would fix can't be judged, so every loaded violation
        // stays.
        violations.extend(errs);
        violations.extend(loaded);
    }
    if violations.is_empty() {
        // `answered-questions`: the spec this applies answers the questions it resolves.
        ops.extend(
            change
                .frontmatter
                .resolves
                .iter()
                .filter_map(|slug| graph.backlog.get(slug))
                .filter(|item| item.frontmatter.kind == clm_model::BacklogKind::Question)
                .map(|item| FileOp::Remove(item.path.clone())),
        );
        ops.push(FileOp::Write(
            change.path.clone(),
            mark_applied(&change.source).into_bytes(),
        ));
        Ok(ops)
    } else {
        Err(violations)
    }
}

/// What the change's deltas do to `specs/`.
struct Writes {
    /// The node files and assets written or removed.
    ops: Vec<FileOp>,
    /// Each node the change writes, by ID, with its new `SPEC.md` path and text, or `None`
    /// when the change removes it.
    specs: BTreeMap<String, Option<(PathBuf, String)>>,
    /// The violations of the deltas that can't be applied.
    violations: Vec<Violation>,
}

/// `loaded-graph`: the violations of the loaded graph (`loaded`) that the change doesn't
/// fix. One the graph check or the parse of a node the change writes reports now
/// (`current`) is fixed unless the graph it leaves (`found`) still has it.
fn left_behind(
    loaded: Vec<Violation>,
    current: &[Violation],
    found: &[Violation],
) -> Vec<Violation> {
    let mut fixed = counts(current);
    let mut kept = counts(found);
    loaded
        .into_iter()
        .filter(|v| !take(&mut fixed, v) || take(&mut kept, v))
        .collect()
}

/// `resulting-graph`: the violations of the rules across nodes, and of the node parser,
/// in the graph the change leaves (`found`) that the graph has no copy of now (`current`).
fn resulting_graph(change: &Change, current: &[Violation], found: &[Violation]) -> Vec<Violation> {
    // Counted, so a second copy of a violation the graph has once is new.
    let mut known = counts(current);
    found
        .iter()
        .filter(|v| !take(&mut known, v))
        .map(|v| {
            violation(
                &v.path,
                "apply/resulting-graph",
                format!(
                    "change `{}` would leave: {}: {}",
                    change.id, v.rule, v.message
                ),
            )
        })
        .collect()
}

type Key = (PathBuf, &'static str, String);

fn key(v: &Violation) -> Key {
    (v.path.clone(), v.rule, v.message.clone())
}

/// How many copies of each violation `violations` holds.
fn counts(violations: &[Violation]) -> BTreeMap<Key, usize> {
    let mut counts = BTreeMap::new();
    for v in violations {
        *counts.entry(key(v)).or_default() += 1;
    }
    counts
}

/// Whether `counts` holds a copy of `v` not yet taken, taking it.
fn take(counts: &mut BTreeMap<Key, usize>, v: &Violation) -> bool {
    match counts.get_mut(&key(v)) {
        Some(n) if *n > 0 => {
            *n -= 1;
            true
        }
        _ => false,
    }
}

/// The violations of the graph check and of the nodes the change writes (`specs`), before
/// the change (`graph`) and after it (`after`, which `resulting` built, checked without its
/// workaround flags: `resulting-graph`).
fn before_and_after(
    graph: &Graph,
    mut after: Graph,
    specs: &BTreeMap<String, Option<(PathBuf, String)>>,
) -> (Vec<Violation>, Vec<Violation>) {
    let declared = graph.declared_frontmatter_keys();
    let mut current = clm_model::check_graph(graph);
    let mut found = Vec::new();
    for (id, spec) in specs {
        if let Some(node) = graph.nodes.get(id) {
            current.extend(clm_model::parse_node(id, &node.path, &node.source, &declared).1);
        }
        if let Some((path, text)) = spec {
            found.extend(clm_model::parse_node(id, path, text, &declared).1);
        }
    }
    // A flag holds the version a build built, which only the current graph gives
    // (`workaround-flags`): the graph the change leaves is checked without them.
    after
        .backlog
        .retain(|_, item| item.frontmatter.kind != BacklogKind::Friction);
    found.extend(clm_model::check_graph(&after));
    (current, found)
}

/// `resulting-graph-call`: the spec graph the change would leave, or `None` when a delta
/// of it can't be applied, whatever its status: each written node parsed from its new
/// text at its path, left out when the parser yields none; each removed node gone; the
/// rest as in `graph`. Writes nothing.
pub fn resulting(graph: &Graph, change: &Change) -> Option<Graph> {
    let Writes {
        specs, violations, ..
    } = node_writes(graph, change);
    if !violations.is_empty() {
        return None;
    }
    let declared = graph.declared_frontmatter_keys();
    let mut after = Graph {
        nodes: graph.nodes.clone(),
        changes: graph.changes.clone(),
        backlog: graph.backlog.clone(),
        decisions: graph.decisions.clone(),
        layers: graph.layers.clone(),
        carried: graph.carried.clone(),
    };
    for (id, spec) in specs {
        after.nodes.remove(&id);
        if let Some((path, text)) = spec {
            let node = clm_model::parse_node(&id, &path, &text, &declared).0;
            after.nodes.extend(node.map(|n| (id, n)));
        }
    }
    Some(after)
}

/// The node files and assets the change's deltas write or remove, and the violations of
/// those that can't.
fn node_writes(graph: &Graph, change: &Change) -> Writes {
    let mut violations = Vec::new();
    let mut writes = Vec::new();
    let mut specs = BTreeMap::new();
    let created: BTreeSet<&str> = change
        .deltas
        .iter()
        .filter(|d| d.create)
        .map(|d| d.node.as_str())
        .collect();
    let removed: BTreeSet<&str> = change
        .deltas
        .iter()
        .filter(|d| d.remove)
        .map(|d| d.node.as_str())
        .collect();
    let declared = graph.declared_frontmatter_keys();
    for delta in &change.deltas {
        let mut errs = Vec::new();
        let written = |(path, text)| (path, Some(text));
        let result = if delta.create {
            create(graph, &created, &declared, delta, &mut errs).map(written)
        } else if delta.remove {
            remove(graph, &created, &removed, delta, &mut errs).map(|path| (path, None))
        } else {
            modify(graph, &declared, delta, &mut errs).map(written)
        };
        let assets = asset_ops(graph, change, delta, &mut errs);
        match (result, errs.is_empty()) {
            // A removed node's `SPEC.md` goes last, so a failed write never leaves its assets
            // without a node.
            (Some((path, None)), true) => {
                writes.extend(assets);
                writes.push(FileOp::Remove(path));
                specs.insert(delta.node.clone(), None);
            }
            (Some((path, Some(text))), true) => {
                writes.push(FileOp::Write(path.clone(), text.clone().into_bytes()));
                writes.extend(assets);
                specs.insert(delta.node.clone(), Some((path, text)));
            }
            _ => violations.extend(errs),
        }
    }
    Writes {
        ops: writes,
        specs,
        violations,
    }
}

/// `apply-assets`: the delta's node's assets the change writes, and those its
/// `REMOVED Assets` deletes, each checked against its recorded base.
fn asset_ops(
    graph: &Graph,
    change: &Change,
    delta: &Delta,
    errs: &mut Vec<Violation>,
) -> Vec<FileOp> {
    let written = change.assets.get(&delta.node);
    let invalid = |message: String| violation(&delta.path, "apply/apply-assets", message);
    if delta.node == ROOT_ID && (written.is_some() || !delta.removed_assets.is_empty()) {
        errs.push(invalid("the root component has no assets".into()));
        return Vec::new();
    }
    let none = BTreeMap::new();
    // A node created by this change has no assets yet; a missing node is reported already.
    let current = match graph.nodes.get(&delta.node) {
        Some(node) if !delta.create => &node.assets,
        _ => &none,
    };
    let check = |errs: &mut Vec<Violation>, path: &str, base: Option<&str>| match current.get(path)
    {
        Some(hash) => check_base(delta, errs, &format!("asset `{path}`"), base, hash, true),
        // A recorded base for an asset that's gone means `main` removed it since.
        None if base.is_some() => errs.push(violation(
            &delta.path,
            "apply/stale-base",
            format!(
                "asset `{path}` of `{}` no longer exists; rebase the delta",
                delta.node
            ),
        )),
        None => {}
    };
    let dir = spec_path(&delta.node).with_file_name("assets");
    let mut ops = Vec::new();
    for (path, bytes) in written.into_iter().flatten() {
        check(errs, path, delta.base.assets.get(path).map(String::as_str));
        ops.push(FileOp::Write(dir.join(path), bytes.clone()));
    }
    let mut removed = BTreeSet::new();
    for (path, base) in &delta.removed_assets {
        // Removal always records a base, so a missing asset is reported as stale.
        check(errs, path, Some(base));
        if written.is_some_and(|w| w.contains_key(path)) || !removed.insert(path) {
            errs.push(invalid(format!(
                "asset `{path}` of `{}` is removed and also written or removed again",
                delta.node
            )));
        }
        ops.push(FileOp::Remove(dir.join(path)));
    }
    ops
}

fn mark_applied(source: &str) -> String {
    let mut in_frontmatter = false;
    let mut done = false;
    source
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if line == "---" {
                in_frontmatter = i == 0;
            } else if in_frontmatter && !done && line.starts_with("status:") {
                done = true;
                return "status: applied\n".to_string();
            }
            format!("{line}\n")
        })
        .collect()
}

fn spec_path(node: &str) -> PathBuf {
    if node == ROOT_ID {
        PathBuf::from("specs/SPEC.md")
    } else {
        PathBuf::from("specs")
            .join(node.replace('.', "/"))
            .join("SPEC.md")
    }
}

fn parent_of(node: &str) -> &str {
    node.rsplit_once('.').map_or(ROOT_ID, |(p, _)| p)
}

fn create(
    graph: &Graph,
    created: &BTreeSet<&str>,
    declared: &BTreeSet<String>,
    delta: &Delta,
    errs: &mut Vec<Violation>,
) -> Option<(PathBuf, String)> {
    let err = |errs: &mut Vec<Violation>, msg: String| {
        errs.push(violation(&delta.path, "apply/create-node", msg))
    };
    if graph.nodes.contains_key(&delta.node) {
        err(errs, format!("component `{}` already exists", delta.node));
    }
    let parent = parent_of(&delta.node);
    if !graph.nodes.contains_key(parent) && !created.contains(parent) {
        err(
            errs,
            format!(
                "parent `{parent}` of `{}` doesn't exist and isn't created by this change",
                delta.node
            ),
        );
    }
    let Some(fm) = &delta.frontmatter else {
        err(
            errs,
            "`create: true` needs a `## Frontmatter` section".into(),
        );
        return None;
    };
    if let Err(msg) = frontmatter_errors(declared, delta, fm) {
        err(errs, msg);
    }

    let section = |t: &Option<String>| t.as_deref().unwrap_or("").trim().to_string();
    let mut text = format!(
        "---\n{}\n---\n\n## Contract\n{}\n\n## Design\n{}\n\n## Requirements\n",
        fm.trim_end(),
        section(&delta.contract),
        section(&delta.design)
    );
    for r in &delta.added {
        text.push('\n');
        text.push_str(r.text.trim_end());
        text.push('\n');
    }
    Some((spec_path(&delta.node), text))
}

/// `remove-node`: whether the node's `SPEC.md` can go: the delta lists everything the
/// node holds with current bases and no child of the node outlives it. `asset_ops` removes
/// the assets it lists.
fn remove(
    graph: &Graph,
    created: &BTreeSet<&str>,
    removed: &BTreeSet<&str>,
    delta: &Delta,
    errs: &mut Vec<Violation>,
) -> Option<PathBuf> {
    let id = delta.node.as_str();
    let err = |errs: &mut Vec<Violation>, msg: String| {
        errs.push(violation(&delta.path, "apply/remove-node", msg))
    };
    if id == ROOT_ID {
        err(errs, format!("the root component `{id}` can't be removed"));
        return None;
    }
    let Some(node) = graph.nodes.get(id) else {
        // As for a modified node: `main` removed it since the delta was written.
        errs.push(violation(
            &delta.path,
            "apply/stale-base",
            format!("component `{id}` no longer exists"),
        ));
        return None;
    };
    for (element, base, current) in [
        (
            "frontmatter",
            &delta.base.frontmatter,
            &node.hashes.frontmatter,
        ),
        ("contract", &delta.base.contract, &node.hashes.contract),
        ("design", &delta.base.design, &node.hashes.design),
    ] {
        check_base(delta, errs, element, base.as_deref(), current, true);
    }
    check_requirements(node, delta, errs);
    for r in &node.requirements {
        let slug = r.id.rsplit_once('/').map_or(r.id.as_str(), |(_, s)| s);
        if !delta.removed.iter().any(|d| d.slug == slug) {
            err(
                errs,
                format!(
                    "removing `{id}` leaves requirement `{}`; list it under `REMOVED Requirements`",
                    r.id
                ),
            );
        }
    }
    for path in node.assets.keys() {
        if !delta.removed_assets.iter().any(|(p, _)| p == path) {
            err(
                errs,
                format!("removing `{id}` leaves asset `{path}`; list it under `REMOVED Assets`"),
            );
        }
    }
    for child in graph.nodes.values() {
        if child.parent.as_deref() == Some(id) && !removed.contains(child.id.as_str()) {
            err(
                errs,
                format!(
                    "removing `{id}` leaves its child `{}`; remove it in the same change",
                    child.id
                ),
            );
        }
    }
    for child in created.iter().filter(|c| parent_of(c) == id) {
        err(
            errs,
            format!("this change removes `{id}` and creates its child `{child}`"),
        );
    }
    Some(node.path.clone())
}

/// `requirements`: every MODIFIED or REMOVED requirement exists and records its current
/// base. A MODIFIED one without its own `#### Serves` keeps the rule's, so only its version
/// can go stale (`stale-base`).
fn check_requirements(node: &Node, delta: &Delta, errs: &mut Vec<Violation>) {
    let modified = delta.modified.iter().map(|r| (r, true));
    for (r, is_modified) in modified.chain(delta.removed.iter().map(|r| (r, false))) {
        let id = format!("{}/{}", node.id, r.slug);
        let Some(cur) = node.requirements.iter().find(|cur| cur.id == id) else {
            errs.push(violation(
                &delta.path,
                "apply/requirements",
                format!("requirement `{id}` doesn't exist"),
            ));
            continue;
        };
        let base = effective_base(cur, r, is_modified);
        check_base(
            delta,
            errs,
            &format!("requirement `{id}`"),
            base,
            &cur.hash,
            true,
        );
    }
}

/// The base `stale-base` compares with the rule's hash: a MODIFIED requirement without its
/// own `#### Serves` keeps the rule's, so only its version can go stale, and a base of
/// another length isn't a hash, so it stays stale.
fn effective_base<'a>(
    cur: &'a clm_model::Requirement,
    r: &'a DeltaRequirement,
    is_modified: bool,
) -> Option<&'a str> {
    match r.base.as_deref() {
        Some(b)
            if is_modified
                && !carries_serves(r)
                && b.len() == cur.hash.len()
                && b.get(..8) == Some(cur.version()) =>
        {
            Some(cur.hash.as_str())
        }
        b => b,
    }
}

/// `stale-base`: whether a recorded base differs from the element's current hash; `validate`
/// and `stale_bases` both judge through it (`stale-elements`).
fn is_stale(base: &str, current: &str) -> bool {
    base != current
}

/// `stale-elements`: each element `delta` modifies or removes whose recorded base is stale
/// against `node`, as `(element, base)`: those `node` holds in `clm hashes` order, then the
/// rest in the delta's order. Frontmatter and asset bases are left out.
pub fn stale_bases(node: Option<&Node>, delta: &Delta) -> Vec<(String, String)> {
    let touches = delta.remove;
    let mut held = Vec::new();
    let mut lacking = Vec::new();
    let mut section = |name: &str, base: &Option<String>, touched: bool, cur: Option<&str>| {
        let Some(b) = base.as_deref().filter(|_| touched) else {
            return;
        };
        if cur.is_none_or(|c| is_stale(b, c)) {
            let out = if cur.is_some() {
                &mut held
            } else {
                &mut lacking
            };
            out.push((name.to_string(), b.to_string()));
        }
    };
    section(
        "contract",
        &delta.base.contract,
        touches || delta.modified_contract.is_some(),
        node.map(|n| n.hashes.contract.as_str()),
    );
    section(
        "design",
        &delta.base.design,
        touches || delta.modified_design.is_some(),
        node.map(|n| n.hashes.design.as_str()),
    );
    let mut reqs = Vec::new();
    let modified = delta.modified.iter().map(|r| (r, true));
    for (r, is_modified) in modified.chain(delta.removed.iter().map(|r| (r, false))) {
        let Some(b) = r.base.as_deref() else { continue };
        let cur = node.and_then(|n| {
            let id = format!("{}/{}", n.id, r.slug);
            n.requirements.iter().find(|c| c.id == id)
        });
        let stale =
            cur.is_none_or(|c| is_stale(effective_base(c, r, is_modified).unwrap_or(b), &c.hash));
        if stale {
            reqs.push((
                format!("requirement:{}", r.slug),
                b.to_string(),
                cur.is_some(),
            ));
        }
    }
    // `clm hashes` order is the node's own requirement order.
    if let Some(n) = node {
        for c in &n.requirements {
            let slug = c.id.rsplit_once('/').map_or(c.id.as_str(), |(_, s)| s);
            for (e, b, h) in &reqs {
                if *h && e.strip_prefix("requirement:") == Some(slug) {
                    held.push((e.clone(), b.clone()));
                }
            }
        }
    }
    lacking.extend(reqs.into_iter().filter(|r| !r.2).map(|(e, b, _)| (e, b)));
    held.extend(lacking);
    held
}

fn modify(
    graph: &Graph,
    declared: &BTreeSet<String>,
    delta: &Delta,
    errs: &mut Vec<Violation>,
) -> Option<(PathBuf, String)> {
    let Some(node) = graph.nodes.get(&delta.node) else {
        errs.push(violation(
            &delta.path,
            "apply/stale-base",
            format!("component `{}` no longer exists", delta.node),
        ));
        return None;
    };
    check_base(
        delta,
        errs,
        "frontmatter",
        delta.base.frontmatter.as_deref(),
        &node.hashes.frontmatter,
        delta.modified_frontmatter.is_some(),
    );
    check_base(
        delta,
        errs,
        "contract",
        delta.base.contract.as_deref(),
        &node.hashes.contract,
        delta.modified_contract.is_some(),
    );
    check_base(
        delta,
        errs,
        "design",
        delta.base.design.as_deref(),
        &node.hashes.design,
        delta.modified_design.is_some(),
    );

    let current = |slug: &str| {
        let id = format!("{}/{slug}", node.id);
        node.requirements.iter().find(|r| r.id == id)
    };
    for r in &delta.added {
        if current(&r.slug).is_some() {
            errs.push(violation(
                &delta.path,
                "apply/requirements",
                format!("ADDED requirement `{}/{}` already exists", node.id, r.slug),
            ));
        }
    }
    check_requirements(node, delta, errs);

    let (fm, body) = split_frontmatter(&node.source)?;
    let fm = match &delta.modified_frontmatter {
        Some(yaml) => match edit_frontmatter(fm, yaml)
            .and_then(|fm| frontmatter_errors(declared, delta, &fm).map(|()| fm))
        {
            Ok(fm) => fm,
            Err(msg) => {
                errs.push(violation(&delta.path, "apply/frontmatter", msg));
                return None;
            }
        },
        None => fm.to_string(),
    };
    let body = edit_body(body, delta, errs, &node.id)?;
    Some((node.path.clone(), format!("---\n{fm}\n---\n{body}")))
}

fn check_base(
    delta: &Delta,
    errs: &mut Vec<Violation>,
    element: &str,
    base: Option<&str>,
    current: &str,
    required: bool,
) {
    let message = match base {
        Some(b) if is_stale(b, current) => format!(
            "{element} of `{}` is stale: delta was written against {b}, current is {current}; rebase the delta",
            delta.node
        ),
        None if required => format!(
            "{element} of `{}` is changed without a recorded base; record the hash from `clm hashes {}`",
            delta.node, delta.node
        ),
        _ => return,
    };
    errs.push(violation(&delta.path, "apply/stale-base", message));
}

/// Replaces (or appends) each top-level key line from `yaml`, leaving other lines untouched.
fn edit_frontmatter(fm: &str, yaml: &str) -> Result<String, String> {
    let mut lines: Vec<String> = fm.lines().map(String::from).collect();
    for line in yaml
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
    {
        let key = match line.split_once(':') {
            Some((k, _)) if !k.is_empty() && !k.starts_with([' ', '\t', '-']) => k,
            _ => {
                return Err(format!(
                    "frontmatter values must be single-line `key: value`, got `{line}`"
                ));
            }
        };
        let prefix = format!("{key}:");
        match lines.iter().position(|l| l.starts_with(&prefix)) {
            Some(i) => {
                if lines
                    .get(i + 1)
                    .is_some_and(|n| n.starts_with([' ', '\t', '-']))
                {
                    return Err(format!(
                        "current `{key}` spans more than one line; make it single-line first"
                    ));
                }
                lines[i] = line.to_string();
            }
            None => lines.push(line.to_string()),
        }
    }
    Ok(lines.join("\n"))
}

/// `declared-frontmatter-keys`: the frontmatter `fm` that `delta` produces, checked as the
/// model checks a loaded node's, by node ID so the root rules run too, accepting
/// `declared`, the keys live changes declare, this one among them. The error names the
/// node and says why.
fn frontmatter_errors(declared: &BTreeSet<String>, delta: &Delta, fm: &str) -> Result<(), String> {
    let errors = match check_node_frontmatter(&delta.node, &delta.path, fm, declared) {
        Ok((_, violations)) if violations.is_empty() => return Ok(()),
        Ok((_, violations)) | Err(violations) => violations,
    };
    let why: Vec<String> = errors.into_iter().map(|v| v.message).collect();
    Err(format!(
        "frontmatter of `{}` is invalid: {}",
        delta.node,
        why.join("; ")
    ))
}

fn push_block(out: &mut Vec<String>, text: &str) {
    out.extend(text.trim_matches('\n').lines().map(String::from));
    out.push(String::new());
}

fn edit_body(body: &str, delta: &Delta, errs: &mut Vec<Violation>, node: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();
    let sections = section_spans(&lines);
    let mut out: Vec<String> = Vec::new();
    let copy = |out: &mut Vec<String>, range: std::ops::Range<usize>| {
        out.extend(lines[range].iter().map(|l| l.to_string()))
    };

    for (name, text) in [
        ("Contract", &delta.modified_contract),
        ("Design", &delta.modified_design),
    ] {
        if text.is_some() && !sections.iter().any(|s| s.name == name) {
            errs.push(violation(
                &delta.path,
                "apply/sections",
                format!("`{node}` has no `## {name}` section"),
            ));
        }
    }

    copy(
        &mut out,
        0..sections.first().map_or(lines.len(), |s| s.heading),
    );
    let mut has_requirements = false;
    for s in &sections {
        out.push(lines[s.heading].to_string());
        match (
            s.name.as_str(),
            &delta.modified_contract,
            &delta.modified_design,
        ) {
            ("Contract", Some(t), _) | ("Design", _, Some(t)) => push_block(&mut out, t),
            ("Requirements", _, _) => {
                has_requirements = true;
                edit_requirements(&lines, s.heading + 1..s.end, delta, &mut out);
            }
            _ => copy(&mut out, s.heading + 1..s.end),
        }
    }
    if !has_requirements && !delta.added.is_empty() {
        if out.last().is_some_and(|l| !l.is_empty()) {
            out.push(String::new());
        }
        out.push("## Requirements".into());
        out.push(String::new());
        for r in &delta.added {
            push_block(&mut out, &r.text);
        }
    }

    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    Some(out.join("\n") + "\n")
}

const SERVES: &str = "#### Serves";

/// Whether a delta's requirement block has its own `#### Serves` block, which then replaces
/// the rule's and is part of its base (`requirements`, `stale-base`).
fn carries_serves(r: &DeltaRequirement) -> bool {
    let own: Vec<&str> = r.text.lines().collect();
    !serves_block(&own).is_empty()
}

/// A requirement block's `#### Serves` heading and body, as the model reads it (outside
/// fences, up to the next heading), blank lines at its end left out; empty when it has none.
fn serves_block<'a>(block: &[&'a str]) -> Vec<&'a str> {
    let Some((at, body)) = clm_model::outline::blocks_named(block, SERVES)
        .into_iter()
        .next()
    else {
        return Vec::new();
    };
    let mut kept = block[at..body.end].to_vec();
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }
    kept
}

fn edit_requirements(
    lines: &[&str],
    range: std::ops::Range<usize>,
    delta: &Delta,
    out: &mut Vec<String>,
) {
    let find =
        |list: &'_ [DeltaRequirement], slug: &str| list.iter().find(|r| r.slug == slug).cloned();
    let spans = requirement_spans(lines, range.clone());
    let preamble_end = spans.first().map_or(range.end, |r| r.lines.start);
    out.extend(
        lines[range.start..preamble_end]
            .iter()
            .map(|l| l.to_string()),
    );
    for span in &spans {
        if find(&delta.removed, &span.slug).is_some() {
            continue;
        }
        match find(&delta.modified, &span.slug) {
            Some(r) => {
                push_block(out, &r.text);
                // A delta without its own `#### Serves` keeps the rule's (`requirements`).
                if !carries_serves(&r) {
                    let kept = serves_block(&lines[span.lines.clone()]);
                    if !kept.is_empty() {
                        out.extend(kept.iter().map(|l| l.to_string()));
                        out.push(String::new());
                    }
                }
            }
            None => out.extend(lines[span.lines.clone()].iter().map(|l| l.to_string())),
        }
    }
    if !delta.added.is_empty() && out.last().is_some_and(|l| !l.is_empty()) {
        out.push(String::new());
    }
    for r in &delta.added {
        push_block(out, &r.text);
    }
}
