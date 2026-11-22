//! `stale-delta-merge` (`stale-delta-three-way-merge`): a stale delta's sections whose edits
//! don't overlap what moved them since their base, `main`'s edits or an earlier change's,
//! merged three-way into the delta with their new base, so the caller resolves only what is
//! left; and, first, a base only this branch's lost tree had taken back to the one before the
//! commit that set it (`other-delta-merge`). `Tree::other_deltas` runs the same for another
//! live change's delta. Both work in the temporary tree, and each change's delta files reach
//! `root` in one commit (`Edits::finish`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use clm_git::Options;
use clm_model::outline::{requirement_spans, section_spans};
use clm_model::{ARCHIVE_DIR, Change, Delta};

use crate::stages::{Movers, Stages};
use crate::stale::text;
use crate::{Error, exists_at, git, io, run};

/// One element of a delta file that a take-back or a merge rewrote, in the order made.
struct Made {
    path: PathBuf,
    node: String,
    element: String,
    /// What moved it, when the merge merged it; `None` for a take-back.
    movers: Option<Movers>,
}

/// What a take-back and a merge did to one change's delta files in the temporary tree,
/// written to `root` and committed once the change is reached (`finish`).
pub(crate) struct Edits {
    change: String,
    /// Each delta file rewritten, CLM-root-relative, with its text as `HEAD` has it.
    head: BTreeMap<PathBuf, String>,
    made: Vec<Made>,
}

impl Edits {
    pub(crate) fn new(change: &str) -> Self {
        Edits {
            change: change.to_string(),
            head: BTreeMap::new(),
            made: Vec::new(),
        }
    }

    /// Records that `path`, whose text was `original` before this change's first edit to it,
    /// is about to be rewritten.
    fn touch(&mut self, path: &Path, original: &str) {
        self.head
            .entry(path.to_path_buf())
            .or_insert_with(|| original.to_string());
    }

    /// Writes into `root` each delta file whose text in `tree` now differs from `HEAD`'s, and
    /// commits them, and nothing else, in one commit: `Merge <movers> into <change>` when a
    /// `merged` line names one of them, else `Take back what this branch merged into
    /// <change>`. Returns, for each element whose text or base differs from `HEAD`'s copy,
    /// in the order first made, `merged <element> of <node> into <path>` when the merge
    /// merged it, else `took back <element> of <node> in <path> to base <base>`.
    pub(crate) fn finish(self, root: &Path, tree: &Path) -> Result<Vec<String>, Error> {
        let mut finals = BTreeMap::new();
        for path in self.head.keys() {
            let at = tree.join(path);
            finals.insert(path, fs::read_to_string(&at).map_err(|e| io(&at, e))?);
        }
        let mut lines = Vec::new();
        let mut movers = Movers::default();
        let mut merged_any = false;
        let mut done: Vec<(&Path, &str)> = Vec::new();
        for m in &self.made {
            if done.contains(&(m.path.as_path(), m.element.as_str())) {
                continue;
            }
            done.push((&m.path, &m.element));
            let now = element_in(&m.path, &finals[&m.path], &m.element);
            if now == element_in(&m.path, &self.head[&m.path], &m.element) {
                continue;
            }
            let path = m.path.display();
            let merged: Vec<&Movers> = self
                .made
                .iter()
                .filter(|o| o.path == m.path && o.element == m.element)
                .filter_map(|o| o.movers.as_ref())
                .collect();
            if merged.is_empty() {
                let base = now.and_then(|(b, _)| b).unwrap_or_default();
                lines.push(format!(
                    "took back {} of {} in {path} to base {base}",
                    m.element, m.node
                ));
            } else {
                merged_any = true;
                merged.into_iter().for_each(|o| movers.extend(o));
                lines.push(format!("merged {} of {} into {path}", m.element, m.node));
            }
        }
        let files: Vec<String> = self
            .head
            .iter()
            .filter(|(path, text)| finals[path] != **text)
            .map(|(path, _)| path.to_string_lossy().into_owned())
            .collect();
        if files.is_empty() {
            return Ok(lines);
        }
        for file in &files {
            let path = root.join(file);
            let text = &finals[&PathBuf::from(file)];
            fs::write(&path, text).map_err(|e| io(&path, e))?;
        }
        let mut add = vec!["add", "--"];
        add.extend(files.iter().map(String::as_str));
        git(root, &add)?;
        let message = if merged_any {
            format!("Merge {} into {}", movers.join(), self.change)
        } else {
            format!("Take back what this branch merged into {}", self.change)
        };
        let mut commit = vec!["commit", "-q", "-m", &message, "--"];
        commit.extend(files.iter().map(String::as_str));
        git(root, &commit)?;
        Ok(lines)
    }
}

