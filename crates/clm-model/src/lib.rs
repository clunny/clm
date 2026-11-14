//! The spec graph: node specs under `specs/` and changes under `changes/`, parsed and
//! validated (see docs/spec-format.md).

mod carried;
mod change;
mod files;
mod layers;
pub mod outline;
mod priority;
mod records;
mod renames;

pub use carried::{Carried, CarriedEntry, Tier, carried};
pub use change::{
    ARCHIVE_DIR, Bases, CHANGE_KEYS, CHANGE_STATUS_TERMS, Change, ChangeFrontmatter, ChangeStatus,
    Delta, DeltaRequirement, parse_change_frontmatter,
};
pub use files::Files;
use files::read_files;
pub use layers::Layer;
pub use priority::Priority;
pub use records::{
    BACKLOG_KIND_TERMS, BacklogFrontmatter, BacklogItem, BacklogKind, Decision,
    DecisionFrontmatter, DecisionStatus, RequirementAt, parse_backlog_item,
};
pub use renames::{Rename, RenameKind};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const ROOT_ID: &str = "clm";

/// A rule violation. `rule` is the ID of the requirement that the rule enforces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: PathBuf,
    pub rule: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Specified, not yet built; written `unbuilt`. The word `plain-terms-renames`
    /// replaced still loads as it (`renamed-words-still-load`).
    #[serde(rename = "unbuilt", alias = "specd")]
    Specd,
    Active,
    Deprecated,
}

/// Every node status with the word CLM writes for it in a file (`term-lists`).
pub const STATUS_TERMS: &[(Status, &str)] = &[
    (Status::Specd, "unbuilt"),
    (Status::Active, "active"),
    (Status::Deprecated, "deprecated"),
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    pub title: String,
    pub status: Status,
    pub owners: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    /// The human-layer entries the node's rules serve, unless a rule's `#### Serves`
    /// block names its own (`serves`).
    #[serde(default)]
    pub serves: Vec<String>,
    #[serde(default)]
    pub successor: Option<String>,
    /// Accepted and ignored: nodes set it while review had tiers (`yes-no-review`), and a
    /// tree from before still loads. A sink, so nothing can read it.
    #[serde(default)]
    significant: change::Ignored,
    /// Accepted and ignored: a list nothing read, which `serves` replaces
    /// (`human-layers`), so a tree from before still loads. A sink, so nothing can read it.
    #[serde(default)]
    capabilities: change::Ignored,
    /// Set on this node only; `Graph::is_optional` adds the ancestors (`optional-nodes`).
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub size_budget: SizeBudget,
    /// The root's globs for the code no node owns yet; set only on the root
    /// (`outside-globs`).
    #[serde(default)]
    pub outside: Option<Vec<String>>,
    /// Globs of the node's test files, which it owns wherever they sit
    /// (`test-ownership-keys`).
    #[serde(default)]
    pub tests: Vec<String>,
    /// The rules the node's own files are temporarily exempt from (`exemptions`).
    #[serde(default)]
    pub exemptions: Vec<Exemption>,
}

/// A rule `clm check` judges code by that a component's files may be exempt from
/// (`exemptions`), written as its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExemptRule {
    UndeclaredImports,
    TestsCiteTheirOwner,
    SkippedTests,
}

/// Every exemptible rule with the word CLM writes for it in a file.
const EXEMPT_RULES: [&str; 3] = [
    "undeclared-imports",
    "tests-cite-their-owner",
    "skipped-tests",
];

/// One entry of a component's `exemptions`: some of its files exempt from one rule until
/// the backlog item `until` names is done (`exemptions`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exemption {
    pub rule: ExemptRule,
    pub files: Vec<String>,
    /// `backlog/<slug>`, as written; whether the item exists isn't judged when loading.
    pub until: String,
    #[serde(default)]
    pub why: Option<String>,
}

/// The node's own size budgets, each in place of `config`'s `node_size` when set
/// (`node-size-budgets`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeBudget {
    pub spec_tokens: Option<u64>,
    pub code_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `<node-id>/<slug>`
    pub id: String,
    pub text: String,
    pub scenarios: usize,
    pub hash: String,
    /// Requirement IDs of other nodes, from its `#### Needs` block (`requirement-needs`).
    pub needs: Vec<String>,
    /// Human-layer entry IDs, from its `#### Serves` block (`serves`); empty for an entry.
    pub serves: Vec<String>,
    /// For an architecture entry, the rules that enforce it, from its `#### Enforced by`
    /// block in block order (`human-layers`); empty for anything else.
    pub enforced_by: Vec<String>,
    /// Whether the entry holds an `#### Untested` block (`human-layers`); false for a rule.
    pub untested: bool,
    /// That block's text, trimmed (`human-layers`); `None` when the entry holds none.
    pub untested_text: Option<String>,
}

