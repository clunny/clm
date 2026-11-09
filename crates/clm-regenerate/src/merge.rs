//! `stale-delta-merge` (`stale-delta-three-way-merge`): a stale delta's sections whose edits
//! don't overlap what moved them since their base, `main`'s edits or an earlier change's,
//! merged three-way into the delta with their new base, so the caller resolves only what is
//! left. `Tree::other_deltas` runs the same merge for another live change's delta (`stale-delta-merge`).

use std::fs;
use std::path::{Path, PathBuf};

use clm_git::Options;
use clm_model::outline::{requirement_spans, section_spans};
use clm_model::{Change, Delta};

use crate::stages::{Movers, Stages};
use crate::stale::text;
use crate::{Error, git, io, run};

/// What the merge did: the delta files it rewrote, repository-relative, a `merged <element>
/// of <node> into <delta path>` line for each element it merged, and what moved those
/// elements, together.
#[derive(Default)]
pub(crate) struct Merged {
    pub files: Vec<PathBuf>,
    pub lines: Vec<String>,
    pub movers: Movers,
}

impl Merged {
    /// Stages the delta files it rewrote, and nothing else, and commits them as `Merge
    /// <movers> into <change>`. Does nothing when it rewrote none.
    pub(crate) fn commit(&self, root: &Path, change: &str) -> Result<(), Error> {
        if self.files.is_empty() {
            return Ok(());
        }
        let files: Vec<String> = self
            .files
            .iter()
            .map(|f| f.to_string_lossy().into_owned())
            .collect();
        let mut add = vec!["add", "--"];
        add.extend(files.iter().map(String::as_str));
        git(root, &add)?;
        let message = format!("Merge {} into {change}", self.movers.join());
        let mut commit = vec!["commit", "-q", "-m", &message, "--"];
        commit.extend(files.iter().map(String::as_str));
        git(root, &commit)?;
        Ok(())
    }
}

/// For each delta of `change` in node-ID order, and each Contract, Design or requirement it
/// MODIFIES whose recorded base `apply` judges stale against the tree it is applied to (the
/// last of `stages`, loaded from `tree`; `stale-elements`), whose element is still there and whose base text a
/// stage or `onto`'s history has, in `clm hashes` order: the delta's text merged three-way
/// with the base's and the tree's. Each clean merge is written into the delta, in `tree` and
/// in the working tree under `root`, with the element's base set to its hash in the tree.
pub(crate) fn merge(
    root: &Path,
    tree: &Path,
    stages: &Stages,
    change: &Change,
    onto: &str,
) -> Result<Merged, Error> {
    let graph = stages.last();
    let mut deltas: Vec<&Delta> = change.deltas.iter().filter(|d| !d.remove).collect();
    deltas.sort_by(|a, b| a.node.cmp(&b.node));
    let mut merged = Merged::default();
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
                merged.movers.extend(&stages.movers(id, &element, base));
                merged.lines.push(format!(
                    "merged {element} of {id} into {}",
                    delta.path.display()
                ));
            }
        }
        if file != original {
            for path in [in_tree, root.join(&delta.path)] {
                fs::write(&path, &file).map_err(|e| io(&path, e))?;
            }
            merged.files.push(delta.path.clone());
        }
    }
    Ok(merged)
}

/// `stale-delta-merge`: once the regenerate applied `regenerated` and committed `Re-apply`,
/// with the regenerated tree, loaded from `tree`, as the last of `stages`. For each live
/// `proposed` or `approved` change it didn't regenerate, in ID order, that `apply` refuses on
/// that tree: its stale deltas merged as `merge` merges them and committed on their own as
/// `Merge <movers> into <change>`. The first change still refused stops it, with the
/// change's violations and the stale-delta report. Returns the `merged` lines and what is
/// left.
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
        let graph = stages.last();
        let mut v = refusal(graph, all.clone(), &id);
        if v.is_empty() {
            continue;
        }
        if v.iter().any(|v| v.rule == "apply/stale-base") {
            let m = merge(root, tree, stages, &graph.changes[&id], onto)?;
            lines.extend(m.lines.iter().cloned());
            if !m.files.is_empty() {
                m.commit(root, &id)?;
                let (graph, reloaded) = clm_model::load(tree);
                stages.reload(graph);
                all = reloaded;
                v = refusal(stages.last(), all.clone(), &id);
            }
        }
        if !v.is_empty() {
            let mut left: Vec<String> = v.iter().map(clm_check::format).collect();
            let change = &stages.last().changes[&id];
            left.extend(crate::stale::report(root, stages, change, onto)?);
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
