//! `clm rename`: the deltas, references and archive a change's recorded renames produce.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use clm_coverage::MarkerSpan;
use clm_git::Failure;
use clm_model::outline::{requirement_spans, section_spans};
use clm_model::{
    CHANGE_STATUS_TERMS, Change, ChangeStatus, Delta, Graph, Layer, Node, ROOT_ID, Rename,
    RenameKind, split_frontmatter,
};

use crate::record;
use crate::text::{Edits, Mention, mentions};

/// What a rename did: each `(old, new)` pair in record order, and every old name, for the
/// mentions it leaves.
pub(crate) struct Done {
    pub(crate) renamed: Vec<(String, String)>,
    pub(crate) olds: Vec<(String, Mention)>,
    /// Each requirement as it was before the rename.
    pub(crate) before: Vec<Was>,
    /// The text edits the rename made, which `other-deltas` makes to other changes' deltas.
    edits: Edits,
    /// What the other live changes' deltas held before the rename.
    held: Held,
    /// The `rebased` lines `other-deltas` printed.
    rebased: Vec<String>,
}

/// A requirement before the rename.
pub(crate) struct Was {
    pub(crate) id: String,
    /// The ID it has after the rename.
    pub(crate) new: String,
    pub(crate) version: String,
    /// The old IDs and slugs `references` rewrote in its text.
    pub(crate) rewritten: BTreeSet<String>,
}

/// What the other live changes' deltas held against the specs before the rename: the
/// elements already stale, and each component's frontmatter hash.
#[derive(Default)]
struct Held {
    stale: BTreeSet<(PathBuf, String)>,
    frontmatter: BTreeMap<String, String>,
}

fn refuse<T>(why: String) -> Result<T, Failure> {
    Err(Failure::Refused(why))
}

fn io(path: &Path, e: std::io::Error) -> Failure {
    Failure::Refused(format!("{}: {e}", path.display()))
}

/// The files `rename` writes, each with what it held before, so a refusal restores them.
struct Writes<'a> {
    root: &'a Path,
    before: Vec<(PathBuf, Option<Vec<u8>>)>,
}

impl Writes<'_> {
    /// Remembers what `rel` holds, or that it holds nothing, before the first write to it.
    fn remember(&mut self, rel: &Path) -> Result<(), Failure> {
        if self.before.iter().any(|(p, _)| p == rel) {
            return Ok(());
        }
        let was = match fs::read(self.root.join(rel)) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io(rel, e)),
        };
        self.before.push((rel.to_path_buf(), was));
        Ok(())
    }

    /// Removes the files directly in `rel`, the change's old deltas.
    fn clear(&mut self, rel: &Path) -> Result<(), Failure> {
        let Ok(entries) = fs::read_dir(self.root.join(rel)) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            let path = rel.join(entry.file_name());
            self.remember(&path)?;
            fs::remove_file(self.root.join(&path)).map_err(|e| io(&path, e))?;
        }
        Ok(())
    }

    fn put(&mut self, rel: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let full = self.root.join(rel);
        self.remember(rel)?;
        if let Some(dir) = full.parent() {
            fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
        }
        fs::write(&full, bytes).map_err(|e| io(&full, e))
    }

    fn undo(self) {
        for (rel, was) in self.before.iter().rev() {
            let full = self.root.join(rel);
            let _ = match was {
                Some(bytes) => fs::write(&full, bytes),
                None => fs::remove_file(&full),
            };
            for dir in rel.ancestors().skip(1) {
                if fs::remove_dir(self.root.join(dir)).is_err() {
                    break;
                }
            }
        }
    }
}

/// Carries out change `id`'s renames in the checkout at `root`, then names what it left.
pub fn rename(root: &Path, id: &str) -> Result<Vec<String>, Failure> {
    let done = carry_out(root, id)?;
    let mut lines: Vec<String> = done
        .renamed
        .iter()
        .map(|(old, new)| format!("renamed {old} -> {new}"))
        .collect();
    lines.extend(done.rebased.iter().cloned());
    lines.extend(leftovers(root, &done.olds, &[])?.into_iter().map(|hit| {
        format!(
            "still names {} {}:{}",
            hit.old,
            hit.file.display(),
            hit.line
        )
    }));
    Ok(lines)
}

