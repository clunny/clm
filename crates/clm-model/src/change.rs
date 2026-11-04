//! Changes under `changes/<id>/`: `change.md` plus `deltas/*.md` (see docs/spec-format.md).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::files::{Files, children, subdirs, text, under};
use crate::outline::{
    SectionSpan, asset_entry, fence_mask, join, requirement_heading, requirement_spans,
    section_spans, yaml_fence,
};
use crate::priority::Priority;
use crate::records::present;
use crate::renames::{Rename, load_renames};
use crate::{Violation, split_frontmatter, v};

const RULE: &str = "model/parse-changes";
const UNDER_A_HEADING: &str = "model/requirement-text-under-a-heading";
const SECTION_FIT: &str = "model/delta-section-fit";
const MISPLACED: &str = "model/misplaced-delta-text";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeStatus {
    Proposed,
    Approved,
    Applied,
    Implementing,
    Done,
    Abandoned,
}

/// Every change status with the word it is written as in a file (`term-lists`).
pub const CHANGE_STATUS_TERMS: &[(ChangeStatus, &str)] = &[
    (ChangeStatus::Proposed, "proposed"),
    (ChangeStatus::Approved, "approved"),
    (ChangeStatus::Applied, "applied"),
    (ChangeStatus::Implementing, "implementing"),
    (ChangeStatus::Done, "done"),
    (ChangeStatus::Abandoned, "abandoned"),
];

/// The keys a live change's `change.md` frontmatter may set: the fields
/// `ChangeFrontmatter` reads, without `after`, `prefer_after` and `gate`, which it
/// accepts only to ignore or reject (`term-lists`).
pub const CHANGE_KEYS: &[&str] = &[
    "status",
    "human_burdens",
    "author",
    "from",
    "resolves",
    "priority",
    "frontmatter_keys",
    "renames",
];

impl ChangeStatus {
    /// Not yet applied, so its deltas must still match current specs.
    pub fn is_pending(self) -> bool {
        matches!(self, ChangeStatus::Proposed | ChangeStatus::Approved)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawChangeFrontmatter")]
pub struct ChangeFrontmatter {
    pub status: ChangeStatus,
    /// What the change spends of the human's money or time, or can't undo: a change
    /// that declares any needs review (`gate/needs-review`). Written `human_burdens`.
    pub burdens: Vec<String>,
    pub author: Option<String>,
    /// The backlog item this change was promoted from.
    pub from: Option<String>,
    /// Backlog items this change resolves. The task that fixes one deletes it,
    /// and `finish` deletes the rest (`task-deletes-fixed-items`), so an entry
    /// may name an item that's gone; `gate/resolves-named-items` catches typos.
    pub resolves: Vec<String>,
    /// `P5` when absent (`priority-caps`).
    pub priority: Priority,
    /// Node frontmatter keys this change adds, accepted on any node while it is live
    /// (`declared-frontmatter-keys`).
    pub frontmatter_keys: Vec<String>,
    /// The raw `renames`, checked by `load_change` into `Change::renames`
    /// (`rename-records`).
    renames: Option<serde_yaml::Value>,
}

/// A `change.md`'s frontmatter as written: what it reads, before `human_burdens` and its
/// old key are read as one (`parse-changes`). Its fields are the keys of `CHANGE_KEYS`,
/// the old key and the three it ignores or rejects aside.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChangeFrontmatter {
    status: ChangeStatus,
    #[serde(default, deserialize_with = "present")]
    human_burdens: Option<Option<Vec<String>>>,
    /// The key `human_burdens` replaced (`renamed-words-still-load`).
    #[serde(default, rename = "burdens", deserialize_with = "present")]
    old_burdens: Option<Option<Vec<String>>>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    resolves: Vec<String>,
    /// Accepted and ignored: changes recorded these while changes were ordered against
    /// each other, and archived changes keep theirs (`node-reconciliation`). Sinks, so
    /// nothing can read them.
    #[serde(default, rename = "after")]
    _after: Ignored,
    /// See `after`.
    #[serde(default, rename = "prefer_after")]
    _prefer_after: Ignored,
    /// The review tier archived changes recorded before review became yes or no
    /// (`yes-no-review`); `load_change` rejects it on a live change.
    #[serde(default, rename = "gate")]
    _gate: Ignored,
    #[serde(default)]
    priority: Priority,
    #[serde(default, deserialize_with = "key_names")]
    frontmatter_keys: Vec<String>,
    #[serde(default)]
    renames: Option<serde_yaml::Value>,
}

impl TryFrom<RawChangeFrontmatter> for ChangeFrontmatter {
    type Error = String;

