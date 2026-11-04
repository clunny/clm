//! Backlog items (`backlog/<slug>.md`) and decisions (`docs/decisions/<name>.md`).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::change::has_key;
use crate::files::{Files, children, text};
use crate::priority::Priority;
use crate::{Graph, Violation, split_frontmatter, v};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BacklogKind {
    Bug,
    Deferred,
    Idea,
    Question,
    /// A step only a person can take (`blocked-by-backlog`); written `human-task`. The
    /// words `plain-terms-renames` replaced still load as the new ones
    /// (`renamed-words-still-load`).
    #[serde(rename = "human-task", alias = "human")]
    Human,
    /// A build's flag that a requirement was hard to build (`workaround-flags`); written
    /// `workaround`.
    #[serde(rename = "workaround", alias = "friction")]
    Friction,
}

/// Every backlog kind with the word CLM writes for it in a file (`term-lists`).
pub const BACKLOG_KIND_TERMS: &[(BacklogKind, &str)] = &[
    (BacklogKind::Bug, "bug"),
    (BacklogKind::Deferred, "deferred"),
    (BacklogKind::Idea, "idea"),
    (BacklogKind::Question, "question"),
    (BacklogKind::Human, "human-task"),
    (BacklogKind::Friction, "workaround"),
];

impl fmt::Display for BacklogKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BacklogKind::Bug => "bug",
            BacklogKind::Deferred => "deferred",
            BacklogKind::Idea => "idea",
            BacklogKind::Question => "question",
            BacklogKind::Human => "human-task",
            BacklogKind::Friction => "workaround",
        })
    }
}

/// A requirement at a version, `<component>/<slug>@<version>`: what a workaround flag names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct RequirementAt {
    /// `<component>/<slug>`
    pub id: String,
    /// The first 8 hex digits of its hash, as `Requirement::version` gives it.
    pub version: String,
}