/// `rename` without the report of what it left: the working tree after the renames.
pub(crate) fn carry_out(root: &Path, id: &str) -> Result<Done, Failure> {
    let (graph, violations) = clm_model::load(root);
    // The model's own refusals of this record, such as `model/rename-records`; a change it
    // rejects isn't in the graph at all.
    let path = Path::new("changes").join(id).join("change.md");
    let rejected: Vec<String> = violations
        .iter()
        .filter(|v| v.path == path)
        .map(|v| format!("{}: {}", v.rule, v.message))
        .collect();
    let Some(change) = graph.changes.get(id).filter(|c| !c.archived) else {
        return refuse(if rejected.is_empty() {
            format!("no live change `{id}`")
        } else {
            rejected.join("\n")
        });
    };
    if change.frontmatter.status != ChangeStatus::Proposed {
        let status = CHANGE_STATUS_TERMS
            .iter()
            .find(|(s, _)| *s == change.frontmatter.status)
            .map_or("unknown", |(_, term)| *term);
        return refuse(format!(
            "change `{id}` is `{status}`; only a proposed change can be renamed"
        ));
    }
    if !rejected.is_empty() {
        return refuse(rejected.join("\n"));
    }
    if change.renames.is_empty() {
        return refuse(format!("change `{id}` records no renames"));
    }
    let (renames, resolves) = (change.renames.clone(), change.frontmatter.resolves.clone());
    drop(graph);
    let mut writes = Writes {
        root,
        before: Vec::new(),
    };
    let done = generate(root, id, &renames, &mut writes)
        .and_then(|done| apply(root, id).map(|()| done))
        .and_then(|done| rewrite_markers(root, &done, &resolves, &mut writes).map(|()| done))
        .and_then(|mut done| {
            done.rebased = rebase_others(root, id, &done, &mut writes)?;
            Ok(done)
        })
        .and_then(|done| rewrite_flags(root, &done, &mut writes).map(|()| done));
    let done = match done {
        Ok(done) => done,
        Err(e) => {
            writes.undo();
            return Err(e);
        }
    };
    finish(root, id, &resolves)?;
    Ok(done)
}

fn apply(root: &Path, id: &str) -> Result<(), Failure> {
    clm_apply::apply(root, id).map(drop).map_err(|violations| {
        let why: Vec<String> = violations
            .iter()
            .map(|v| format!("{}: {}", v.path.display(), v.message))
            .collect();
        Failure::Refused(why.join("\n"))
    })
}

/// Marks the applied change `done` by its `status:` line alone, archives it, and deletes the
/// items it resolves.
fn finish(root: &Path, id: &str, resolves: &[String]) -> Result<(), Failure> {
    let change = root.join("changes").join(id).join("change.md");
    let applied = fs::read_to_string(&change).map_err(|e| io(&change, e))?;
    fs::write(&change, record::with_status(&applied, "done")).map_err(|e| io(&change, e))?;
    clm_apply::archive(root, id).map_err(|vs| {
        let why: Vec<String> = vs.iter().map(|v| v.message.clone()).collect();
        Failure::Refused(why.join("\n"))
    })?;
    for slug in resolves {
        let _ = fs::remove_file(root.join("backlog").join(format!("{slug}.md")));
    }
    Ok(())
}

