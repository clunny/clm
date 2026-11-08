//! `regenerate-renames`: a branch that adds a rename is made one commit on `origin/main`
//! holding what `clm rename` produces there, never rebased.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use clm_git::Options;
use clm_regenerate::{exists_at, git, io, run};

use crate::{Error, names, succeeded, with_status};

/// What the rename step found.
pub(crate) enum Outcome {
    /// The branch adds no rename: the usual rebase applies.
    Other,
    /// Something is left for the caller: lines to print, exit 1.
    Left(Vec<String>),
    /// `HEAD` is now one commit on `origin/main` holding the rename.
    Done,
}

/// For a branch adding exactly one change whose `change.md` sets `renames` (`added`, as
/// `added_changes` lists them), asks `rename/replay` whether it replays against `base`, the
/// merge base with `onto`, and then rebuilds it on `onto`.
pub(crate) fn regenerate(
    root: &Path,
    onto: &str,
    base: &str,
    added: &[(String, bool)],
) -> Result<Outcome, Error> {
    let [(id, _)] = added else {
        return Ok(Outcome::Other);
    };
    let live = format!("changes/{id}/change.md");
    let path = if exists_at(root, "HEAD", &live)? {
        live
    } else {
        format!("changes/{}/{id}/change.md", clm_model::ARCHIVE_DIR)
    };
    let bytes = run(
        root,
        &["cat-file", "blob", &format!("HEAD:{path}")],
        Options::default(),
    )?;
    let source = String::from_utf8_lossy(&bytes).into_owned();
    if !sets_renames(&source) {
        return Ok(Outcome::Other);
    }
    if !clm_rename::replays(root, base, "HEAD")? {
        let differing = match replayed(root, base, id, &source) {
            Ok(tree) => differing(root, base, tree.path())?,
            Err(Error::Refused(_)) => Vec::new(),
            Err(e) => return Err(e),
        };
        let paths = if differing.is_empty() {
            vec![path]
        } else {
            differing
        };
        return Ok(Outcome::Left(
            paths
                .iter()
                .map(|p| format!("not a rename branch {p}"))
                .collect(),
        ));
    }
    let tree = match replayed(root, onto, id, &source) {
        Ok(tree) => tree,
        Err(Error::Refused(why)) => return Ok(Outcome::Left(vec![why])),
        Err(e) => return Err(e),
    };
    commit(root, onto, base, id, tree.path())
}

/// Whether `source`'s frontmatter records at least one rename.
fn sets_renames(source: &str) -> bool {
    let text = source.replace("\r\n", "\n");
    let Some((front, _)) = clm_model::split_frontmatter(&text) else {
        return false;
    };
    serde_yaml::from_str::<serde_yaml::Value>(front)
        .ok()
        .and_then(|v| v.get("renames").and_then(|r| r.as_sequence().map(Vec::len)))
        .is_some_and(|n| n > 0)
}

/// What `clm rename` produces from `rev`'s files with the change placed live, `proposed`.
fn replayed(root: &Path, rev: &str, id: &str, source: &str) -> Result<tempfile::TempDir, Error> {
    let tree = clm_git::extract_tree(root, rev)?;
    let change = tree.path().join("changes").join(id).join("change.md");
    if let Some(dir) = change.parent() {
        fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    }
    fs::write(&change, with_status(source, "proposed")).map_err(|e| io(&change, e))?;
    // No repository of its own and no git command here, as `rename/replay` builds its tree:
    // `rename` reads a directory that is no repository as it reads the replay's, and a
    // `GIT_DIR` the environment sets never reaches the clone.
    clm_rename::rename(tree.path(), id)?;
    Ok(tree)
}

/// The blob ID of each regular file under `dir`, by path relative to it.
fn tree_ids(root: &Path, dir: &Path) -> Result<BTreeMap<String, String>, Error> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        for entry in fs::read_dir(&d).map_err(|e| io(&d, e))?.flatten() {
            match entry.file_type() {
                Ok(t) if t.is_dir() && entry.file_name() == ".git" && d == dir => {}
                Ok(t) if t.is_dir() => pending.push(entry.path()),
                Ok(t) if t.is_file() => files.push(entry.path()),
                _ => {}
            }
        }
    }
    files.sort();
    let input: String = files.iter().map(|f| format!("{}\n", f.display())).collect();
    let out = run(
        root,
        &["hash-object", "--no-filters", "--stdin-paths"],
        Options {
            stdin: Some(input.as_bytes()),
            env: &[],
        },
    )?;
    let ids = String::from_utf8_lossy(&out).into_owned();
    Ok(files
        .iter()
        .zip(ids.lines())
        .filter_map(|(f, id)| {
            let rel = f.strip_prefix(dir).ok()?;
            Some((rel.display().to_string(), id.to_string()))
        })
        .collect())
}

