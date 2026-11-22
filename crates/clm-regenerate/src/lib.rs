//! The regenerated `specs/` for `clm rebase` and `clm land`, its stale-delta merges and report
//! (see specs/sync/regenerate/SPEC.md).

use std::fs;
use std::path::{Path, PathBuf};

use clm_model::ARCHIVE_DIR;
use stages::Stages;

mod lock;
mod merge;
mod stages;
mod stale;

pub use lock::{git, run};

/// A refusal (`Refused`, exit 1) or a git failure (`Git`, exit 2): `clm-git`'s shared
/// `Failure` (`shared-plumbing-in-git`).
pub use clm_git::Failure as Error;

/// A regenerated `specs/` (`regenerated-tree`): the tree every change of `ids` was applied
/// to, in a temporary directory, with the stages it went through and the `merged` lines of
/// the deltas merged or taken back on the way.
pub struct Tree {
    /// Keeps the temporary directory alive.
    _dir: tempfile::TempDir,
    /// The tree's CLM root inside the directory, which holds the repository's prefix.
    root: PathBuf,
    stages: Stages,
    /// The `merged` lines, one per element merged, with the `took back` lines among them in
    /// the order they were made.
    pub merged: Vec<String>,
}

impl Tree {
    /// The CLM root of the regenerated tree: its `specs/`, `changes/` and `backlog/`.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// `stale-delta-merge`: once the caller has mirrored the tree into `root`'s `specs/` and
    /// committed it, takes back and merges the deltas of the other live `proposed` or
    /// `approved` changes (not `ids`, the changes the tree regenerated), in ID order, each
    /// change committed at most once, against `onto`. Returns the `merged` and `took back`
    /// lines and what is left: the violations and the stale-delta report of the first change
    /// still refused.
    pub fn other_deltas(
        &mut self,
        root: &Path,
        ids: &[String],
        onto: &str,
    ) -> Result<(Vec<String>, Vec<String>), Error> {
        merge::other_deltas(root, &self.root, &mut self.stages, ids, onto)
    }
}

/// What `regenerated` gives: the tree, or the lines that stopped it.
pub enum Regenerated {
    Tree(Tree),
    /// The first change with violations or that doesn't apply: the `merged` lines so far, and
    /// the violations followed by the stale-delta report.
    Stopped {
        merged: Vec<String>,
        left: Vec<String>,
    },
}

/// `regenerated-tree`: `onto`'s `specs/` (none when it has none) with `HEAD`'s `changes/`,
/// `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md`, each taken under
/// the CLM root's prefix in the repository, then the changes of `ids` re-applied one at a
/// time in the order their bases give (`next_change`), stopping at the first with violations
/// under its own directory or that doesn't apply. `clm rebase`'s regenerate step and `clm
/// land`'s derived-specs check both build their tree here. With `merge`, each change is
/// first taken back and merged as `stale-delta-merge` states, and its delta files committed
/// in `root` once it is reached; without it nothing in `root`'s working tree, index or
/// history changes. An `onto` that names no commit, and an ID `HEAD` holds no change for,
/// are refused before anything is written.
pub fn regenerated(
    root: &Path,
    onto: &str,
    ids: &[String],
    merge: bool,
) -> Result<Regenerated, Error> {
    let prefix = git(root, &["rev-parse", "--show-prefix"])?;
    let under = |dir: &str| format!("{prefix}{dir}");
    let commit = format!("{onto}^{{commit}}");
    match git(root, &["rev-parse", "--verify", "--quiet", &commit]) {
        Ok(_) => {}
        Err(clm_git::Error::Failed { .. }) => {
            return Err(Error::Git(format!("`{onto}` names no commit")));
        }
        Err(e) => return Err(e.into()),
    }
    // `reapply-order-follows-bases`: `ids` are a set, applied in the order the bases give.
    let mut left_to_apply: Vec<&String> = ids.iter().collect();
    left_to_apply.sort();
    left_to_apply.dedup();
    for &id in &left_to_apply {
        let live = under(&format!("changes/{id}/change.md"));
        let archived = under(&format!("changes/{ARCHIVE_DIR}/{id}/change.md"));
        if !exists_at(root, "HEAD", &live)? && !exists_at(root, "HEAD", &archived)? {
            return Err(Error::Refused(format!(
                "`HEAD` holds no change `{id}`, live or archived"
            )));
        }
    }
    if merge {
        merge::refuse_uncommitted(root, left_to_apply.iter().map(|id| id.as_str()))?;
    }
    let tmp = extract_present(root, onto, &[&under("specs")])?;
    // The rebased branch's changes, backlog, decisions and human layers, so links such as
    // `resolves` and `after` resolve as they do in the repository. From `HEAD`, not the
    // working tree, where an untracked backlog item may sit. Moved into the one tree
    // `clm_model::load` reads.
    let layers = [
        "changes",
        "backlog",
        "docs/decisions",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
    ]
    .map(under);
    let layers: Vec<&str> = layers.iter().map(String::as_str).collect();
    let branch = extract_present(root, "HEAD", &layers)?;
    let tree = tmp.path().join(&prefix);
    let from_branch = branch.path().join(&prefix);
    fs::create_dir_all(&tree).map_err(|e| io(&tree, e))?;
    if from_branch.exists() {
        for entry in fs::read_dir(&from_branch).map_err(|e| io(&from_branch, e))? {
            let from = entry.map_err(|e| io(&from_branch, e))?.path();
            let to = tree.join(from.file_name().unwrap_or_default());
            fs::rename(&from, &to).map_err(|e| io(&to, e))?;
        }
    }
    let mut merged_lines = Vec::new();
    // `stale-delta-names-movers`: the graph loaded before each change, kept as its stage.
    let mut stages = Stages::new();
    let mut previous: Option<String> = None;
    while !left_to_apply.is_empty() {
        let (graph, mut all) = clm_model::load(&tree);
        stages.push(previous.as_deref(), graph);
        // Ordered by the bases as committed, before any take-back.
        let id = next_change(root, &stages, &left_to_apply, onto)?;
        let mut edits = merge::Edits::new(id);
        if merge && let Some(change) = stages.last().changes.get(id).cloned() {
            // `stale-delta-merge`: a base only this branch's lost tree had, taken back.
            if merge::take_back(root, &tree, &stages, &change, onto, &mut edits)? {
                let (graph, reloaded) = clm_model::load(&tree);
                stages.reload(graph);
                all = reloaded;
            }
        }
        let mut merged = false;
        let stopped: Option<Vec<String>> = loop {
            let graph = stages.last();
            let violations = own_violations(graph, all.clone(), id);
            if !violations.is_empty() {
                break Some(violations.iter().map(clm_check::format).collect());
            }
            let Some(change) = graph.changes.get(id) else {
                break Some(vec![format!("change `{id}` doesn't load")]);
            };
            let v = match clm_apply::reapply(graph, change) {
                Ok(ops) => {
                    for op in ops {
                        op.perform(&tree).map_err(|e| io(op.path(), e))?;
                    }
                    break None;
                }
                Err(v) => v,
            };
            // `stale-delta-merge`: once per change refused for stale bases, what merges
            // cleanly is written into its delta and the change applied again.
            if merge && !merged && v.iter().any(|v| v.rule == "apply/stale-base") {
                merged = true;
                if merge::merge(root, &tree, &stages, change, onto, &mut edits)? {
                    let (graph, reloaded) = clm_model::load(&tree);
                    stages.reload(graph);
                    all = reloaded;
                    continue;
                }
            }
            // `stale-delta-diff`: what moved each element left stale, and who moved it.
            let mut left: Vec<String> = v.iter().map(clm_check::format).collect();
            left.extend(stale::report(root, &stages, change, onto)?);
            break Some(left);
        };
        // One commit for the change reached, kept when an element is left.
        merged_lines.extend(edits.finish(root, &tree)?);
        if let Some(left) = stopped {
            return Ok(Regenerated::Stopped {
                merged: merged_lines,
                left,
            });
        }
        left_to_apply.retain(|&other| other != id);
        previous = Some(id.clone());
    }
    // The regenerated tree is the last stage.
    let (graph, _) = clm_model::load(&tree);
    stages.push(previous.as_deref(), graph);
    Ok(Regenerated::Tree(Tree {
        root: tree,
        _dir: tmp,
        stages,
        merged: merged_lines,
    }))
}