/// Rewrites the markers that cite a renamed requirement, or one whose text the rename
/// changed, in the files the coverage counts them in (`marker-rewrite`): a marker at the
/// requirement's version before cites the new ID at its version after, any other marker the
/// new ID at the version it had.
///
/// It works in two passes around the search for what the code still names: the markers first
/// name their new IDs, so none is a mention, then the mentions say which requirements the code
/// has not caught up with, and the versions of the rest follow. A current marker of a
/// requirement whose text `references` rewrote keeps its version when a mention of an old ID
/// or slug rewritten in that text lies in a file its component owns or one holding its
/// markers, so the requirement is `behind` and its build brings the code to the new name.
fn rewrite_markers(
    root: &Path,
    done: &Done,
    resolves: &[String],
    writes: &mut Writes,
) -> Result<(), Failure> {
    let (graph, _) = clm_model::load(root);
    // Each ID the markers now cite, to the requirement as it was.
    let was: BTreeMap<&str, &Was> = done.before.iter().map(|w| (w.new.as_str(), w)).collect();
    let rewrite = |writes: &mut Writes, edit: &dyn Fn(&MarkerSpan) -> Option<String>| {
        let files: BTreeSet<PathBuf> = clm_coverage::scan(root)
            .req
            .into_iter()
            .map(|m| m.path)
            .collect();
        for file in files {
            let Ok(text) = fs::read_to_string(root.join(&file)) else {
                continue;
            };
            let mut out = String::with_capacity(text.len());
            let mut at = 0;
            for span in clm_coverage::marker_spans(&text) {
                if let Some(marker) = edit(&span) {
                    out.push_str(&text[at..span.range.start]);
                    out.push_str(&marker);
                    at = span.range.end;
                }
            }
            out.push_str(&text[at..]);
            if out != text {
                writes.put(&file, out.as_bytes())?;
            }
        }
        Ok::<(), Failure>(())
    };
    let marker = |id: &str, version: Option<&str>| match version {
        Some(version) => format!("req:{id}@{version}"),
        None => format!("req:{id}"),
    };
    let named: BTreeMap<&str, &str> = done
        .before
        .iter()
        .filter(|w| w.id != w.new)
        .map(|w| (w.id.as_str(), w.new.as_str()))
        .collect();
    rewrite(&mut *writes, &|span| {
        let new = named.get(span.id.as_str())?;
        Some(marker(new, span.version.as_deref()))
    })?;

    let skip: Vec<PathBuf> = resolves
        .iter()
        .map(|slug| Path::new("backlog").join(format!("{slug}.md")))
        .collect();
    let hits = leftovers(root, &done.olds, &skip)?;
    let holding: BTreeSet<(String, PathBuf)> = clm_coverage::scan(root)
        .req
        .into_iter()
        .map(|m| (m.id, m.path))
        .collect();
    let behind = |w: &Was| {
        let component = w.new.split('/').next().unwrap_or("");
        let mentioned = |hit: &&Hit| hit.kind != Mention::Term && w.rewritten.contains(&hit.old);
        hits.iter().filter(mentioned).any(|hit| {
            clm_coverage::owner(&graph, &hit.file) == Some(component)
                || holding.contains(&(w.new.clone(), hit.file.clone()))
        })
    };
    rewrite(&mut *writes, &|span| {
        let w = was.get(span.id.as_str())?;
        let now = graph.requirement(&w.new)?.version();
        let current = span.version.as_deref() == Some(w.version.as_str());
        (current && now != w.version && !behind(w)).then(|| marker(&w.new, Some(now)))
    })
}

/// Rewrites the `requirement` of each backlog item that records one (`flag-rerecord`), as
/// `rewrite_markers` rewrites a marker: `generate` already gave it the new ID, so a version
/// that was the requirement's before becomes its version after, any other stays.
fn rewrite_flags(root: &Path, done: &Done, writes: &mut Writes) -> Result<(), Failure> {
    let (graph, _) = clm_model::load(root);
    let mut current: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for was in &done.before {
        if let Some(now) = graph.requirement(&was.new).map(|r| r.version()) {
            current.insert(&was.new, (&was.version, now));
        }
    }
    for item in graph.backlog.values() {
        let Some(at) = &item.frontmatter.requirement else {
            continue;
        };
        let id = at.id.as_str();
        let Some((was, now)) = current.get(id).filter(|(was, now)| was != now) else {
            continue;
        };
        let Ok(text) = fs::read_to_string(root.join(&item.path)) else {
            continue;
        };
        let old = format!("requirement: {id}@{was}");
        let out = text.replacen(&old, &format!("requirement: {id}@{now}"), 1);
        if out != text {
            writes.put(&item.path, out.as_bytes())?;
        }
    }
    Ok(())
}

/// Whether `change`'s deltas still wait to be applied: a proposed or approved change that
/// isn't archived. An applied one is already in `specs/`.
fn unapplied(change: &clm_model::Change) -> bool {
    use clm_model::ChangeStatus::{Approved, Proposed};
    !change.archived && matches!(change.frontmatter.status, Proposed | Approved)
}

/// The live changes other than `id` whose deltas wait to be applied, in ID order.
fn others<'g>(graph: &'g Graph, id: &'g str) -> impl Iterator<Item = &'g Change> {
    graph
        .changes
        .values()
        .filter(move |c| c.id != id && unapplied(c))
}