/// `Error::Refused` naming the first delta file of `changes` that differs in `root`'s working
/// tree or index from `HEAD`, before anything is written: a merge or a take-back would
/// overwrite the edit.
pub(crate) fn refuse_uncommitted<'a>(
    root: &Path,
    changes: impl Iterator<Item = &'a str>,
) -> Result<(), Error> {
    let mut paths = Vec::new();
    for id in changes {
        paths.push(format!("changes/{id}/deltas"));
        paths.push(format!("changes/{ARCHIVE_DIR}/{id}/deltas"));
    }
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["status", "--porcelain", "-z", "--untracked-files=no", "--"];
    args.extend(paths.iter().map(String::as_str));
    let out = run(root, &args, Options::default())?;
    let first = out.split(|&b| b == 0).find(|entry| entry.len() > 3);
    match first {
        Some(entry) => Err(Error::Refused(format!(
            "{} differs from `HEAD`: commit the edit or undo it, then run again",
            String::from_utf8_lossy(&entry[3..])
        ))),
        None => Ok(()),
    }
}

/// The delta file at `path` (CLM-root-relative, under its change's directory) with `text`,
/// parsed as `clm-model` parses it.
fn parse_delta(path: &Path, text: &str) -> Option<Delta> {
    let dir = path.parent()?.parent()?;
    let mut files = clm_model::Files::new();
    let stub = "---\nstatus: applied\n---\n\n# c\n";
    files.insert(dir.join("change.md"), stub.as_bytes().to_vec());
    files.insert(path.to_path_buf(), text.as_bytes().to_vec());
    let (graph, _) = clm_model::load_files(&files);
    let change = graph.changes.into_values().next()?;
    change.deltas.into_iter().find(|d| d.path == path)
}

/// The recorded base and the text of `element`, a Contract, Design or requirement `delta`
/// MODIFIES; `None` when it doesn't modify it.
fn modified<'d>(delta: &'d Delta, element: &str) -> Option<(Option<&'d str>, &'d str)> {
    match element {
        "contract" => delta
            .modified_contract
            .as_deref()
            .map(|t| (delta.base.contract.as_deref(), t)),
        "design" => delta
            .modified_design
            .as_deref()
            .map(|t| (delta.base.design.as_deref(), t)),
        _ => {
            let slug = element.strip_prefix("requirement:")?;
            delta
                .modified
                .iter()
                .find(|r| r.slug == slug)
                .map(|r| (r.base.as_deref(), r.text.as_str()))
        }
    }
}

/// `modified` of the delta file at `path` with `text`, owned.
fn element_in(path: &Path, text: &str, element: &str) -> Option<(Option<String>, String)> {
    let delta = parse_delta(path, text)?;
    modified(&delta, element).map(|(b, t)| (b.map(str::to_string), t.to_string()))
}

/// The commits `HEAD` holds and `onto` doesn't that changed one delta file, newest first,
/// followed from `changes/archive/<id>/` back to `changes/<id>/`.
struct DeltaHistory {
    commits: Vec<String>,
    /// The file's place in an archived change, then in a live one, CLM-root-relative.
    places: [PathBuf; 2],
}

