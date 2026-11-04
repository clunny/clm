//! `stale-delta-diff` (`stale-delta-shows-main`, `stale-delta-names-movers`): what moved each
//! stale element of a delta since its recorded base, and who moved it, so the author merges
//! that text into the delta rather than updating only its hashes.

use std::fs;
use std::path::{Path, PathBuf};

use clm_git::Options;
use clm_model::{Change, Delta, Graph, Node};

use crate::stages::Stages;
use crate::{Error, exists_at, git, io, run};

/// For each delta of `change`, in node-ID order, and each Contract, Design or requirement it
/// MODIFIES or REMOVES whose recorded base `apply` judges stale against the tree the change
/// was applied to (the last of `stages`; `stale-elements`), in `clm hashes` order: a header naming the movers, the
/// hunks from the base's text to the text in the tree, and what to do; or one `no text` line
/// when neither a stage nor `onto`'s history has the base.
pub(crate) fn report(
    root: &Path,
    stages: &Stages,
    change: &Change,
    onto: &str,
) -> Result<Vec<String>, Error> {
    let graph = stages.last();
    let mut deltas: Vec<&Delta> = change.deltas.iter().collect();
    deltas.sort_by(|a, b| a.node.cmp(&b.node));
    let mut out = Vec::new();
    for delta in deltas {
        let id = delta.node.as_str();
        let node = graph.nodes.get(id);
        let current = node.map(clm_apply::hashes).unwrap_or_default();
        let position = |element: &str| current.iter().position(|(e, _)| e == element);
        // `stale-elements`: the elements `apply` judges stale, not a comparison of hashes.
        let stale = clm_apply::stale_bases(node, delta);
        let delta_path = delta.path.display();
        for (element, base) in &stale {
            let movers = stages.movers(id, element, base).join();
            let Some(old) = stages.base_text(root, onto, id, element, base)? else {
                out.push(format!(
                    "no text for {element} of {id} at {base}; compare {delta_path} with \
                     {element} of {id} after {movers} by hand"
                ));
                continue;
            };
            let new = node.and_then(|n| text(n, element)).unwrap_or_default();
            out.push(format!("{movers} changed {element} of {id} since {base}"));
            out.extend(diff(root, &old, &new)?);
            match position(element).filter(|_| !new.trim().is_empty()) {
                Some(i) => out.push(format!(
                    "merge the changes above into {delta_path}, then set its base to {}",
                    current[i].1
                )),
                None => out.push(format!(
                    "rework {delta_path} by hand: {id} no longer has {element}"
                )),
            }
        }
    }
    Ok(out)
}

/// `element`'s text in `node`, as `clm hashes` names it.
pub(crate) fn text(node: &Node, element: &str) -> Option<String> {
    match element {
        "contract" => Some(node.contract.clone()),
        "design" => Some(node.design.clone()),
        _ => {
            let slug = element.strip_prefix("requirement:")?;
            let id = format!("{}/{slug}", node.id);
            node.requirements
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.text.clone())
        }
    }
}

/// `onto`'s history of one `SPEC.md`, newest first, each commit's file parsed on first use.
pub(crate) struct History {
    root: PathBuf,
    path: String,
    commits: Vec<String>,
    /// The live changes' `change.md` files.
    changes: clm_model::Files,
    /// The node at each of the first commits, `None` where its file doesn't parse.
    parsed: Vec<Option<Node>>,
}

impl History {
    pub(crate) fn new(root: &Path, graph: &Graph, onto: &str, path: &Path) -> Result<Self, Error> {
        let path = path.to_string_lossy().into_owned();
        let log = git(root, &["log", "--format=%H", onto, "--", &path])?;
        let changes = graph
            .changes
            .values()
            .filter(|c| !c.archived)
            .map(|c| (c.path.clone(), c.source.clone().into_bytes()))
            .collect();
        Ok(History {
            root: root.to_path_buf(),
            commits: log.lines().map(str::to_string).collect(),
            path,
            changes,
            parsed: Vec::new(),
        })
    }

    /// `element`'s text at the newest commit where it hashes to `base`.
    pub(crate) fn text_at(
        &mut self,
        node: &str,
        element: &str,
        base: &str,
    ) -> Result<Option<String>, Error> {
        for i in 0..self.commits.len() {
            if i == self.parsed.len() {
                let parsed = self.parse(&self.commits[i], node)?;
                self.parsed.push(parsed);
            }
            if let Some(n) = &self.parsed[i]
                && clm_apply::hashes(n)
                    .iter()
                    .any(|(e, h)| e == element && h == base)
            {
                return Ok(text(n, element));
            }
        }
        Ok(None)
    }

    /// The node in the commit's file, parsed with only the live changes beside it, so the
    /// frontmatter keys they declare are accepted (`declared-frontmatter-keys`).
    fn parse(&self, commit: &str, node: &str) -> Result<Option<Node>, Error> {
        let Some(bytes) = show(&self.root, commit, &self.path)? else {
            return Ok(None);
        };
        let mut files = self.changes.clone();
        files.insert(PathBuf::from(&self.path), bytes);
        Ok(clm_model::load_files(&files).0.nodes.remove(node))
    }
}

/// `path` at `commit`, `None` when the commit deleted it.
fn show(root: &Path, commit: &str, path: &str) -> Result<Option<Vec<u8>>, Error> {
    if !exists_at(root, commit, path)? {
        return Ok(None);
    }
    let spec = format!("{commit}:{path}");
    Ok(Some(run(
        root,
        &["cat-file", "blob", &spec],
        Options::default(),
    )?))
}

/// The hunks, from the first `@@` on, of a `-U3` unified diff from `old` to `new`, each
/// ending in one newline (or empty), so git prints no `\ No newline` line.
fn diff(root: &Path, old: &str, new: &str) -> Result<Vec<String>, Error> {
    let tmp = tempfile::TempDir::new().map_err(|e| Error::Git(format!("temp dir: {e}")))?;
    let (a, b, out) = (
        tmp.path().join("base"),
        tmp.path().join("main"),
        tmp.path().join("diff"),
    );
    for (path, text) in [(&a, old), (&b, new)] {
        let text = text.trim_end_matches('\n');
        let text = if text.is_empty() {
            String::new()
        } else {
            format!("{text}\n")
        };
        fs::write(path, text).map_err(|e| io(path, e))?;
    }
    let output = format!("--output={}", out.display());
    let args = [
        "diff",
        "--no-index",
        "--no-color",
        "--no-ext-diff",
        "-U3",
        &output,
        "--",
        &a.to_string_lossy(),
        &b.to_string_lossy(),
    ];
    // `--no-index` exits 1 when the files differ.
    match run(root, &args, Options::default()) {
        Ok(_) => {}
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => {}
        Err(e) => return Err(e.into()),
    }
    let text = fs::read_to_string(&out).map_err(|e| io(&out, e))?;
    Ok(text
        .lines()
        .skip_while(|l| !l.starts_with("@@"))
        .map(str::to_string)
        .collect())
}