fn held(graph: &Graph, id: &str) -> Held {
    let mut held = Held::default();
    for delta in others(graph, id).flat_map(|c| &c.deltas) {
        let Some(node) = graph.nodes.get(&delta.node) else {
            continue;
        };
        held.frontmatter
            .insert(node.id.clone(), node.hashes.frontmatter.clone());
        for (element, _) in clm_apply::stale_bases(Some(node), delta) {
            held.stale.insert((delta.path.clone(), element));
        }
    }
    held
}

/// `other-deltas`: rebases the elements of other live changes' deltas that the rename moved,
/// and names each in a `rebased` line, in change, component and `clm hashes` order.
fn rebase_others(
    root: &Path,
    id: &str,
    done: &Done,
    writes: &mut Writes,
) -> Result<Vec<String>, Failure> {
    let (graph, _) = clm_model::load(root);
    let mut lines = Vec::new();
    for change in others(&graph, id) {
        let mut deltas: Vec<&Delta> = change.deltas.iter().filter(|d| !d.create).collect();
        deltas.sort_by(|a, b| a.node.cmp(&b.node));
        for delta in deltas {
            let Some(node) = graph.nodes.get(&delta.node) else {
                continue;
            };
            let current = clm_apply::hashes(node);
            let fresh = |element: &str| current.iter().find(|(e, _)| e == element);
            let mut moved: Vec<(String, String)> = clm_apply::stale_bases(Some(node), delta)
                .into_iter()
                .filter(|(e, _)| !done.held.stale.contains(&(delta.path.clone(), e.clone())))
                .collect();
            let before = done.held.frontmatter.get(&delta.node);
            if let Some(base) = &delta.base.frontmatter
                && before == Some(base)
                && *base != node.hashes.frontmatter
            {
                moved.push(("frontmatter".into(), base.clone()));
            }
            moved.sort_by_key(|(e, _)| current.iter().position(|(c, _)| c == e));
            let original =
                fs::read_to_string(root.join(&delta.path)).map_err(|e| io(&delta.path, e))?;
            let mut text = original.clone();
            for (element, base) in moved {
                let Some((_, new)) = fresh(&element) else {
                    continue;
                };
                if let Some(edited) = rebase_element(&text, &element, &base, new, &done.edits) {
                    text = edited;
                    lines.push(format!(
                        "rebased {element} of {} in {}",
                        delta.node,
                        delta.path.display()
                    ));
                }
            }
            if text != original {
                writes.put(&delta.path, text.as_bytes())?;
            }
        }
    }
    Ok(lines)
}

/// `lines[range]` run through `edit` as one text, which keeps its line count.
fn edit_lines(lines: &mut [String], range: Range<usize>, edit: impl Fn(&str) -> String) {
    let edited = edit(&lines[range.clone()].concat());
    for (slot, line) in lines[range].iter_mut().zip(edited.split_inclusive('\n')) {
        *slot = line.to_string();
    }
}

/// The delta file `text` with `element` rebased: its recorded base `old` set to `new` and,
/// where the delta modifies it, its text run through the rename's edits. `None` when the
/// element or its base isn't where `clm-apply` reads it.
fn rebase_element(
    text: &str,
    element: &str,
    old: &str,
    new: &str,
    edits: &Edits,
) -> Option<String> {
    let mut lines: Vec<String> = text.split_inclusive('\n').map(String::from).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let sections = section_spans(&refs);
    let section = |name: &str| sections.iter().find(|s| s.name == name);
    let plain = BTreeMap::new();
    if let Some(slug) = element.strip_prefix("requirement:") {
        let (modifies, span) = [
            ("MODIFIED Requirements", true),
            ("REMOVED Requirements", false),
        ]
        .into_iter()
        .filter_map(|(name, modifies)| Some((modifies, section(name)?)))
        .find_map(|(modifies, s)| {
            let spans = requirement_spans(&refs, s.heading + 1..s.end);
            let found = spans
                .into_iter()
                .find(|r| r.slug == slug && r.base.as_deref() == Some(old));
            found.map(|r| (modifies, r))
        })?;
        let heading = span.lines.start;
        let rebased = lines[heading].replacen(old, new, 1);
        if modifies {
            edit_lines(&mut lines, span.lines, |t| edits.body(t, &plain));
        }
        lines[heading] = rebased;
        return Some(lines.concat());
    }
    let first = refs.first()?.trim_end() == "---";
    let end = 1 + refs.iter().skip(1).position(|l| l.trim_end() == "---")?;
    let at = (1..end).find(|i| set_base(&lines[*i], element, old, new).is_some())?;
    if !first {
        return None;
    }
    lines[at] = set_base(&lines[at], element, old, new)?;
    let name = match element {
        "frontmatter" => "MODIFIED Frontmatter",
        "contract" => "MODIFIED Contract",
        _ => "MODIFIED Design",
    };
    if let Some(s) = section(name) {
        let range = s.heading + 1..s.end;
        if element == "frontmatter" {
            let keys = ["depends_on", "serves", "successor"];
            edit_lines(&mut lines, range, |t| edits.values(t, &keys));
        } else {
            edit_lines(&mut lines, range, |t| edits.body(t, &plain));
        }
    }
    Some(lines.concat())
}