/// The blob ID of each regular file in `rev`'s tree.
fn rev_ids(root: &Path, rev: &str) -> Result<BTreeMap<String, String>, Error> {
    let out = run(
        root,
        &["ls-tree", "-r", "-z", "--full-tree", rev],
        Options::default(),
    )?;
    Ok(String::from_utf8_lossy(&out)
        .split('\0')
        .filter_map(|entry| {
            let (meta, path) = entry.split_once('\t')?;
            let mut parts = meta.split(' ');
            let (mode, kind, id) = (parts.next()?, parts.next()?, parts.next()?);
            (kind == "blob" && mode.starts_with("100")).then(|| (path.to_string(), id.to_string()))
        })
        .collect())
}

/// The files `HEAD` adds under `backlog/` over `base`: a session files defect reports on
/// any branch, and the rename keeps them.
fn added_backlog(
    head: &BTreeMap<String, String>,
    base: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    head.keys()
        .filter(|p| p.starts_with("backlog/") && !base.contains_key(*p))
        .cloned()
        .collect()
}

/// The paths where `HEAD` differs from what `rename` produced in `replay` from `base`,
/// leaving out the backlog files `HEAD` adds, sorted.
fn differing(root: &Path, base: &str, replay: &Path) -> Result<Vec<String>, Error> {
    let head = rev_ids(root, "HEAD")?;
    let kept = added_backlog(&head, &rev_ids(root, base)?);
    let made = tree_ids(root, replay)?;
    let paths: BTreeSet<&String> = head.keys().chain(made.keys()).collect();
    Ok(paths
        .into_iter()
        .filter(|p| !kept.contains(*p) && head.get(*p) != made.get(*p))
        .cloned()
        .collect())
}

/// Makes `HEAD` one commit on `onto`, `Rename <id>`, holding `tree` (what `rename`
/// produced from `onto`) and the backlog files the branch added over `base`, writing only
/// the paths that change. An uncommitted edit to one of those paths stops it.
fn commit(root: &Path, onto: &str, base: &str, id: &str, tree: &Path) -> Result<Outcome, Error> {
    let head = rev_ids(root, "HEAD")?;
    let added = added_backlog(&head, &rev_ids(root, base)?);
    let mut made = tree_ids(root, tree)?;
    let mut extra = Vec::new();
    for path in &added {
        if !made.contains_key(path) {
            let bytes = run(
                root,
                &["cat-file", "blob", &format!("HEAD:{path}")],
                Options::default(),
            )?;
            let to = tree.join(path);
            if let Some(dir) = to.parent() {
                fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
            }
            fs::write(&to, bytes).map_err(|e| io(&to, e))?;
            extra.push(path.clone());
        }
    }
    if !extra.is_empty() {
        made = tree_ids(root, tree)?;
    }
    let before = rev_ids(root, onto)?;
    let all: BTreeSet<&String> = head
        .keys()
        .chain(made.keys())
        .chain(before.keys())
        .collect();
    // Where `HEAD` isn't the result yet, and where the result isn't `onto`'s.
    let off_head = |p: &&String| head.get(*p) != made.get(*p);
    let touched: BTreeSet<&String> = all
        .iter()
        .copied()
        .filter(|p| off_head(p) || before.get(*p) != made.get(*p))
        .collect();
    let parent = succeeded(root, &["rev-parse", "-q", "--verify", "HEAD^"])?;
    // Already one commit on `onto` holding the result: a second run adds nothing.
    if !all.iter().any(off_head) && parent.as_deref() == Some(onto) {
        return Ok(Outcome::Done);
    }
    let edited: BTreeSet<String> =
        names(root, &["diff", "--name-only", "-z", "--no-renames", "HEAD"])?
            .into_iter()
            .collect();
    let blocked: Vec<String> = touched
        .iter()
        .filter(|p| edited.contains(**p))
        .map(|p| format!("uncommitted {p}"))
        .collect();
    if !blocked.is_empty() {
        return Ok(Outcome::Left(blocked));
    }
    for path in &touched {
        let to = root.join(path);
        if made.contains_key(*path) {
            if let Some(dir) = to.parent() {
                fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
            }
            let from = tree.join(path);
            fs::copy(&from, &to).map_err(|e| io(&from, e))?;
        } else if to.exists() {
            fs::remove_file(&to).map_err(|e| io(&to, e))?;
            for dir in to.ancestors().skip(1) {
                if dir == root || fs::remove_dir(dir).is_err() {
                    break;
                }
            }
        }
    }
    git(root, &["reset", "-q", "--mixed", onto])?;
    let listing: String = touched.iter().map(|p| format!("{p}\0")).collect();
    run(
        root,
        &["update-index", "--add", "--remove", "-z", "--stdin"],
        Options {
            stdin: Some(listing.as_bytes()),
            env: &[],
        },
    )?;
    if !git(root, &["diff", "--cached", "--name-only"])?.is_empty() {
        git(root, &["commit", "-q", "-m", &format!("Rename {id}")])?;
    }
    Ok(Outcome::Done)
}