impl DeltaHistory {
    fn new(root: &Path, onto: &str, change: &Change, delta: &Delta) -> Result<Self, Error> {
        let dir = change.path.parent().unwrap_or(Path::new(""));
        let rest = delta.path.strip_prefix(dir).unwrap_or(&delta.path);
        let changes = Path::new("changes");
        let places = [
            changes.join(ARCHIVE_DIR).join(&change.id).join(rest),
            changes.join(&change.id).join(rest),
        ];
        let range = format!("{onto}..HEAD");
        let mut args = vec!["log", "--full-history", "--format=%H", &range, "--"];
        let names: Vec<String> = places
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        args.extend(names.iter().map(String::as_str));
        let log = git(root, &args)?;
        Ok(DeltaHistory {
            commits: log.lines().map(str::to_string).collect(),
            places,
        })
    }

    /// `element`'s recorded base and text in the file at `rev`, read where the file is then.
    fn element_at(
        &self,
        root: &Path,
        rev: &str,
        element: &str,
    ) -> Result<Option<(Option<String>, String)>, Error> {
        for place in &self.places {
            let relative = format!("./{}", place.display());
            if !exists_at(root, rev, &relative)? {
                continue;
            }
            let spec = format!("{rev}:{relative}");
            let bytes = run(root, &["cat-file", "blob", &spec], Options::default())?;
            return Ok(element_in(place, &String::from_utf8_lossy(&bytes), element));
        }
        Ok(None)
    }

    /// The delta file `file` with `element`, whose recorded base `base` is unknown, taken
    /// back past the newest commit that set it to `base` from another, and again while the
    /// base so restored is unknown; `None`, leaving it as it was, when a step conflicts or
    /// no commit set the base.
    #[allow(clippy::too_many_arguments)]
    fn take_back(
        &self,
        root: &Path,
        stages: &Stages,
        onto: &str,
        delta: &Delta,
        file: &str,
        element: &str,
        base: &str,
    ) -> Result<Option<String>, Error> {
        let (mut file, mut base) = (file.to_string(), base.to_string());
        for commit in &self.commits {
            let Some((Some(at), set)) = self.element_at(root, commit, element)? else {
                continue;
            };
            let parent = format!("{commit}^");
            let before = self.element_at(root, &parent, element)?;
            let Some((Some(before_base), before_text)) = before else {
                continue;
            };
            if at != base || before_base == base {
                continue;
            }
            let Some((_, now)) = element_in(&delta.path, &file, element) else {
                return Ok(None);
            };
            let Some(text) = merge_file(root, &now, &set, &before_text)? else {
                return Ok(None);
            };
            let Some(edited) = rewrite(&file, element, &text, &base, &before_base) else {
                return Ok(None);
            };
            (file, base) = (edited, before_base);
            if stages
                .base_text(root, onto, &delta.node, element, &base)?
                .is_some()
            {
                return Ok(Some(file));
            }
        }
        Ok(None)
    }
}