/// `line` with the pair `<key>: <old>`, quoted either way or bare, set to `<new>`, where the
/// key starts the text or follows `{`, `,` or a space.
fn set_base(line: &str, key: &str, old: &str, new: &str) -> Option<String> {
    for quote in ["\"", "'", ""] {
        let pair = format!("{key}: {quote}{old}{quote}");
        let mut from = 0;
        while let Some(i) = line[from..].find(&pair).map(|i| i + from) {
            let before = line[..i].chars().next_back();
            if before.is_none_or(|c| matches!(c, '{' | ',' | ' ' | '\t')) {
                let mut out = line.to_string();
                out.replace_range(i..i + pair.len(), &format!("{key}: {quote}{new}{quote}"));
                return Some(out);
            }
            from = i + 1;
        }
    }
    None
}

/// The old name and the new one as `renamed <old> -> <new>` prints them.
fn pair(rename: &Rename) -> (String, String) {
    let new = match rename.kind {
        RenameKind::Requirement => format!(
            "{}/{}",
            rename.name.split('/').next().unwrap_or(""),
            rename.to
        ),
        _ => rename.to.clone(),
    };
    (rename.name.clone(), new)
}

/// The slug of requirement ID `id`.
fn slug(id: &str) -> &str {
    id.split_once('/').map_or(id, |(_, s)| s)
}

fn spec_path(node: &str) -> PathBuf {
    if node == ROOT_ID {
        PathBuf::from("specs/SPEC.md")
    } else {
        Path::new("specs")
            .join(node.replace('.', "/"))
            .join("SPEC.md")
    }
}

/// What the recorded renames come to in the loaded graph.
struct Plan {
    edits: Edits,
    /// Each renamed component, descendants included, to its new ID.
    components: BTreeMap<String, String>,
    /// Each renamed requirement or entry ID to its new ID.
    requirements: BTreeMap<String, String>,
    renamed: Vec<(String, String)>,
}