impl TryFrom<String> for RequirementAt {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let parsed = text.split_once('@').filter(|(id, version)| {
            id.split_once('/').is_some_and(|(node, slug)| {
                !node.is_empty() && !slug.is_empty() && !slug.contains('/')
            }) && !id.contains(char::is_whitespace)
                && version.len() == 8
                && version
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
        });
        match parsed {
            Some((id, version)) => Ok(RequirementAt {
                id: id.into(),
                version: version.into(),
            }),
            None => Err(format!(
                "`requirement` must be `<component>/<slug>@<version>`, with 8 lowercase hex digits, not `{text}`"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawBacklogFrontmatter")]
pub struct BacklogFrontmatter {
    pub title: String,
    pub kind: BacklogKind,
    /// The components the item concerns, written `components` (`component-keys`).
    pub nodes: Vec<String>,
    pub origin: Option<String>,
    /// `P5` when absent (`priority-caps`).
    pub priority: Priority,
    /// Requirement IDs that can't be built until this item is resolved
    /// (`requirement-blocks`).
    pub blocks: Vec<String>,
    /// The requirement a `workaround` item flags, at the version the build built; set on
    /// a `workaround` item only (`workaround-flags`).
    pub requirement: Option<RequirementAt>,
}

/// A backlog item's frontmatter as written, before `components` and its old key
/// `nodes` are read as one (`component-keys`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBacklogFrontmatter {
    title: String,
    kind: BacklogKind,
    #[serde(default, deserialize_with = "present")]
    components: Option<Option<Vec<String>>>,
    #[serde(default, deserialize_with = "present")]
    nodes: Option<Option<Vec<String>>>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    priority: Priority,
    #[serde(default)]
    blocks: Vec<String>,
    #[serde(default)]
    requirement: Option<RequirementAt>,
}

/// A key's value when the key is written at all, a null included.
pub(crate) fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

impl TryFrom<RawBacklogFrontmatter> for BacklogFrontmatter {
    type Error = String;

    fn try_from(raw: RawBacklogFrontmatter) -> Result<Self, String> {
        let nodes = match (raw.components, raw.nodes) {
            (Some(_), Some(_)) => {
                return Err(
                    "sets both `components` and `nodes`, its old key: use `components`".into(),
                );
            }
            (Some(list), None) | (None, Some(list)) => list.unwrap_or_default(),
            (None, None) => Vec::new(),
        };
        Ok(BacklogFrontmatter {
            title: raw.title,
            kind: raw.kind,
            nodes,
            origin: raw.origin,
            priority: raw.priority,
            blocks: raw.blocks,
            requirement: raw.requirement,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklogItem {
    pub slug: String,
    pub path: PathBuf,
    pub frontmatter: BacklogFrontmatter,
}

impl fmt::Display for BacklogItem {
    /// `<slug> <kind> <priority> [<nodes>] <title>`, as `clm backlog` prints it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fm = &self.frontmatter;
        write!(
            f,
            "{} {} {} [{}] {}",
            self.slug,
            fm.kind,
            fm.priority,
            fm.nodes.join(", "),
            fm.title
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    Accepted,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionFrontmatter {
    pub title: String,
    pub date: String,
    pub status: DecisionStatus,
    #[serde(default)]
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub name: String,
    pub path: PathBuf,
    pub frontmatter: DecisionFrontmatter,
}

/// `(stem, repo-relative path, frontmatter)` for each `*.md` in `dir`, parsed as `T`.
fn load_dir<T: for<'de> Deserialize<'de>>(
    files: &Files,
    dir: &str,
    rule: &'static str,
    violations: &mut Vec<Violation>,
) -> Vec<(String, PathBuf, T)> {
    let mut out = Vec::new();
    for rel in children(files, Path::new(dir)) {
        if rel.extension().is_none_or(|x| x != "md") {
            continue;
        }
        let stem = rel.file_stem().unwrap().to_string_lossy().into_owned();
        let text = text(files, &rel)
            .and_then(Result::ok)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        let Some((fm, _)) = split_frontmatter(&text) else {
            violations.push(v(&rel, rule, "missing `---` frontmatter block".into()));
            continue;
        };
        match serde_yaml::from_str::<T>(fm) {
            Ok(parsed) => out.push((stem, rel, parsed)),
            Err(e) => violations.push(v(&rel, rule, e.to_string())),
        }
    }
    out
}

/// Loads `backlog/`, parsing each item; `check_backlog` checks its references.
pub(crate) fn load_backlog(
    files: &Files,
    violations: &mut Vec<Violation>,
) -> BTreeMap<String, BacklogItem> {
    load_dir::<BacklogFrontmatter>(files, "backlog", BACKLOG, violations)
        .into_iter()
        .map(|(slug, path, frontmatter)| {
            // The old key still deserialises, for history reads, but `backlog/` may not
            // hold it. Reported, and the item stays loaded (`component-keys`).
            let old_key = text(files, &path).and_then(Result::ok).and_then(|t| {
                split_frontmatter(&t.replace("\r\n", "\n")).map(|(fm, _)| has_key(fm, "nodes"))
            });
            if old_key == Some(true) {
                violations.push(v(
                    &path,
                    BACKLOG,
                    "sets `nodes`, its old key: use `components`".into(),
                ));
            }
            // A `workaround` item names its requirement, and no other kind does. Reported,
            // and kept loaded like an item with a bad reference, so its `blocks` hold.
            let workaround = frontmatter.kind == BacklogKind::Friction;
            if workaround != frontmatter.requirement.is_some() {
                let problem = if workaround {
                    "a `workaround` item needs a `requirement`, `<component>/<slug>@<version>`"
                } else {
                    "only a `workaround` item sets `requirement`"
                };
                violations.push(v(&path, BACKLOG, problem.into()));
            }
            (
                slug.clone(),
                BacklogItem {
                    slug,
                    path,
                    frontmatter,
                },
            )
        })
        .collect()
}

const BACKLOG: &str = "model/parse-backlog";

/// Whether a `blocks` entry, or an `#### Enforced by` ID (`human-layers`), may name
/// requirement `id`: it exists, or a loaded delta for its node, of a change outside the
/// archive whatever its status, adds it under `## ADDED Requirements`, so the spec PR
/// that adds a requirement may file the item that blocks it.
pub(crate) fn blockable(graph: &Graph, id: &str) -> bool {
    let Some((node, slug)) = id.split_once('/') else {
        return false;
    };
    graph.requirement(id).is_some()
        || graph
            .changes
            .values()
            .filter(|c| !c.archived)
            .flat_map(|c| &c.deltas)
            .filter(|d| d.node == node)
            .any(|d| d.added.iter().any(|r| r.slug == slug))
}

/// Checks each backlog item's node, origin, `blocks` and `requirement` references against
/// `graph`. A node may also be one that a `create: true` delta of a change outside the
/// archive creates, so a spec PR can file items naming the node it adds
/// (`backlog-nodes`). A `blocks` entry must name a requirement that exists or that a live
/// change adds, so the spec PR that removes a requirement edits the items that block it
/// (`requirement-blocks`). A workaround flag must name a requirement that exists at its
/// current version, so the spec change that removes or rewrites one deletes its flags or
/// re-records them (`workaround-flags`).
pub(crate) fn check_backlog(graph: &Graph, violations: &mut Vec<Violation>) {
    for item in graph.backlog.values() {
        let (fm, path) = (&item.frontmatter, &item.path);
        for node in &fm.nodes {
            if !graph.backlog_node_known(node) {
                violations.push(v(path, BACKLOG, format!("unknown component `{node}`")));
            }
        }
        if let Some(origin) = &fm.origin
            && !graph.changes.contains_key(origin)
        {
            violations.push(v(
                path,
                BACKLOG,
                format!("unknown origin change `{origin}`"),
            ));
        }
        for id in &fm.blocks {
            if !blockable(graph, id) {
                violations.push(v(
                    path,
                    BACKLOG,
                    format!(
                        "`blocks` names `{id}`, which isn't a requirement or one a live change adds"
                    ),
                ));
            }
        }
        // On another kind, `requirement` is reported by loading alone.
        if let Some(RequirementAt { id, version }) = &fm.requirement
            && fm.kind == BacklogKind::Friction
        {
            let problem = match graph.requirement(id) {
                None => "isn't a requirement".to_string(),
                Some(r) if r.version() != version => format!(
                    "is at version `{}`, not `{version}`: re-record the flag or delete it",
                    r.version()
                ),
                Some(_) => continue,
            };
            violations.push(v(
                path,
                BACKLOG,
                format!("`requirement` names `{id}`, which {problem}"),
            ));
        }
    }
}

/// Loads `docs/decisions/`, checking `superseded_by` links.
pub(crate) fn load_decisions(
    files: &Files,
    violations: &mut Vec<Violation>,
) -> BTreeMap<String, Decision> {
    const RULE: &str = "model/parse-decisions";
    let decisions: BTreeMap<String, Decision> =
        load_dir::<DecisionFrontmatter>(files, "docs/decisions", RULE, violations)
            .into_iter()
            .map(|(name, path, frontmatter)| {
                (
                    name.clone(),
                    Decision {
                        name,
                        path,
                        frontmatter,
                    },
                )
            })
            .collect();
    for d in decisions.values() {
        let fm = &d.frontmatter;
        match (fm.status, &fm.superseded_by) {
            (DecisionStatus::Superseded, None) => violations.push(v(
                &d.path,
                RULE,
                "superseded decision needs `superseded_by`".into(),
            )),
            (DecisionStatus::Accepted, Some(_)) => violations.push(v(
                &d.path,
                RULE,
                "`superseded_by` is only allowed when `status: superseded`".into(),
            )),
            (_, Some(by)) if !decisions.contains_key(by) => violations.push(v(
                &d.path,
                RULE,
                format!("superseded by unknown decision `{by}`"),
            )),
            _ => {}
        }
    }
    decisions
}
