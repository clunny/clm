//! `clm view <name> --at <rev>` (`view-at-rev`): the graph, markers and size report of a
//! commit's tree, read from git's objects in memory, writing no file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clm_check::{Length, Marker, Size};
use clm_model::{Files, Graph};

use clm_view::At;

use crate::{BudgetsFrom, Error};

/// Bytes git reads from the start of a blob when deciding it is binary.
const BINARY_PROBE: usize = 8000;

/// What `--at` reads from a commit.
pub(crate) struct Commit {
    pub graph: Graph,
    pub markers: Vec<Marker>,
    /// The nodes over their budgets, when `budgets` was given and could be read.
    pub oversized: Option<BTreeSet<String>>,
    pub head: At,
}

/// A commit's tree: the files under the CLM root, relative to it, and every
/// `.gitattributes` of the whole tree with its directory relative to the repository's top.
struct Tree {
    files: Files,
    attributes: Vec<(String, String)>,
    /// The CLM root's path below the repository's top, `/`-terminated, or empty.
    prefix: String,
}

/// A blob's text, or `None` when it is binary (a NUL in its first 8000 bytes, as git
/// judges it) or not UTF-8, which the working tree's scan skips too.
fn text(bytes: &[u8]) -> Option<&str> {
    let probe = &bytes[..bytes.len().min(BINARY_PROBE)];
    if probe.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

/// The commit `rev` names, read from the local objects without fetching: its graph and
/// markers, and with `budgets`, its size report by its own `clm.toml`.
pub(crate) fn read(root: &Path, rev: &str, budgets: Option<BudgetsFrom>) -> Result<Commit, Error> {
    let failed = |e: String| Error::At(format!("`{rev}`: {e}"));
    let hash = clm_git::git(
        root,
        &[
            "rev-parse",
            "--verify",
            "-q",
            "--end-of-options",
            &format!("{rev}^{{commit}}"),
        ],
    )
    .map_err(|e| match e {
        // `--verify -q` exits 1, saying nothing, for a revision that names no commit.
        clm_git::Error::Failed { status, .. } if status.code() == Some(1) => {
            Error::At(format!("`{rev}` names no commit"))
        }
        e => failed(e.to_string()),
    })?;
    let tree = tree(root, &hash).map_err(failed)?;
    let files = &tree.files;
    let (graph, violations) = clm_model::load_files(files);
    if !violations.is_empty() {
        return Err(Error::Violations(violations));
    }
    let markers: Vec<Marker> = files
        .iter()
        .filter_map(|(path, bytes)| Some(clm_check::markers_in(path, text(bytes)?)))
        .flatten()
        .collect();
    let oversized = budgets.and_then(|budgets| {
        let toml = files
            .get(Path::new("clm.toml"))
            .map(|b| String::from_utf8_lossy(b));
        let defaults = budgets(toml.as_deref())
            .inspect_err(|e| eprintln!("size not checked: {e}"))
            .ok()?;
        let mut lengths: BTreeMap<PathBuf, Length> = files
            .iter()
            .filter(|(path, bytes)| {
                let full = format!("{}{}", tree.prefix, path.to_string_lossy());
                text(bytes).is_some() && !clm_check::linguist_excluded(&tree.attributes, &full)
            })
            .map(|(path, bytes)| (path.clone(), Length::of(path, bytes)))
            .collect();
        // `clm size` counts each node's `SPEC.md` whatever its attributes say.
        for node in graph.nodes.values() {
            if let Some(bytes) = files.get(&node.path) {
                lengths.insert(node.path.clone(), (bytes.len() as u64).into());
            }
        }
        let sizes = clm_check::size_of(&graph, &lengths, &markers, defaults);
        Some(
            sizes
                .into_iter()
                .filter(Size::oversized)
                .map(|s| s.node)
                .collect(),
        )
    });
    let head = At {
        rev: rev.to_string(),
        commit: hash.get(..7).unwrap_or(&hash).to_string(),
        fetched: fetched(root),
        branch: clm_git::current_branch(root).ok().flatten(),
        now: clm_git::now(),
    };
    Ok(Commit {
        graph,
        markers,
        oversized,
        head,
    })
}

/// `commit`'s tree through `clm_git::tree_blobs`: the files under the CLM root, relative to
/// it, and every `.gitattributes` of the whole tree. A blob missing from the local objects is
/// an error; the layer tells git not to fetch it.
fn tree(root: &Path, commit: &str) -> Result<Tree, String> {
    let prefix = clm_git::git(root, &["rev-parse", "--show-prefix"]).map_err(|e| e.to_string())?;
    let keep = |path: &str| {
        path.starts_with(&prefix) || path == ".gitattributes" || path.ends_with("/.gitattributes")
    };
    let blobs = clm_git::tree_blobs(root, commit, &keep).map_err(|e| e.to_string())?;
    let mut out = Tree {
        files: Files::new(),
        attributes: Vec::new(),
        prefix: prefix.clone(),
    };
    for (path, content) in blobs {
        if let Some(dir) = path.strip_suffix(".gitattributes")
            && (dir.is_empty() || dir.ends_with('/'))
            && let Some(text) = text(&content)
        {
            out.attributes
                .push((dir.trim_end_matches('/').to_string(), text.to_string()));
        }
        if let Some(rel) = path.strip_prefix(&prefix) {
            out.files.insert(PathBuf::from(rel), content);
        }
    }
    Ok(out)
}

/// When the clone was last fetched, in Unix seconds: the newest modification time of the
/// main checkout's `FETCH_HEAD` and every linked worktree's, since a fetch in any of them
/// moves the shared `origin/*`; `None` when there is none.
fn fetched(root: &Path) -> Option<u64> {
    let common = clm_git::common_dir(root).ok()?;
    let linked = std::fs::read_dir(common.join("worktrees"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("FETCH_HEAD"));
    std::iter::once(common.join("FETCH_HEAD"))
        .chain(linked)
        .filter_map(|path| {
            let modified = std::fs::metadata(path).ok()?.modified().ok()?;
            Some(
                modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs(),
            )
        })
        .max()
}