fn plan(graph: &Graph, id: &str, renames: &[Rename]) -> Result<Plan, Failure> {
    let mut ids = BTreeMap::new();
    let mut slugs = BTreeMap::new();
    let mut terms = Vec::new();
    let mut components = BTreeMap::new();
    let mut requirements = BTreeMap::new();
    for rename in renames {
        let (from, to) = (&rename.name, &rename.to);
        match rename.kind {
            RenameKind::Component => {
                if !graph.nodes.contains_key(from) {
                    return refuse(format!("no component `{from}`"));
                }
                if graph.nodes.contains_key(to) || components.values().any(|n| n == to) {
                    return refuse(format!("component `{to}` already exists"));
                }
                let under = format!("{from}.");
                for node in graph
                    .nodes
                    .values()
                    .filter(|n| n.id == *from || n.id.starts_with(&under))
                {
                    let new = format!("{to}{}", &node.id[from.len()..]);
                    for r in &node.requirements {
                        ids.insert(r.id.clone(), format!("{new}/{}", slug(&r.id)));
                    }
                    ids.insert(node.id.clone(), new.clone());
                    components.insert(node.id.clone(), new);
                }
            }
            RenameKind::Requirement => {
                if graph.requirement(from).is_none() {
                    return refuse(format!("no requirement `{from}`"));
                }
                let new = format!("{}/{to}", from.split('/').next().unwrap_or(""));
                if graph.requirement(&new).is_some() || requirements.values().any(|n| *n == new) {
                    return refuse(format!("requirement `{new}` already exists"));
                }
                let others = graph
                    .requirements()
                    .filter(|r| r.id != *from && slug(&r.id) == slug(from))
                    .count();
                if others == 0 && !graph.decisions.contains_key(slug(from)) {
                    slugs.insert(slug(from).to_string(), to.clone());
                }
                ids.insert(from.clone(), new.clone());
                requirements.insert(from.clone(), new);
            }
            RenameKind::Term => terms.push((from.clone(), to.clone())),
        }
    }
    // A rule of a renamed component would be renamed twice, in an order the record can't
    // say, so the two go in separate changes.
    for old in requirements.keys() {
        let node = old.split('/').next().unwrap_or("");
        if components.contains_key(node) {
            return refuse(format!(
                "`{old}` and its component `{node}` are both renamed; rename them in separate changes"
            ));
        }
    }
    // Another live change's deltas were written against the old names, and would no
    // longer apply once they are gone.
    for other in graph
        .changes
        .values()
        .filter(|c| c.id != id && unapplied(c))
    {
        for delta in &other.deltas {
            if components.contains_key(&delta.node) {
                return refuse(format!(
                    "change `{}` has a delta for `{}`, which is renamed",
                    other.id, delta.node
                ));
            }
            let touched = delta.modified.iter().chain(&delta.removed);
            for r in touched {
                let target = format!("{}/{}", delta.node, r.slug);
                if requirements.contains_key(&target) {
                    return refuse(format!(
                        "change `{}` modifies or removes `{target}`, which is renamed",
                        other.id
                    ));
                }
            }
        }
    }
    Ok(Plan {
        edits: Edits::new(ids, slugs, terms),
        components,
        requirements,
        renamed: renames.iter().map(pair).collect(),
    })
}

/// The headings that rename requirements of `owner`'s file: `### <word>: <old>` to new.
fn headings(plan: &Plan, owner: &str, heading: &str) -> BTreeMap<String, String> {
    plan.requirements
        .iter()
        .filter(|(old, _)| old.split_once('/').is_some_and(|(n, _)| n == owner))
        .map(|(old, new)| {
            (
                format!("{heading} {}", slug(old)),
                format!("{heading} {}", slug(new)),
            )
        })
        .collect()
}

/// Writes the change's deltas and assets, and the human layers and backlog items the
/// renames edit, so `apply` finds a graph that holds the new names.
fn generate(
    root: &Path,
    id: &str,
    renames: &[Rename],
    writes: &mut Writes,
) -> Result<Done, Failure> {
    let (graph, _) = clm_model::load(root);
    let plan = plan(&graph, id, renames)?;
    let declared = graph.declared_frontmatter_keys();
    let deltas = Path::new("changes").join(id).join("deltas");
    writes.clear(&deltas)?;
    for node in graph.nodes.values() {
        let heads = headings(&plan, &node.id, "### Requirement:");
        let (fm, body) = split_frontmatter(&node.source).unwrap_or(("", &node.source));
        let fm = plan
            .edits
            .values(fm, &["depends_on", "serves", "successor"]);
        let source = format!("---\n{fm}\n---\n{}", plan.edits.body(body, &heads));
        let delta = match plan.components.get(&node.id) {
            Some(new_id) => {
                let new = clm_model::parse_node(new_id, &spec_path(new_id), &source, &declared)
                    .0
                    .ok_or_else(|| Failure::Refused(format!("`{new_id}` doesn't parse")))?;
                copy_assets(root, id, node, new_id, writes)?;
                let name = format!("{new_id}.md");
                writes.put(&deltas.join(name), create(&new).as_bytes())?;
                Some(removal(node))
            }
            None if source != node.source => modification(node, &source, &declared, &plan.edits),
            None => None,
        };
        if let Some(text) = delta {
            writes.put(&deltas.join(format!("{}.md", node.id)), text.as_bytes())?;
        }
    }
    for layer in Layer::ALL {
        let file = Path::new(layer.file());
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue;
        };
        let word = graph.heading(layer);
        let at = text
            .match_indices("### ")
            .find(|(i, _)| *i == 0 || text.as_bytes()[i - 1] == b'\n')
            .map_or(text.len(), |(i, _)| i);
        let entries = plan
            .edits
            .body(&text[at..], &headings(&plan, layer.id(), word));
        let new = format!("{}{entries}", &text[..at]);
        if new != text {
            writes.put(file, new.as_bytes())?;
        }
    }
    for item in graph.backlog.values() {
        let Ok(text) = fs::read_to_string(root.join(&item.path)) else {
            continue;
        };
        let Some((fm, body)) = split_frontmatter(&text) else {
            continue;
        };
        let fm = plan
            .edits
            .values(fm, &["components", "blocks", "requirement"]);
        let new = format!("---\n{fm}\n---\n{body}");
        if new != text {
            writes.put(&item.path, new.as_bytes())?;
        }
    }
    if let Some(term) = plan.edits.unswapped().first() {
        return refuse(format!("term `{term}` occurs nowhere in spec text"));
    }
    let olds = plan
        .edits
        .olds()
        .map(|(old, kind)| (old.to_string(), kind))
        .collect();
    let before = graph
        .requirements()
        .map(|r| Was {
            id: r.id.clone(),
            new: plan.edits.id(&r.id).unwrap_or(&r.id).to_string(),
            version: r.version().to_string(),
            rewritten: plan.edits.rewritten(&r.text),
        })
        .collect();
    Ok(Done {
        renamed: plan.renamed,
        olds,
        before,
        held: held(&graph, id),
        edits: plan.edits,
        rebased: Vec::new(),
    })
}