    fn try_from(raw: RawChangeFrontmatter) -> Result<Self, String> {
        let burdens = match (raw.human_burdens, raw.old_burdens) {
            (Some(_), Some(_)) => {
                return Err(
                    "sets both `human_burdens` and `burdens`, its old key: use `human_burdens`"
                        .into(),
                );
            }
            (Some(list), None) | (None, Some(list)) => list.unwrap_or_default(),
            (None, None) => Vec::new(),
        };
        Ok(ChangeFrontmatter {
            status: raw.status,
            burdens,
            author: raw.author,
            from: raw.from,
            resolves: raw.resolves,
            priority: raw.priority,
            frontmatter_keys: raw.frontmatter_keys,
            renames: raw.renames,
        })
    }
}

/// A list of lowercase key names: a lowercase letter, then lowercase letters, digits
/// and `_`.
fn key_names<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    // A bare `frontmatter_keys:` is null: the default, none.
    let keys = Option::<Vec<String>>::deserialize(d)?.unwrap_or_default();
    let bad = keys.iter().find(|k| {
        let mut chars = k.chars();
        !chars.next().is_some_and(|c| c.is_ascii_lowercase())
            || !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    });
    match bad {
        Some(k) => Err(serde::de::Error::custom(format!(
            "`frontmatter_keys` entry `{k}` isn't a lowercase key name"
        ))),
        None => Ok(keys),
    }
}

/// A frontmatter key that is accepted, whatever its value, and holds nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Ignored;

impl<'de> Deserialize<'de> for Ignored {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        IgnoredAny::deserialize(d).map(|_| Ignored)
    }
}