impl Requirement {
    /// The version a test marker cites, `req:<id>@<version>`: the first 8 hex digits of
    /// the hash (`versioned-markers`).
    pub fn version(&self) -> &str {
        &self.hash[..8]
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hashes {
    pub frontmatter: String,
    pub contract: String,
    pub design: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub parent: Option<String>,
    /// Repo-relative path of the SPEC.md.
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    pub contract: String,
    pub design: String,
    pub requirements: Vec<Requirement>,
    pub hashes: Hashes,
    /// The whole SPEC.md with LF line endings.
    pub source: String,
    /// Files under the node's `assets/`: path relative to it (`/`-separated) to
    /// `asset_hash`.
    pub assets: BTreeMap<String, String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Graph {
    pub nodes: BTreeMap<String, Node>,
    pub changes: BTreeMap<String, Change>,
    pub backlog: BTreeMap<String, BacklogItem>,
    pub decisions: BTreeMap<String, Decision>,
    /// The human layers' entries, each layer's in file order (`human-layers`).
    pub layers: BTreeMap<Layer, Vec<Requirement>>,
    /// CLM's carried constraints for this repository (`carried-constraints`).
    pub carried: Carried,
}

impl Graph {
    /// The requirement with ID `<node-id>/<slug>`, or the human-layer entry with ID
    /// `<layer>/<slug>`, if it exists.
    pub fn requirement(&self, id: &str) -> Option<&Requirement> {
        if Layer::of(id).is_some() {
            return self.entry(id);
        }
        let (node, _) = id.split_once('/')?;
        self.nodes
            .get(node)?
            .requirements
            .iter()
            .find(|r| r.id == id)
    }

    /// The human-layer entry with ID `<layer>/<slug>`, if it exists.
    pub fn entry(&self, id: &str) -> Option<&Requirement> {
        self.layers
            .get(&Layer::of(id)?)?
            .iter()
            .find(|r| r.id == id)
    }

    /// The heading `layer`'s file writes its entries under, such as `### Constraint:`,
    /// which `clm rename` rewrites (`human-layers`).
    pub fn heading(&self, layer: Layer) -> &'static str {
        layer.heading()
    }

    /// The top-level node `capability`, the capability layer's node, which the
    /// capabilities are placed on, if the graph holds it (`reserved-ids`). Its own
    /// requirements are empty.
    pub fn capability_node(&self) -> Option<&Node> {
        self.nodes.get(Layer::Capability.id())
    }

    /// The node that builds the capability tests: the capability node when the graph
    /// holds it, else the root, which `clm pending` places the capabilities on
    /// (`capability-tests-node`).
    pub fn capability_tests_node(&self) -> &str {
        self.capability_node().map_or(ROOT_ID, |n| n.id.as_str())
    }

    /// Every node's requirements, by node ID, then the architecture entries, then the
    /// capabilities.
    pub fn requirements(&self) -> impl Iterator<Item = &Requirement> {
        let rules = self.nodes.values().flat_map(|n| &n.requirements);
        rules.chain(self.layers.values().flatten())
    }

    /// What rule `id` serves: its own `#### Serves` block's IDs, else its node's `serves`.
    /// Empty for an entry or an ID the graph doesn't hold (`serves`).
    pub fn effective_serves(&self, id: &str) -> &[String] {
        let Some((node, _)) = id.split_once('/') else {
            return &[];
        };
        let Some(node) = self.nodes.get(node) else {
            return &[];
        };
        match node.requirements.iter().find(|r| r.id == id) {
            Some(r) if !r.serves.is_empty() => &r.serves,
            Some(_) => &node.frontmatter.serves,
            None => &[],
        }
    }

    /// Whether node `id` or one of its ancestors in the graph sets `optional: true`
    /// (`optional-nodes`). The root's flag is rejected, so it is ignored here, and an ID
    /// not in the graph is required.
    pub fn is_optional(&self, id: &str) -> bool {
        let mut node = self.nodes.get(id);
        while let Some(n) = node {
            if n.frontmatter.optional && n.id != ROOT_ID {
                return true;
            }
            node = n.parent.as_ref().and_then(|p| self.nodes.get(p));
        }
        false
    }

    /// Whether a backlog item may name node `id`: it is in the graph, or a `create: true`
    /// delta of a change outside the archive creates it (`backlog-nodes`).
    pub fn backlog_node_known(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
            || self
                .changes
                .values()
                .filter(|c| !c.archived)
                .flat_map(|c| &c.deltas)
                .any(|d| d.create && d.node == id)
    }

    /// The root's `outside` globs in the order written, absent when the root doesn't set
    /// the key (`outside-globs`).
    pub fn outside(&self) -> Option<&[String]> {
        self.nodes.get(ROOT_ID)?.frontmatter.outside.as_deref()
    }

    /// The node frontmatter keys that live changes, outside the archive and not
    /// `abandoned`, declare in `frontmatter_keys` (`declared-frontmatter-keys`).
    pub fn declared_frontmatter_keys(&self) -> BTreeSet<String> {
        self.changes
            .values()
            .filter(|c| !c.archived && c.frontmatter.status != ChangeStatus::Abandoned)
            .flat_map(|c| c.frontmatter.frontmatter_keys.iter().cloned())
            .collect()
    }
}

/// Hash of `text`, insensitive to line endings and trailing whitespace.
pub fn content_hash(text: &str) -> String {
    let normalized: Vec<&str> = text.lines().map(str::trim_end).collect();
    let normalized = normalized.join("\n");
    let digest = Sha256::digest(normalized.trim_end().as_bytes());
    hex8(&digest)
}

/// A requirement's hash: the first 8 hex digits of `content_hash` of its block less
/// its `#### Serves` block, which is its version, then the last 8 of `content_hash` of
/// its whole block. Without a `#### Serves` block it is the block's `content_hash`.
fn requirement_hash(less_serves: &str, block: &str) -> String {
    let whole = content_hash(block);
    if less_serves == block {
        return whole;
    }
    let version = content_hash(less_serves);
    format!("{}{}", &version[..8], &whole[whole.len() - 8..])
}

/// Hash of an asset: `content_hash` of its text when it is UTF-8, otherwise the first
/// 8 bytes of the SHA-256 of its bytes, in hex.
pub fn asset_hash(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => content_hash(text),
        Err(_) => hex8(&Sha256::digest(bytes)),
    }
}

fn hex8(digest: &[u8]) -> String {
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// For a path under `specs/` with an `assets` directory in it: the node directory above
/// the first such `assets`, and the path below it, `/`-separated.
fn split_asset(specs: &Path, path: &Path) -> Option<(PathBuf, String)> {
    let parts: Vec<_> = path.strip_prefix(specs).ok()?.components().collect();
    let at = parts[..parts.len().saturating_sub(1)]
        .iter()
        .position(|c| c.as_os_str() == "assets")?;
    let dir: PathBuf = specs
        .iter()
        .chain(parts[..at].iter().map(|c| c.as_os_str()))
        .collect();
    let rest: Vec<_> = parts[at + 1..]
        .iter()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    Some((dir, rest.join("/")))
}

/// Puts each file under `specs/<node path>/assets/` into its node's `assets`.
fn load_assets(
    files: &Files,
    specs: &Path,
    present: &BTreeSet<String>,
    graph: &mut Graph,
    violations: &mut Vec<Violation>,
) {
    const RULE: &str = "model/node-assets";
    for (path, bytes) in files::under(files, specs).map(|p| (p, &files[p])) {
        let Some((dir, asset)) = split_asset(specs, path) else {
            continue;
        };
        let id = node_id(specs, &dir.join("SPEC.md"));
        if id == ROOT_ID {
            violations.push(v(path, RULE, "the root component has no assets".into()));
        } else if let Some(node) = graph.nodes.get_mut(&id) {
            node.assets.insert(asset, asset_hash(bytes));
        } else if !present.contains(&id) {
            violations.push(v(
                path,
                RULE,
                format!("asset of component `{id}`, which has no SPEC.md"),
            ));
        }
    }
}

/// Loads `<root>/specs`, `changes`, `backlog`, `docs/decisions` and the root's
/// `ARCHITECTURE.md` and `CAPABILITIES.md`, returning the graph of valid nodes and every
/// violation found. The same as `load_files` over those files.
pub fn load(root: &Path) -> (Graph, Vec<Violation>) {
    let (files, mut violations) = read_files(root);
    let (graph, more) = load_files(&files);
    violations.extend(more);
    (graph, violations)
}

/// Loads the graph from repo-relative paths and their contents, ignoring paths outside
/// `specs/`, `changes/`, `backlog/`, `docs/decisions/` and the two root files of the
/// human layers.
pub fn load_files(files: &Files) -> (Graph, Vec<Violation>) {
    let mut graph = Graph::default();
    let mut violations = Vec::new();
    let specs = Path::new("specs");
    // Changes first: live ones declare node frontmatter keys (`valid-frontmatter`).
    for c in change::load_changes(files, &mut violations) {
        graph.changes.insert(c.id.clone(), c);
    }
    let declared = graph.declared_frontmatter_keys();

    let spec_files: Vec<&PathBuf> = files::under(files, specs)
        .filter(|f| f.file_name().is_some_and(|n| n == "SPEC.md"))
        .filter(|f| split_asset(specs, f).is_none())
        .collect();
    // Nodes whose SPEC.md exists, even if invalid, so they aren't also reported as missing.
    let present: BTreeSet<String> = spec_files.iter().map(|f| node_id(specs, f)).collect();

    for file in spec_files {
        let id = node_id(specs, file);
        let text = match files::text(files, file) {
            Some(Ok(t)) => t,
            Some(Err(e)) => {
                violations.push(v(
                    file,
                    "model/parse-node-specs",
                    format!("unreadable: {e}"),
                ));
                continue;
            }
            None => continue,
        };
        let (node, found) = parse_node(&id, file, &text, &declared);
        violations.extend(found);
        if let Some(node) = node {
            graph.nodes.insert(node.id.clone(), node);
        }
    }
    load_assets(files, specs, &present, &mut graph, &mut violations);

    if !present.contains(ROOT_ID) {
        violations.push(v(
            Path::new("specs/SPEC.md"),
            "model/parse-node-specs",
            "root component spec is missing".into(),
        ));
    }
    for node in graph.nodes.values() {
        if let Some(parent) = &node.parent
            && (!present.contains(parent) || Layer::reserves(parent))
            && parent != ROOT_ID
        {
            violations.push(v(
                &node.path,
                "model/parse-node-specs",
                format!("parent component `{parent}` has no SPEC.md"),
            ));
        }
    }
    graph.backlog = records::load_backlog(files, &mut violations);
    graph.layers = layers::load_layers(files, &mut violations);
    graph.carried = carried::load(files, &mut violations);
    violations.extend(check_graph(&graph));
    graph.decisions = records::load_decisions(files, &mut violations);

    (graph, violations)
}

pub(crate) fn v(path: &Path, rule: &'static str, message: String) -> Violation {
    Violation {
        path: path.to_path_buf(),
        rule,
        message,
    }
}

/// The frontmatter's `key` value, as one line of YAML, when it is present and not a
/// boolean.
fn bad_bool(fm: &serde_yaml::Value, key: &str) -> Option<String> {
    let value = fm.get(key)?;
    if value.is_bool() {
        return None;
    }
    Some(one_line(value))
}

/// What is wrong with `fm`'s `size_budget`, flattened to one line: the whole value when
/// it isn't a mapping, else its first entry that isn't `spec_tokens` or `code_tokens`
/// set to a positive integer (`model/size-budget`).
fn bad_size_budget(fm: &serde_yaml::Value) -> Option<String> {
    let value = fm.get("size_budget")?;
    let Some(map) = value.as_mapping() else {
        return Some(one_line(value));
    };
    map.iter()
        .find(|(key, count)| {
            !matches!(key.as_str(), Some("spec_tokens" | "code_tokens"))
                || !count.as_u64().is_some_and(|n| n > 0)
        })
        .map(|(key, count)| format!("{}: {}", one_line(key), one_line(count)))
}

fn one_line(value: &serde_yaml::Value) -> String {
    let text = serde_yaml::to_string(value).unwrap_or_default();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn node_id(specs: &Path, file: &Path) -> String {
    let dir = file.parent().unwrap().strip_prefix(specs).unwrap();
    let parts: Vec<_> = dir
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    if parts.is_empty() {
        ROOT_ID.into()
    } else {
        parts.join(".")
    }
}

fn parent_id(id: &str) -> Option<String> {
    match id.rsplit_once('.') {
        _ if id == ROOT_ID => None,
        Some((parent, _)) => Some(parent.into()),
        None => Some(ROOT_ID.into()),
    }
}

/// The node frontmatter keys the model knows: `Frontmatter`'s fields.
const KNOWN_KEYS: [&str; 14] = [
    "title",
    "status",
    "owners",
    "depends_on",
    "paths",
    "serves",
    "successor",
    "significant",
    "capabilities",
    "optional",
    "size_budget",
    "outside",
    "tests",
    "exemptions",
];

/// The rules `fm_text`, the YAML between a node's `---` lines, alone decides, for the file
/// at `path` (`valid-frontmatter`): the first step of `check_node_frontmatter`, private
/// so no caller skips the root rules. A top-level key the model doesn't know is rejected
/// unless `declared`, the keys live changes declare (`Graph::declared_frontmatter_keys`),
/// holds it; then it is accepted, whatever its value, and left out. Every key the model
/// knows is validated, declared or not, `outside` by its shape.
fn check_frontmatter(
    path: &Path,
    fm_text: &str,
    declared: &BTreeSet<String>,
) -> Result<Frontmatter, Vec<Violation>> {
    let invalid = |e: serde_yaml::Error| vec![v(path, "model/valid-frontmatter", e.to_string())];
    let mut fm: serde_yaml::Value = serde_yaml::from_str(fm_text).map_err(invalid)?;
    let mut errors = Vec::new();
    if let Some(bad) = bad_bool(&fm, "optional") {
        errors.push(v(
            path,
            "model/optional-nodes",
            format!("`optional` must be `true` or `false`, not `{bad}`"),
        ));
    }
    if let Some(bad) = bad_outside(&fm) {
        errors.push(v(
            path,
            OUTSIDE,
            format!("`outside` must be a list of non-empty globs, not `{bad}`"),
        ));
    }
    if let Some(bad) = bad_tests(&fm) {
        errors.push(v(
            path,
            TEST_KEYS,
            format!("`tests` must be a list of non-empty strings, not `{bad}`"),
        ));
    }
    errors.extend(
        bad_exemptions(&fm)
            .into_iter()
            .map(|m| v(path, EXEMPTIONS, m)),
    );
    if let Some(bad) = bad_size_budget(&fm) {
        errors.push(v(
            path,
            "model/size-budget",
            format!(
                "`size_budget` must map `spec_tokens` or `code_tokens` to positive integers, not `{bad}`"
            ),
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let accepted = |key: &serde_yaml::Value| {
        key.as_str()
            .is_some_and(|k| !KNOWN_KEYS.contains(&k) && declared.contains(k))
    };
    let dropped = match fm.as_mapping_mut() {
        Some(map) if map.keys().any(accepted) => {
            map.retain(|key, _| !accepted(key));
            true
        }
        _ => false,
    };
    // Parsed from text: `from_value`'s errors don't name the field. The file's own text,
    // with the accepted keys' lines blanked when that leaves what was kept, so errors
    // point at its lines; else the kept mapping written out again.
    let text = if dropped {
        let blanked = blank_keys(fm_text, |k| accepted(&k.into()));
        match serde_yaml::from_str::<serde_yaml::Value>(&blanked) {
            Ok(kept) if kept == fm => blanked,
            _ => serde_yaml::to_string(&fm).map_err(invalid)?,
        }
    } else {
        fm_text.to_string()
    };
    serde_yaml::from_str(&text).map_err(invalid)
}

/// The model's one check of a node's frontmatter, which loading runs and `apply` runs on
/// a delta (`optional-nodes`): node `id`'s `fm_text` at `path`, given the keys live
/// changes declare. The rules the text alone decides, then, once those pass, the root
/// rules, `optional: true` on the root (`optional-nodes`) and `outside` on any other node
/// (`outside-globs`). Those leave the frontmatter parsed, so they come back with it.
pub fn check_node_frontmatter(
    id: &str,
    path: &Path,
    fm_text: &str,
    declared: &BTreeSet<String>,
) -> Result<(Frontmatter, Vec<Violation>), Vec<Violation>> {
    let fm = check_frontmatter(path, fm_text, declared)?;
    let mut violations = Vec::new();
    if id == ROOT_ID && fm.optional {
        violations.push(v(
            path,
            "model/optional-nodes",
            "the root component can't be `optional`".into(),
        ));
    }
    if id != ROOT_ID && fm.outside.is_some() {
        violations.push(v(
            path,
            OUTSIDE,
            format!("only the root sets `outside`, not component `{id}`"),
        ));
    }
    Ok((fm, violations))
}

const OUTSIDE: &str = "model/outside-globs";
const TEST_KEYS: &str = "model/test-ownership-keys";

const EXEMPTIONS: &str = "model/exemptions";

/// The frontmatter's `tests` value, as one line of YAML, when it is present and not a
/// list of non-empty strings. A null counts.
fn bad_tests(fm: &serde_yaml::Value) -> Option<String> {
    let value = fm.get("tests")?;
    let ok = value.as_sequence().is_some_and(|items| {
        items
            .iter()
            .all(|g| g.as_str().is_some_and(|g| !g.is_empty()))
    });
    (!ok).then(|| one_line(value))
}

/// What is wrong with the frontmatter's `exemptions`, one message per problem, each
/// naming the key and, for an entry, its 1-based position.
fn bad_exemptions(fm: &serde_yaml::Value) -> Vec<String> {
    let Some(value) = fm.get("exemptions") else {
        return Vec::new();
    };
    // A value that isn't a list is left to the typed parse.
    let Some(entries) = value.as_sequence() else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let at = format!("`exemptions` entry {}", i + 1);
        let Some(map) = entry.as_mapping() else {
            problems.push(format!("{at} must be a map, not `{}`", one_line(entry)));
            continue;
        };
        for key in map.keys() {
            if !matches!(key.as_str(), Some("rule" | "files" | "until" | "why")) {
                problems.push(format!("{at} holds the unknown key `{}`", one_line(key)));
            }
        }
        match map.get("rule") {
            None => problems.push(format!("{at} lacks `rule`")),
            Some(rule) if !rule.as_str().is_some_and(|r| EXEMPT_RULES.contains(&r)) => {
                problems.push(format!(
                    "{at} names the rule `{}`, which can't be exempted (one of {})",
                    one_line(rule),
                    EXEMPT_RULES.join(", ")
                ));
            }
            Some(_) => {}
        }
        match map.get("files") {
            None => problems.push(format!("{at} lacks `files`")),
            Some(files) => {
                let ok = files.as_sequence().is_some_and(|globs| {
                    !globs.is_empty()
                        && globs
                            .iter()
                            .all(|g| g.as_str().is_some_and(|g| !g.is_empty()))
                });
                if !ok {
                    problems.push(format!(
                        "{at} needs `files` to be a non-empty list of non-empty globs, not `{}`",
                        one_line(files)
                    ));
                }
            }
        }
        match map.get("until") {
            None => problems.push(format!("{at} lacks `until`")),
            Some(until) => {
                let ok = until.as_str().is_some_and(|u| {
                    u.strip_prefix("backlog/")
                        .is_some_and(|s| !s.is_empty() && !s.contains(['/', ' ']))
                });
                if !ok {
                    problems.push(format!(
                        "{at} needs `until` to be `backlog/<slug>`, not `{}`",
                        one_line(until)
                    ));
                }
            }
        }
    }
    problems
}

/// The frontmatter's `outside` value, as one line of YAML, when it is present and not a
/// list of non-empty strings.
fn bad_outside(fm: &serde_yaml::Value) -> Option<String> {
    let value = fm.get("outside")?;
    let globs = value.as_sequence().is_some_and(|items| {
        items
            .iter()
            .all(|g| g.as_str().is_some_and(|g| !g.is_empty()))
    });
    (!globs).then(|| one_line(value))
}

/// `yaml` with each top-level `key:` line whose key `drop` holds, and the lines that
/// continue its value, made blank, so every other line keeps its number.
fn blank_keys(yaml: &str, drop: impl Fn(&str) -> bool) -> String {
    let mut dropping = false;
    let lines: Vec<&str> = yaml
        .lines()
        .map(|line| {
            let top = !line.starts_with([' ', '\t', '-', '#']) && !line.trim().is_empty();
            if top {
                dropping = line.split_once(':').is_some_and(|(k, _)| drop(k.trim()));
            }
            if dropping { "" } else { line }
        })
        .collect();
    lines.join("\n")
}

/// The node parser loading runs on each `SPEC.md` (`graph-check`): node `id`'s `text`,
/// at repo-relative `path`, given the frontmatter keys live changes declare. It gives the
/// node, when one loads, with no assets, and the violations that file alone decides,
/// the root rules included. Rules across nodes are `check_graph`'s.
pub fn parse_node(
    id: &str,
    path: &Path,
    text: &str,
    declared: &BTreeSet<String>,
) -> (Option<Node>, Vec<Violation>) {
    let mut violations = Vec::new();
    let node = parse_node_into(id, path, text, declared, &mut violations);
    (node, violations)
}

fn parse_node_into(
    id: &str,
    path: &Path,
    text: &str,
    declared: &BTreeSet<String>,
    violations: &mut Vec<Violation>,
) -> Option<Node> {
    if Layer::reserves(id) {
        violations.push(v(
            path,
            "model/reserved-ids",
            format!("`{id}` is reserved for the human layers' IDs, so no component may take it"),
        ));
        return None;
    }
    let text = text.replace("\r\n", "\n");
    let Some((fm_text, body)) = split_frontmatter(&text) else {
        violations.push(v(
            path,
            "model/parse-node-specs",
            "missing `---` frontmatter block".into(),
        ));
        return None;
    };
    let frontmatter = match check_node_frontmatter(id, path, fm_text, declared) {
        Ok((fm, root_rules)) => {
            violations.extend(root_rules);
            fm
        }
        Err(errors) => {
            violations.extend(errors);
            return None;
        }
    };

    let lines: Vec<&str> = body.lines().collect();
    let sections = outline::section_spans(&lines);
    let section = |name: &str| sections.iter().find(|s| s.name == name);
    let section_text = |name: &str| {
        section(name).map_or(String::new(), |s| {
            outline::join(&lines[s.heading + 1..s.end])
        })
    };
    let contract = section_text("Contract");
    let design = section_text("Design");
    let spans = section("Requirements")
        .map(|s| outline::requirement_spans(&lines, s.heading + 1..s.end))
        .unwrap_or_default();
    let mut needs_blocks = outline::blocks_named(&lines, "#### Needs");
    let mut serves_blocks = outline::blocks_named(&lines, "#### Serves");
    let requirements: Vec<Requirement> = spans
        .iter()
        .map(|r| {
            let text = outline::join(&lines[r.lines.clone()]);
            let id = format!("{id}/{}", r.slug);
            let take = |blocks: &mut Vec<_>| {
                let (mine, rest) = blocks.drain(..).partition(|(at, _)| r.lines.contains(at));
                *blocks = rest;
                mine
            };
            let (needs, serves) = (take(&mut needs_blocks), take(&mut serves_blocks));
            // What a rule serves isn't what its code must do, so a `#### Serves` block
            // leaves its version as it was, but still moves its hash
            // (`content-hash`, `serves-in-the-base`).
            let less_serves: Vec<&str> = r
                .lines
                .clone()
                .filter(|i| !serves.iter().any(|(at, body)| (*at..body.end).contains(i)))
                .map(|i| lines[i])
                .collect();
            let owner = format!("requirement `{id}`");
            Requirement {
                needs: parse_ids(&lines, needs, &NEEDS, &owner, path, violations),
                serves: parse_ids(&lines, serves, &SERVES, &owner, path, violations),
                enforced_by: Vec::new(),
                untested: false,
                untested_text: None,
                scenarios: outline::scenario_count(&lines, r.lines.clone()),
                hash: requirement_hash(&outline::join(&less_serves), &text),
                text,
                id,
            }
        })
        .collect();
    for (block, stray) in [(&NEEDS, needs_blocks), (&SERVES, serves_blocks)] {
        for _ in stray {
            violations.push(v(
                path,
                block.rule,
                format!("a `#### {}` block outside any requirement", block.name),
            ));
        }
    }

    // The capability layer's node holds no rules of its own: the capabilities are placed
    // on it, and are written in `CAPABILITIES.md` (`reserved-ids`).
    let requirements = if id == Layer::Capability.id() {
        for r in &requirements {
            let slug = r.id.split_once('/').map_or("", |(_, s)| s);
            violations.push(v(
                path,
                "model/reserved-ids",
                format!(
                    "requirement `{slug}` is in component `{id}`, whose capabilities are written in `{}`",
                    Layer::Capability.file()
                ),
            ));
        }
        Vec::new()
    } else {
        requirements
    };

    let mut seen = BTreeSet::new();
    for r in &requirements {
        if !seen.insert(&r.id) {
            violations.push(v(
                path,
                "model/unique-requirement-ids",
                format!("duplicate requirement `{}`", r.id),
            ));
        }
        if r.scenarios == 0 {
            violations.push(v(
                path,
                "model/unique-requirement-ids",
                format!("requirement `{}` has no scenario", r.id),
            ));
        }
    }

    Some(Node {
        parent: parent_id(id),
        hashes: Hashes {
            frontmatter: content_hash(fm_text),
            contract: content_hash(&contract),
            design: content_hash(&design),
        },
        id: id.to_string(),
        path: path.to_path_buf(),
        frontmatter,
        contract,
        design,
        requirements,
        source: text.clone(),
        assets: BTreeMap::new(),
    })
}

/// A `#### <name>` block of IDs a requirement or entry may hold: what its lines name, and
/// the rule its violations break.
pub(crate) struct IdBlock {
    pub(crate) name: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) shape: &'static str,
    /// Whether a block naming nothing is a violation.
    pub(crate) nonempty: bool,
}

const NEEDS: IdBlock = IdBlock {
    name: "Needs",
    rule: "model/requirement-needs",
    shape: "<component>/<slug>",
    nonempty: false,
};

const SERVES: IdBlock = IdBlock {
    name: "Serves",
    rule: "model/serves",
    shape: "<layer>/<slug>",
    nonempty: true,
};

/// The IDs in `owner`'s `block`, given its `blocks` from `outline::blocks_named`,
/// reporting a second block, any line that isn't a `- <a>/<b>` bullet and, when the block
/// must name something, an empty one. `owner` names it in messages, as
/// ``requirement `a/x` `` does.
pub(crate) fn parse_ids(
    lines: &[&str],
    blocks: Vec<(usize, std::ops::Range<usize>)>,
    block: &IdBlock,
    owner: &str,
    path: &Path,
    violations: &mut Vec<Violation>,
) -> Vec<String> {
    let IdBlock { name, rule, .. } = *block;
    if blocks.len() > 1 {
        violations.push(v(
            path,
            rule,
            format!("{owner} has more than one `#### {name}` block"),
        ));
    }
    let Some((_, body)) = blocks.into_iter().next() else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    let mut empty = true;
    for line in lines[body]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
    {
        empty = false;
        let named = line.strip_prefix("- ").map(str::trim).filter(|n| {
            n.split_once('/').is_some_and(|(node, slug)| {
                !node.is_empty() && !slug.is_empty() && !slug.contains('/')
            }) && !n.contains(|c: char| c.is_whitespace() || c == '`')
        });
        match named {
            Some(named) => ids.push(named.to_string()),
            None => violations.push(v(
                path,
                rule,
                format!(
                    "{owner} has a `#### {name}` line that isn't `- {}`: `{line}`",
                    block.shape
                ),
            )),
        }
    }
    if empty && block.nonempty {
        violations.push(v(
            path,
            rule,
            format!("{owner} has an empty `#### {name}` block"),
        ));
    }
    ids
}

pub fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest
        .find("\n---\n")
        .map(|i| (i, i + 5))
        .or_else(|| rest.strip_suffix("\n---").map(|fm| (fm.len(), rest.len())))?;
    Some((&rest[..end.0], &rest[end.1..]))
}

/// The graph check loading runs (`graph-check`): the rules that join nodes, and backlog
/// items, to other nodes. `depends_on` naming no node and cycles (`dependency-dag`), a
/// required node depending on an optional one (`optional-nodes`), needs across
/// requirements (`requirement-needs`), what rules serve (`serves`), what architecture
/// entries are enforced by (`human-layers`), and backlog items' `origin`, `nodes` and
/// `blocks` (`parse-backlog`).
pub fn check_graph(graph: &Graph) -> Vec<Violation> {
    let mut violations = Vec::new();
    check_dependencies(graph, &mut violations);
    check_needs(graph, &mut violations);
    check_serves(graph, &mut violations);
    layers::check_enforced_by(graph, &mut violations);
    records::check_backlog(graph, &mut violations);
    violations
}

fn check_dependencies(graph: &Graph, violations: &mut Vec<Violation>) {
    for node in graph.nodes.values() {
        let required = !graph.is_optional(&node.id);
        for dep in &node.frontmatter.depends_on {
            if !graph.nodes.contains_key(dep) {
                violations.push(v(
                    &node.path,
                    "model/dependency-dag",
                    format!("depends on unknown component `{dep}`"),
                ));
            } else if required && graph.is_optional(dep) {
                violations.push(v(
                    &node.path,
                    "model/optional-nodes",
                    format!(
                        "required component `{}` depends on optional component `{dep}`",
                        node.id
                    ),
                ));
            }
        }
    }

    let edges = |id: &str| -> Vec<&str> {
        let deps = &graph.nodes[id].frontmatter.depends_on;
        deps.iter()
            .map(String::as_str)
            .filter(|d| graph.nodes.contains_key(*d))
            .collect()
    };
    for cycle in find_cycles(graph.nodes.keys().map(String::as_str), edges) {
        let mut names = cycle.clone();
        names.push(cycle[0]);
        violations.push(v(
            &graph.nodes[cycle[0]].path,
            "model/dependency-dag",
            format!("dependency cycle: {}", names.join(" -> ")),
        ));
    }
}

/// Reports needs that name no requirement, a requirement of the same component, or an optional
/// node's requirement from a required node's, and cycles of needs (`requirement-needs`).
fn check_needs(graph: &Graph, violations: &mut Vec<Violation>) {
    const RULE: &str = "model/requirement-needs";
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in graph.nodes.values() {
        let required = !graph.is_optional(&node.id);
        for r in &node.requirements {
            let valid = edges.entry(r.id.as_str()).or_default();
            for need in &r.needs {
                let (target, _) = need.split_once('/').unwrap_or_default();
                let problem = if Layer::of(need).is_some() {
                    "a human-layer entry, not a requirement"
                } else if target == node.id {
                    "a requirement of the same component"
                } else if graph.requirement(need).is_none() {
                    "a requirement that doesn't exist"
                } else if required && graph.is_optional(target) {
                    "a requirement of an optional component, from a required component"
                } else {
                    valid.push(need);
                    continue;
                };
                violations.push(v(
                    &node.path,
                    RULE,
                    format!("`{}` needs `{need}`, {problem}", r.id),
                ));
            }
        }
    }
    for cycle in find_cycles(edges.keys().copied(), |id| edges[id].clone()) {
        let mut names: Vec<String> = cycle.iter().map(|id| format!("`{id}`")).collect();
        names.push(names[0].clone());
        let (node, _) = cycle[0].split_once('/').unwrap_or_default();
        violations.push(v(
            &graph.nodes[node].path,
            RULE,
            format!("cycle of needs: {}", names.join(" -> ")),
        ));
    }
}

/// Reports a node's `serves`, or a requirement's `#### Serves` block, naming an ID that
/// isn't `<layer>/<slug>` or an entry that doesn't exist (`serves`).
fn check_serves(graph: &Graph, violations: &mut Vec<Violation>) {
    for node in graph.nodes.values() {
        let owners =
            std::iter::once((format!("component `{}`", node.id), &node.frontmatter.serves));
        let rules = node
            .requirements
            .iter()
            .map(|r| (format!("`{}`", r.id), &r.serves));
        for (owner, ids) in owners.chain(rules) {
            for id in ids {
                let problem = if Layer::of(id).is_none() {
                    "which isn't `architecture/<slug>` or `capability/<slug>`"
                } else if graph.entry(id).is_none() {
                    "which names no entry"
                } else {
                    continue;
                };
                violations.push(v(
                    &node.path,
                    "model/serves",
                    format!("{owner} serves `{id}`, {problem}"),
                ));
            }
        }
    }
}

/// Cycles among `ids` found by DFS, each once, rotated to start at its smallest ID: at
/// least one whenever there is any, though not every cycle. `edges` gives an ID's
/// successors, all of them among `ids`.
fn find_cycles<'a>(
    ids: impl Iterator<Item = &'a str>,
    edges: impl Fn(&'a str) -> Vec<&'a str>,
) -> BTreeSet<Vec<&'a str>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Visiting,
        Done,
    }
    fn visit<'a>(
        id: &'a str,
        edges: &impl Fn(&'a str) -> Vec<&'a str>,
        marks: &mut BTreeMap<&'a str, Mark>,
        stack: &mut Vec<&'a str>,
        cycles: &mut BTreeSet<Vec<&'a str>>,
    ) {
        match marks.get(id) {
            Some(Mark::Done) => return,
            Some(Mark::Visiting) => {
                let start = stack.iter().position(|s| *s == id).unwrap();
                let mut cycle = stack[start..].to_vec();
                let min = cycle.iter().enumerate().min_by_key(|(_, s)| **s).unwrap().0;
                cycle.rotate_left(min);
                cycles.insert(cycle);
                return;
            }
            None => {}
        }
        marks.insert(id, Mark::Visiting);
        stack.push(id);
        for next in edges(id) {
            visit(next, edges, marks, stack, cycles);
        }
        stack.pop();
        marks.insert(id, Mark::Done);
    }

    let mut marks = BTreeMap::new();
    let mut cycles = BTreeSet::new();
    for id in ids {
        visit(id, &edges, &mut marks, &mut Vec::new(), &mut cycles);
    }
    cycles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_are_the_frontmatter_fields() {
        // req:model/valid-frontmatter@b4618360
        let msg = serde_yaml::from_str::<Frontmatter>("zz_probe: 1")
            .unwrap_err()
            .to_string();
        let expected = msg.split("expected one of ").nth(1).expect(&msg);
        let fields: Vec<&str> = expected
            .split(',')
            .map(|f| f.trim().split('`').nth(1).unwrap_or_default())
            .collect();
        assert_eq!(fields, KNOWN_KEYS, "{msg}");
    }
}