/// Copies a renamed node's assets to the change's, under the new ID.
fn copy_assets(
    root: &Path,
    id: &str,
    node: &Node,
    new_id: &str,
    writes: &mut Writes,
) -> Result<(), Failure> {
    let from = node.path.with_file_name("assets");
    let to = Path::new("changes").join(id).join("assets").join(new_id);
    for path in node.assets.keys() {
        let bytes = fs::read(root.join(&from).join(path)).map_err(|e| io(&from.join(path), e))?;
        writes.put(&to.join(path), &bytes)?;
    }
    Ok(())
}

/// A requirement block headed `### Requirement: <slug>`, with its base when it has one.
fn block(text: &str, base: Option<&str>) -> String {
    let mut lines = text.trim_end().lines();
    let heading = lines.next().unwrap_or("");
    let rest: Vec<&str> = lines.collect();
    let heading = base.map_or(heading.to_string(), |b| {
        format!("{heading} <!-- base: {b} -->")
    });
    format!("{heading}\n{}\n\n", rest.join("\n"))
}

/// The delta that removes `node`, listing everything it holds with its base.
fn removal(node: &Node) -> String {
    let mut text = format!(
        "---\ncomponent: {}\nremove: true\nbase: {{ frontmatter: \"{}\", contract: \"{}\", design: \"{}\" }}\n---\n",
        node.id, node.hashes.frontmatter, node.hashes.contract, node.hashes.design
    );
    text.push_str("## REMOVED Requirements\n");
    for r in &node.requirements {
        let heading = format!(
            "### Requirement: {} <!-- base: {} -->\n",
            slug(&r.id),
            r.hash
        );
        text.push_str(&heading);
    }
    if !node.assets.is_empty() {
        text.push_str("\n## REMOVED Assets\n");
        for (path, hash) in &node.assets {
            text.push_str(&format!("- `{path}` <!-- base: {hash} -->\n"));
        }
    }
    text
}

/// The delta that creates `node`, with the frontmatter, Contract, Design and requirements
/// it holds.
fn create(node: &Node) -> String {
    let fm = split_frontmatter(&node.source).map_or("", |(fm, _)| fm);
    let mut text = format!(
        "---\ncomponent: {}\ncreate: true\n---\n## Frontmatter\n```yaml\n{fm}\n```\n\n## Contract\n{}\n\n## Design\n{}\n\n## ADDED Requirements\n\n",
        node.id,
        node.contract.trim(),
        node.design.trim()
    );
    for r in &node.requirements {
        text.push_str(&block(&r.text, None));
    }
    text
}