/// `stale-delta-merge`'s take-back, run as `change` is reached and before it is judged: for
/// each delta of `change` in node-ID order, and each Contract, Design or requirement it
/// MODIFIES whose recorded base is unknown against `stages` (no stage and no commit of
/// `onto`'s history has its text), the element taken back as `DeltaHistory::take_back`
/// does, in `tree`. A file with CRLF endings is left alone. Returns whether a file changed.
pub(crate) fn take_back(
    root: &Path,
    tree: &Path,
    stages: &Stages,
    change: &Change,
    onto: &str,
    edits: &mut Edits,
) -> Result<bool, Error> {
    let graph = stages.last();
    let mut deltas: Vec<&Delta> = change.deltas.iter().filter(|d| !d.remove).collect();
    deltas.sort_by(|a, b| a.node.cmp(&b.node));
    let mut changed = false;
    for delta in deltas {
        let node = graph.nodes.get(&delta.node);
        let mut unknown = Vec::new();
        for (element, base) in clm_apply::stale_bases(node, delta) {
            if modified(delta, &element).is_some()
                && stages
                    .base_text(root, onto, &delta.node, &element, &base)?
                    .is_none()
            {
                unknown.push((element, base));
            }
        }
        if unknown.is_empty() {
            continue;
        }
        let in_tree = tree.join(&delta.path);
        let original = fs::read_to_string(&in_tree).map_err(|e| io(&in_tree, e))?;
        if original.contains('\r') {
            continue;
        }
        let history = DeltaHistory::new(root, onto, change, delta)?;
        let mut file = original.clone();
        for (element, base) in unknown {
            if let Some(taken) =
                history.take_back(root, stages, onto, delta, &file, &element, &base)?
            {
                file = taken;
                edits.made.push(Made {
                    path: delta.path.clone(),
                    node: delta.node.clone(),
                    element,
                    movers: None,
                });
            }
        }
        if file != original {
            edits.touch(&delta.path, &original);
            fs::write(&in_tree, &file).map_err(|e| io(&in_tree, e))?;
            changed = true;
        }
    }
    Ok(changed)
}

/// For each delta of `change` in node-ID order, and each Contract, Design or requirement it
/// MODIFIES whose recorded base `apply` judges stale against the tree it is applied to (the
/// last of `stages`, loaded from `tree`; `stale-elements`), whose element is still there and
/// whose base text a stage or `onto`'s history has, in `clm hashes` order: the delta's text
/// merged three-way with the base's and the tree's. Each clean merge is written into the
/// delta in `tree`, with the element's base set to its hash in the tree, and recorded in
/// `edits`. Returns whether a file changed.
pub(crate) fn merge(
    root: &Path,
    tree: &Path,
    stages: &Stages,
    change: &Change,
    onto: &str,
    edits: &mut Edits,
) -> Result<bool, Error> {
    let graph = stages.last();
    let mut deltas: Vec<&Delta> = change.deltas.iter().filter(|d| !d.remove).collect();
    deltas.sort_by(|a, b| a.node.cmp(&b.node));
    let mut changed = false;
    for delta in deltas {
        let id = delta.node.as_str();
        let Some(node) = graph.nodes.get(id) else {
            continue;
        };
        let current = clm_apply::hashes(node);
        let hash = |element: &str| {
            current
                .iter()
                .find(|(e, _)| e == element)
                .map(|(_, h)| h.clone())
        };
        let mut stale: Vec<(String, &str, &str)> = Vec::new();
        for (element, base, mine) in [
            ("contract", &delta.base.contract, &delta.modified_contract),
            ("design", &delta.base.design, &delta.modified_design),
        ] {
            if let (Some(base), Some(mine)) = (base, mine) {
                stale.push((element.to_string(), base, mine));
            }
        }
        for r in &delta.modified {
            if let Some(base) = &r.base {
                stale.push((format!("requirement:{}", r.slug), base, &r.text));
            }
        }
        // `stale-elements`: the elements `apply` judges stale, not a comparison of hashes.
        let judged = clm_apply::stale_bases(Some(node), delta);
        stale.retain(|(element, base, _)| {
            judged.iter().any(|(e, b)| e == element && b == base)
                && hash(element).is_some()
                && text(node, element).is_some_and(|t| !t.trim().is_empty())
        });
        stale.sort_by_key(|(element, _, _)| current.iter().position(|(e, _)| e == element));
        if stale.is_empty() {
            continue;
        }
        let in_tree = tree.join(&delta.path);
        let original = fs::read_to_string(&in_tree).map_err(|e| io(&in_tree, e))?;
        // Rewritten line by line, so a file with CRLF endings would change throughout.
        if original.contains('\r') {
            continue;
        }
        let mut file = original.clone();
        for (element, base, mine) in stale {
            let Some(old) = stages.base_text(root, onto, id, &element, base)? else {
                continue;
            };
            let theirs = text(node, &element).unwrap_or_default();
            let Some(text) = merge_file(root, mine, &old, &theirs)? else {
                continue;
            };
            let new_base = hash(&element).expect("retained above");
            if let Some(edited) = rewrite(&file, &element, &text, base, &new_base) {
                file = edited;
                edits.made.push(Made {
                    path: delta.path.clone(),
                    node: id.to_string(),
                    movers: Some(stages.movers(id, &element, base)),
                    element,
                });
            }
        }
        if file != original {
            edits.touch(&delta.path, &original);
            fs::write(&in_tree, &file).map_err(|e| io(&in_tree, e))?;
            changed = true;
        }
    }
    Ok(changed)
}