/// `reapply-order-follows-bases`: of the changes left, in ID order, the first none of whose
/// recorded bases is unknown, or the first when every one has an unknown base. A change the
/// graph doesn't hold has none, so ID order reaches it and its violations stop the run.
fn next_change<'a>(
    root: &Path,
    stages: &Stages,
    left: &[&'a String],
    onto: &str,
) -> Result<&'a String, Error> {
    for &id in left {
        match stages.last().changes.get(id) {
            Some(change) if stages.has_unknown_base(root, onto, change)? => {}
            _ => return Ok(id),
        }
    }
    Ok(left[0])
}

/// The violations of loading `graph` that lie under change `id`'s directory, or all of them
/// when the graph has no such change. Only the change's own files can keep it from
/// applying: the rest of the mixed tree the regenerate loads, `main`'s `specs/` under the
/// branch's `backlog/`, may not hold yet, since a `blocks` naming a requirement a change
/// adds is valid only once it applies. `clm check` judges the result (`judged-by-check`).
fn own_violations(
    graph: &clm_model::Graph,
    all: Vec<clm_model::Violation>,
    id: &str,
) -> Vec<clm_model::Violation> {
    match graph.changes.get(id) {
        Some(change) => {
            let dir = change.path.parent().unwrap_or(Path::new(""));
            all.into_iter()
                .filter(|v| v.path.starts_with(dir))
                .collect()
        }
        None => all,
    }
}

/// `shared-helpers`: whether `path` exists in the tree of `rev`; false when git says it
/// doesn't, a `rev` that names no commit included, and an error only when git can't run.
pub fn exists_at(root: &Path, rev: &str, path: &str) -> Result<bool, Error> {
    match git(root, &["cat-file", "-e", &format!("{rev}:{path}")]) {
        Ok(_) => Ok(true),
        Err(clm_git::Error::Failed { .. }) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// `shared-helpers`: the ones of `paths` that `rev` has, written as a checkout would write
/// them into a temporary directory by `clm-git`'s `extract_paths`, which runs no hook and
/// leaves the working tree and the index alone. Empty when `rev` has none of them.
pub fn extract_present(root: &Path, rev: &str, paths: &[&str]) -> Result<tempfile::TempDir, Error> {
    let mut present = Vec::new();
    for &path in paths {
        if exists_at(root, rev, path)? {
            present.push(path);
        }
    }
    Ok(clm_git::extract_paths(root, rev, &present)?)
}

/// `shared-helpers`: an I/O failure at `path`, as a git failure (exit 2).
pub fn io(path: &Path, e: std::io::Error) -> Error {
    Error::Git(format!("{}: {e}", path.display()))
}