/// The delta that changes `old` to what `source` holds, or none when no element differs.
fn modification(
    old: &Node,
    source: &str,
    declared: &BTreeSet<String>,
    edits: &Edits,
) -> Option<String> {
    let new = clm_model::parse_node(&old.id, &old.path, source, declared).0?;
    let old_fm = split_frontmatter(&old.source).map_or("", |(fm, _)| fm);
    let new_fm = split_frontmatter(source).map_or("", |(fm, _)| fm);
    let changed: Vec<&str> = new_fm
        .lines()
        .filter(|l| !old_fm.lines().any(|o| o == *l))
        .collect();
    let (mut bases, mut text) = (Vec::new(), String::new());
    if !changed.is_empty() {
        bases.push(format!("frontmatter: \"{}\"", old.hashes.frontmatter));
        text.push_str(&format!(
            "## MODIFIED Frontmatter\n```yaml\n{}\n```\n\n",
            changed.join("\n")
        ));
    }
    for (name, was, now, hash) in [
        (
            "Contract",
            &old.contract,
            &new.contract,
            &old.hashes.contract,
        ),
        ("Design", &old.design, &new.design, &old.hashes.design),
    ] {
        if was != now {
            bases.push(format!("{}: \"{hash}\"", name.to_lowercase()));
            text.push_str(&format!("## MODIFIED {name}\n{}\n\n", now.trim()));
        }
    }
    let (mut removed, mut added, mut modified) = (String::new(), String::new(), String::new());
    for r in &old.requirements {
        let target = edits.id(&r.id).unwrap_or(&r.id);
        let Some(now) = new.requirements.iter().find(|n| n.id == target) else {
            continue;
        };
        if target != r.id {
            let heading = format!(
                "### Requirement: {} <!-- base: {} -->\n",
                slug(&r.id),
                r.hash
            );
            removed.push_str(&heading);
            added.push_str(&block(&now.text, None));
        } else if now.text != r.text {
            modified.push_str(&block(&now.text, Some(&r.hash)));
        }
    }
    for (title, section) in [
        ("REMOVED", removed),
        ("ADDED", added),
        ("MODIFIED", modified),
    ] {
        if !section.is_empty() {
            text.push_str(&format!("## {title} Requirements\n{section}"));
            if title == "REMOVED" {
                text.push('\n');
            }
        }
    }
    if text.is_empty() {
        return None;
    }
    let base = if bases.is_empty() {
        String::new()
    } else {
        format!("base: {{ {} }}\n", bases.join(", "))
    };
    Some(format!("---\ncomponent: {}\n{base}---\n{text}", old.id))
}

/// A mention of an old name that a rename left.
pub(crate) struct Hit {
    pub(crate) old: String,
    pub(crate) kind: Mention,
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
}

/// The tracked files under `root`; every file but git's and a build's when `root` is no
/// checkout, as `replays` extracts one.
fn text_files(root: &Path) -> Result<Vec<PathBuf>, Failure> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if !matches!(entry.file_name().to_str(), Some(".git" | "target")) {
                    walk(root, &path, out);
                }
            } else if kind.is_file()
                && let Ok(rel) = path.strip_prefix(root)
            {
                out.push(rel.to_path_buf());
            }
        }
    }
    if root.join(".git").exists() {
        return clm_git::tracked_files(root).map_err(|e| Failure::Git(e.to_string()));
    }
    let mut files = Vec::new();
    walk(root, root, &mut files);
    Ok(files)
}

/// Each mention of an old name in the tracked text files that hold the author's prose and
/// code, outside `skip`, sorted by file and line (`finish`).
fn leftovers(
    root: &Path,
    olds: &[(String, Mention)],
    skip: &[PathBuf],
) -> Result<Vec<Hit>, Failure> {
    let history = |p: &Path| {
        ["specs", "changes", "docs/decisions"]
            .iter()
            .any(|d| p.starts_with(d))
            || p == Path::new("ARCHITECTURE.md")
            || p == Path::new("CAPABILITIES.md")
            || skip.iter().any(|s| s == p)
    };
    let mut hits = Vec::new();
    for file in text_files(root)?.into_iter().filter(|p| !history(p)) {
        let Ok(text) = fs::read_to_string(root.join(&file)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            for (old, kind) in olds.iter().filter(|(old, kind)| mentions(line, old, *kind)) {
                hits.push(Hit {
                    old: old.clone(),
                    kind: *kind,
                    file: file.clone(),
                    line: n + 1,
                });
            }
        }
    }
    hits.sort_by(|a, b| (&a.file, a.line, &a.old).cmp(&(&b.file, b.line, &b.old)));
    Ok(hits)
}