/// `stale-delta-merge`: once the regenerate applied `regenerated` and the caller committed
/// `Re-apply`, with the regenerated tree, loaded from `tree`, as the last of `stages`. For
/// each live `proposed` or `approved` change it didn't regenerate, in ID order: its deltas
/// taken back, then, when `apply` refuses it for a stale base, merged as `merge` merges
/// them, and committed once (`Edits::finish`). The first change still refused stops it, with
/// the change's violations and the stale-delta report. Returns the `merged` and `took back`
/// lines and what is left.
pub(crate) fn other_deltas(
    root: &Path,
    tree: &Path,
    stages: &mut Stages,
    regenerated: &[String],
    onto: &str,
) -> Result<(Vec<String>, Vec<String>), Error> {
    let others: Vec<String> = stages
        .last()
        .changes
        .values()
        .filter(|c| !c.archived && c.frontmatter.status.is_pending())
        .filter(|c| !regenerated.contains(&c.id))
        .map(|c| c.id.clone())
        .collect();
    refuse_uncommitted(root, others.iter().map(String::as_str))?;
    let refusal = |graph: &clm_model::Graph, all, id: &str| {
        let mut v = crate::own_violations(graph, all, id);
        if let Some(change) = graph.changes.get(id) {
            v.extend(clm_apply::validate(graph, change));
        }
        v
    };
    // The load's violations of the tree as the regenerate left it.
    let (_, mut all) = clm_model::load(tree);
    let mut lines = Vec::new();
    for id in others {
        let mut edits = Edits::new(&id);
        if let Some(change) = stages.last().changes.get(&id).cloned()
            && take_back(root, tree, stages, &change, onto, &mut edits)?
        {
            let (graph, reloaded) = clm_model::load(tree);
            stages.reload(graph);
            all = reloaded;
        }
        let mut v = refusal(stages.last(), all.clone(), &id);
        if v.iter().any(|v| v.rule == "apply/stale-base")
            && let Some(change) = stages.last().changes.get(&id).cloned()
            && merge(root, tree, stages, &change, onto, &mut edits)?
        {
            let (graph, reloaded) = clm_model::load(tree);
            stages.reload(graph);
            all = reloaded;
            v = refusal(stages.last(), all.clone(), &id);
        }
        lines.extend(edits.finish(root, tree)?);
        if !v.is_empty() {
            let mut left: Vec<String> = v.iter().map(clm_check::format).collect();
            if let Some(change) = stages.last().changes.get(&id) {
                left.extend(crate::stale::report(root, stages, change, onto)?);
            }
            return Ok((lines, left));
        }
    }
    Ok((lines, Vec::new()))
}

/// `text` without its leading and trailing blank lines, ending in one newline (empty when
/// it is all blank).
fn trimmed(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let last = lines.iter().rposition(|l| !l.trim().is_empty());
    match (first, last) {
        (Some(first), Some(last)) => clm_model::outline::join(&lines[first..=last]),
        _ => String::new(),
    }
}