/// Parses a `change.md`'s frontmatter, or `None` if it has none or it's invalid.
pub fn parse_change_frontmatter(text: &str) -> Option<ChangeFrontmatter> {
    let text = text.replace("\r\n", "\n");
    let (fm, _) = split_frontmatter(&text)?;
    serde_yaml::from_str(fm).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub id: String,
    /// Repo-relative path of `change.md`.
    pub path: PathBuf,
    pub source: String,
    pub frontmatter: ChangeFrontmatter,
    pub deltas: Vec<Delta>,
    /// Added or modified assets, `assets/<node-id>/<path>`: node to path to contents.
    pub assets: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
    /// The renames the change records, in file order (`rename-records`).
    pub renames: Vec<Rename>,
    /// Lives under `changes/archive/`.
    pub archived: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bases {
    pub frontmatter: Option<String>,
    pub contract: Option<String>,
    pub design: Option<String>,
    /// Asset path to the hash it had when a modified asset was written.
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeltaFrontmatter {
    #[serde(default, deserialize_with = "present")]
    component: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    node: Option<Option<String>>,
    #[serde(default)]
    create: bool,
    #[serde(default)]
    remove: bool,
    #[serde(default)]
    base: Bases,
}

impl DeltaFrontmatter {
    /// The component the delta edits, named by `component`. A delta of a change outside
    /// the archive that sets the old key `node` is rejected; an archived one is read
    /// as written (`component-keys`, `archive-immutable`).
    fn component(
        &self,
        path: &Path,
        archived: bool,
        violations: &mut Vec<Violation>,
    ) -> Option<String> {
        let problem = match (&self.component, &self.node) {
            (_, Some(_)) if !archived => "sets `node`, its old key: use `component`",
            (Some(Some(c)), None) | (None, Some(Some(c))) => return Some(c.clone()),
            (Some(_), Some(_)) => "sets both `component` and `node`, its old key: use `component`",
            _ => "names no component: set `component`",
        };
        violations.push(v(path, RULE, problem.into()));
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaRequirement {
    pub slug: String,
    pub base: Option<String>,
    /// The full block with a clean `### Requirement: <slug>` heading (empty for REMOVED).
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Delta {
    /// Repo-relative path of the delta file.
    pub path: PathBuf,
    pub node: String,
    pub create: bool,
    /// Removes the node: only `REMOVED` sections (`node-removal`).
    pub remove: bool,
    pub base: Bases,
    /// `create: true` only.
    pub frontmatter: Option<String>,
    pub contract: Option<String>,
    pub design: Option<String>,
    pub modified_frontmatter: Option<String>,
    pub modified_contract: Option<String>,
    pub modified_design: Option<String>,
    pub added: Vec<DeltaRequirement>,
    pub modified: Vec<DeltaRequirement>,
    pub removed: Vec<DeltaRequirement>,
    /// `## REMOVED Assets`: (path, base hash).
    pub removed_assets: Vec<(String, String)>,
}

pub const ARCHIVE_DIR: &str = "archive";

/// Changes in `changes/<id>/` and `changes/archive/<id>/`.
pub(crate) fn load_changes(files: &Files, violations: &mut Vec<Violation>) -> Vec<Change> {
    let changes = Path::new("changes");
    let mut out = Vec::new();
    for (dir, archived) in [
        (changes.to_path_buf(), false),
        (changes.join(ARCHIVE_DIR), true),
    ] {
        let dirs = subdirs(files, &dir)
            .into_iter()
            .filter(|p| archived || p.file_name().is_none_or(|n| n != ARCHIVE_DIR));
        out.extend(dirs.filter_map(|d| load_change(files, &d, archived, violations)));
    }
    out
}

fn load_change(
    files: &Files,
    dir: &Path,
    archived: bool,
    violations: &mut Vec<Violation>,
) -> Option<Change> {
    let path = dir.join("change.md");
    let source = match text(files, &path) {
        Some(Ok(t)) => t,
        Some(Err(e)) => {
            violations.push(v(&path, RULE, format!("unreadable: {e}")));
            return None;
        }
        None => {
            violations.push(v(dir, RULE, "missing change.md".into()));
            return None;
        }
    };
    let source = source.replace("\r\n", "\n");
    let Some((fm, _)) = split_frontmatter(&source) else {
        violations.push(v(&path, RULE, "missing `---` frontmatter block".into()));
        return None;
    };
    let frontmatter: ChangeFrontmatter = match serde_yaml::from_str(fm) {
        Ok(fm) => fm,
        Err(e) => {
            violations.push(v(&path, RULE, e.to_string()));
            return None;
        }
    };
    if !archived && has_key(fm, "gate") {
        violations.push(v(
            &path,
            RULE,
            "`gate` is gone: a change needs review or not (`yes-no-review`); declare `human_burdens` instead".into(),
        ));
    }

    let before = violations.len();
    let renames = load_renames(frontmatter.renames.as_ref(), &path, archived, violations);
    if violations.len() > before {
        return None;
    }

    let deltas: Vec<Delta> = children(files, &dir.join("deltas"))
        .into_iter()
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .filter_map(|f| match text(files, &f)? {
            // The archive is never edited, so its deltas load as they were when they landed.
            Ok(text) if !archived && misplaced_text(&f, &text, violations) => {
                parse_delta(f, &text, archived, violations);
                None
            }
            Ok(text) => parse_delta(f, &text, archived, violations),
            Err(e) => {
                violations.push(v(&f, RULE, format!("unreadable: {e}")));
                None
            }
        })
        .collect();
    let assets = load_assets(files, dir, &deltas, violations);
    Some(Change {
        id: dir.file_name()?.to_string_lossy().into(),
        path,
        source,
        frontmatter,
        deltas,
        assets,
        renames,
        archived,
    })
}

/// Whether the frontmatter `fm`, which parsed, has the top-level key `key`.
pub(crate) fn has_key(fm: &str, key: &str) -> bool {
    serde_yaml::from_str::<serde_yaml::Mapping>(fm).is_ok_and(|m| m.contains_key(key))
}

/// `<dir>/assets/<node-id>/**`, keyed by node then by path below the node's directory.
/// Each node needs a delta in the change, which carries the assets' bases.
fn load_assets(
    files: &Files,
    dir: &Path,
    deltas: &[Delta],
    violations: &mut Vec<Violation>,
) -> BTreeMap<String, BTreeMap<String, Vec<u8>>> {
    let root = dir.join("assets");
    let mut assets: BTreeMap<String, BTreeMap<String, Vec<u8>>> = BTreeMap::new();
    for path in under(files, &root) {
        let parts: Vec<_> = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .iter()
            .map(|c| c.to_string_lossy())
            .collect();
        let [node, rest @ ..] = parts.as_slice() else {
            continue;
        };
        if rest.is_empty() {
            violations.push(v(
                path,
                RULE,
                "a change's asset belongs under `assets/<component-id>/`".into(),
            ));
            continue;
        }
        assets
            .entry(node.to_string())
            .or_default()
            .insert(rest.join("/"), files[path].clone());
    }
    for node in assets.keys() {
        let mut named = deltas.iter().filter(|d| d.node == *node).peekable();
        let problem = if named.peek().is_none() {
            "has no delta in this change"
        } else if named.any(|d| d.remove) {
            "this change removes"
        } else {
            continue;
        };
        violations.push(v(
            &root.join(node),
            RULE,
            format!("assets for component `{node}`, which {problem}"),
        ));
    }
    assets
}

/// The known sections a `remove: true` delta may not hold: it holds only
/// `REMOVED Requirements` and `REMOVED Assets`. An unknown one is reported as unknown.
const NOT_IN_REMOVAL: [&str; 8] = [
    "Frontmatter",
    "Contract",
    "Design",
    "MODIFIED Frontmatter",
    "MODIFIED Contract",
    "MODIFIED Design",
    "ADDED Requirements",
    "MODIFIED Requirements",
];

/// In an `ADDED` or `MODIFIED Requirements` section, the index of its first non-blank
/// line before its first `### Requirement:` heading outside a fence, or anywhere in it
/// when it has no such heading. `REMOVED` text before a heading is rationale.
fn stray_requirement_text(lines: &[&str], s: &SectionSpan) -> Option<usize> {
    if !matches!(
        s.name.as_str(),
        "ADDED Requirements" | "MODIFIED Requirements"
    ) {
        return None;
    }
    let body = s.heading + 1..s.end;
    let first = requirement_spans(lines, body.clone())
        .first()
        .map_or(s.end, |r| r.lines.start);
    (body.start..first).find(|i| !lines[*i].trim().is_empty())
}

/// Pushes a violation for a delta's text before its first section and one for each section
/// other than a requirements section that holds a `### Requirement:` heading outside a code
/// fence; returns whether it pushed any.
fn misplaced_text(path: &Path, text: &str, violations: &mut Vec<Violation>) -> bool {
    let text = text.replace("\r\n", "\n");
    let Some((_, body)) = split_frontmatter(&text) else {
        return false;
    };
    let lines: Vec<&str> = body.lines().collect();
    let above = text[..text.len() - body.len()].matches('\n').count();
    let sections = section_spans(&lines);
    let mut found = Vec::new();
    let first = sections.first().map_or(lines.len(), |s| s.heading);
    if let Some(i) = (0..first).find(|i| !lines[*i].trim().is_empty()) {
        found.push(format!(
            "text at line {} before the delta's first section",
            above + i + 1
        ));
    }
    let mask = fence_mask(&lines);
    for s in &sections {
        if matches!(
            s.name.as_str(),
            "ADDED Requirements" | "MODIFIED Requirements" | "REMOVED Requirements"
        ) {
            continue;
        }
        let heading =
            (s.heading + 1..s.end).find(|i| !mask[*i] && requirement_heading(lines[*i]).is_some());
        if let Some(i) = heading {
            found.push(format!(
                "`### Requirement:` at line {} in `## {}`, which isn't a requirements section",
                above + i + 1,
                s.name
            ));
        }
    }
    let any = !found.is_empty();
    violations.extend(found.into_iter().map(|m| v(path, MISPLACED, m)));
    any
}

/// What's wrong with a section a delta without `remove: true` can't apply, if anything:
/// a whole-node section without `create: true`, an edit section with it, or an empty
/// `MODIFIED Contract` or `MODIFIED Design`. Edit-only wins over empty.
fn section_misfit(create: bool, lines: &[&str], s: &SectionSpan) -> Option<String> {
    let name = s.name.as_str();
    match name {
        "Frontmatter" | "Contract" | "Design" if !create => Some(format!(
            "needs `create: true`: `## MODIFIED {name}` edits an existing component's {name}"
        )),
        "MODIFIED Frontmatter"
        | "MODIFIED Contract"
        | "MODIFIED Design"
        | "MODIFIED Requirements"
        | "REMOVED Requirements"
        | "REMOVED Assets"
            if create =>
        {
            Some("can't be applied: a `create: true` delta writes its component whole".into())
        }
        "MODIFIED Contract" | "MODIFIED Design"
            if lines[s.heading + 1..s.end]
                .iter()
                .all(|l| l.trim().is_empty()) =>
        {
            Some("is empty, but it replaces the section's whole body".into())
        }
        _ => None,
    }
}

pub(crate) fn parse_delta(
    path: PathBuf,
    text: &str,
    archived: bool,
    violations: &mut Vec<Violation>,
) -> Option<Delta> {
    let text = text.replace("\r\n", "\n");
    let Some((fm, body)) = split_frontmatter(&text) else {
        violations.push(v(&path, RULE, "missing `---` frontmatter block".into()));
        return None;
    };
    let meta: DeltaFrontmatter = match serde_yaml::from_str(fm) {
        Ok(m) => m,
        Err(e) => {
            violations.push(v(&path, RULE, e.to_string()));
            return None;
        }
    };
    let component = meta.component(&path, archived, violations)?;
    if meta.create && meta.remove {
        violations.push(v(
            &path,
            RULE,
            "sets both `create` and `remove`: a delta creates its component or removes it".into(),
        ));
        return None;
    }
    let mut delta = Delta {
        node: component,
        create: meta.create,
        remove: meta.remove,
        base: meta.base,
        ..Delta::default()
    };

    let lines: Vec<&str> = body.lines().collect();
    // Lines of the file above the body, so a body index `i` is file line `above + i + 1`.
    let above = text[..text.len() - body.len()].matches('\n').count();
    let mut ok = true;
    for s in section_spans(&lines) {
        if delta.remove && NOT_IN_REMOVAL.contains(&s.name.as_str()) {
            violations.push(v(
                &path,
                RULE,
                format!(
                    "a `remove: true` delta holds only `REMOVED` sections, not `## {}`",
                    s.name
                ),
            ));
            ok = false;
            continue;
        }
        // Judged no further once it misfits, but `requirement-text-under-a-heading` still
        // judges it. A removal's sections are `parse-changes`' alone: the ones it may not
        // hold were rejected above, and its `REMOVED` ones fit a delta without `create`.
        let misfit = section_misfit(delta.create, &lines, &s);
        if let Some(what) = &misfit {
            violations.push(v(
                &path,
                SECTION_FIT,
                format!("`## {}` at line {} {what}", s.name, above + s.heading + 1),
            ));
            ok = false;
        }
        if let Some(i) = stray_requirement_text(&lines, &s) {
            violations.push(v(
                &path,
                UNDER_A_HEADING,
                format!(
                    "`## {}` has text at line {} that isn't under a `### Requirement:` heading",
                    s.name,
                    above + i + 1
                ),
            ));
            ok = false;
            continue;
        }
        if misfit.is_some() {
            continue;
        }
        let section = &lines[s.heading + 1..s.end];
        let mut yaml = |slot: &mut Option<String>| match yaml_fence(section) {
            Some(y) => *slot = Some(y),
            None => {
                violations.push(v(
                    &path,
                    RULE,
                    format!("`## {}` has no ```yaml fence", s.name),
                ));
                ok = false;
            }
        };
        match s.name.as_str() {
            "Frontmatter" => yaml(&mut delta.frontmatter),
            "MODIFIED Frontmatter" => yaml(&mut delta.modified_frontmatter),
            "Contract" => delta.contract = Some(join(section)),
            "Design" => delta.design = Some(join(section)),
            "MODIFIED Contract" => delta.modified_contract = Some(join(section)),
            "MODIFIED Design" => delta.modified_design = Some(join(section)),
            "ADDED Requirements" | "MODIFIED Requirements" | "REMOVED Requirements" => {
                let reqs = requirement_spans(&lines, s.heading + 1..s.end)
                    .into_iter()
                    .map(|r| DeltaRequirement {
                        text: format!(
                            "### Requirement: {}\n{}",
                            r.slug,
                            join(&lines[r.lines.start + 1..r.lines.end])
                        ),
                        slug: r.slug,
                        base: r.base,
                    })
                    .collect();
                match s.name.as_str() {
                    "ADDED Requirements" => delta.added = reqs,
                    "MODIFIED Requirements" => delta.modified = reqs,
                    _ => delta.removed = reqs,
                }
            }
            "REMOVED Assets" => {
                for line in section.iter().filter(|l| l.starts_with("- ")) {
                    match asset_entry(line) {
                        Some((asset, Some(base))) => delta.removed_assets.push((asset, base)),
                        Some((asset, None)) => {
                            violations.push(v(
                                &path,
                                RULE,
                                format!("removed asset `{asset}` has no `<!-- base: <hash> -->`"),
                            ));
                            ok = false;
                        }
                        None => {
                            violations.push(v(
                                &path,
                                RULE,
                                format!("`{line}` isn't a removed asset, `` - `<path>` <!-- base: <hash> --> ``"),
                            ));
                            ok = false;
                        }
                    }
                }
            }
            other => {
                violations.push(v(
                    &path,
                    RULE,
                    format!("unknown delta section `## {other}`"),
                ));
                ok = false;
            }
        }
    }
    delta.path = path;
    ok.then_some(delta)
}