/// `line` with `<key>: "<old>"` set to `<key>: "<new>"`, where the key starts the line's
/// text or follows `{`, `,` or a space, so `contract` never matches `old_contract`; `None`
/// when the line holds no such pair.
fn set_base(line: &str, key: &str, old: &str, new: &str) -> Option<String> {
    let pair = format!("{key}: \"{old}\"");
    let mut from = 0;
    while let Some(i) = line[from..].find(&pair).map(|i| i + from) {
        let before = line[..i].chars().next_back();
        if before.is_none_or(|c| matches!(c, '{' | ',' | ' ' | '\t')) {
            let mut out = line.to_string();
            out.replace_range(i..i + pair.len(), &format!("{key}: \"{new}\""));
            return Some(out);
        }
        from = i + 1;
    }
    None
}

/// `git merge-file` of the delta's text, the base's and `main`'s: the merged text, or `None`
/// when the edits conflict.
fn merge_file(root: &Path, mine: &str, base: &str, theirs: &str) -> Result<Option<String>, Error> {
    let tmp = tempfile::TempDir::new().map_err(|e| Error::Git(format!("temp dir: {e}")))?;
    let mut paths = Vec::new();
    for (name, text) in [("delta", mine), ("base", base), ("main", theirs)] {
        let path = tmp.path().join(name);
        fs::write(&path, trimmed(text)).map_err(|e| io(&path, e))?;
        paths.push(path.to_string_lossy().into_owned());
    }
    let args = [
        "merge-file",
        "-p",
        "--quiet",
        "-L",
        "delta",
        "-L",
        "base",
        "-L",
        "main",
        &paths[0],
        &paths[1],
        &paths[2],
    ];
    match run(root, &args, Options::default()) {
        Ok(out) => Ok(Some(String::from_utf8_lossy(&out).into_owned())),
        // The exit code is the number of conflicts.
        Err(clm_git::Error::Failed { status, .. })
            if status.code().is_some_and(|c| (1..=127).contains(&c)) =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// The delta file `file` with `element`'s text replaced by `merged` and its recorded base
/// `old` set to `new`; nothing else changes, the blank lines around the old text included.
/// `None` when the element or its base isn't where `clm-apply` would find it.
fn rewrite(file: &str, element: &str, merged: &str, old: &str, new: &str) -> Option<String> {
    let lines: Vec<&str> = file.lines().collect();
    if lines.first()?.trim_end() != "---" {
        return None;
    }
    let fm_end = 1 + lines.iter().skip(1).position(|l| l.trim_end() == "---")?;
    let sections = section_spans(&lines);
    let merged: Vec<&str> = merged.lines().collect();
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let (range, replacement) = match element.strip_prefix("requirement:") {
        None => {
            let name = match element {
                "contract" => "MODIFIED Contract",
                _ => "MODIFIED Design",
            };
            let s = sections.iter().find(|s| s.name == name)?;
            let body = s.heading + 1..s.end;
            let blank = |i: &usize| lines[*i].trim().is_empty();
            let start = body.clone().find(|i| !blank(i))?;
            let end = body.clone().rev().find(|i| !blank(i))? + 1;
            // Only the frontmatter's `<key>: "<old>"`, in a flow map or a block map.
            let edited = out[1..fm_end]
                .iter_mut()
                .find_map(|l| set_base(l, element, old, new).map(|e| (l, e)))?;
            *edited.0 = edited.1;
            (start..end, merged.iter().map(|l| l.to_string()).collect())
        }
        Some(slug) => {
            let s = sections
                .iter()
                .find(|s| s.name == "MODIFIED Requirements")?;
            let r = requirement_spans(&lines, s.heading + 1..s.end)
                .into_iter()
                .find(|r| r.slug == slug)?;
            if r.base.as_deref() != Some(old) {
                return None;
            }
            let (heading, body) = merged.split_first()?;
            clm_model::outline::requirement_heading(heading)?;
            let end = r
                .lines
                .clone()
                .rev()
                .find(|i| !lines[*i].trim().is_empty())?
                + 1;
            let mut block = vec![lines[r.lines.start].replacen(old, new, 1)];
            block.extend(body.iter().map(|l| l.to_string()));
            (r.lines.start..end, block)
        }
    };
    out.splice(range, replacement);
    let mut text = out.join("\n");
    if file.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}
