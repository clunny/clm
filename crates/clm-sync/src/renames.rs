//! `regenerate-renames`: a branch that adds a rename is made one commit on `origin/main`
//! holding what `clm rename` produces there, never rebased. Each revision is read as the CLM
//! root's tree, as `rename/replay` reads one (`fingerprint-covers-the-clm-root`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use clm_git::Options;
use clm_regenerate::{exists_at, git, io, run};

use crate::root::{Root, object};
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
    place: &Root,
    onto: &str,
    base: &str,
    added: &[(String, bool)],
) -> Result<Outcome, Error> {
    let [(id, _)] = added else {
        return Ok(Outcome::Other);
    };
    let live = format!("changes/{id}/change.md");
    let path = if exists_at(root, "HEAD", &format!("./{live}"))? {
        live
    } else {
        format!("changes/{}/{id}/change.md", clm_model::ARCHIVE_DIR)
    };
    let bytes = run(
        root,
        &["cat-file", "blob", &object("HEAD", &path)],
        Options::default(),
    )?;
    let source = String::from_utf8_lossy(&bytes).into_owned();
    if !sets_renames(&source) {
        return Ok(Outcome::Other);
    }
    if !clm_rename::replays(root, base, "HEAD")? {
        let replay = match replayed(root, place, base, id, &source) {
            Ok(replay) => Some(replay),
            Err(Error::Refused(_)) => None,
            Err(e) => return Err(e),
        };
        let differing = differing(root, place, base, replay.as_ref().map(|r| r.dir.as_path()))?;
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
    let replay = match replayed(root, place, onto, id, &source) {
        Ok(replay) => replay,
        Err(Error::Refused(why)) => return Ok(Outcome::Left(vec![why])),
        Err(e) => return Err(e),
    };
    commit(root, place, onto, base, id, &replay.dir)
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

/// A revision's tree extracted, with `rename` carried out in the CLM root's directory of it.
struct Replay {
    _tree: tempfile::TempDir,
    /// The CLM root's directory in the extracted tree.
    dir: PathBuf,
}

/// What `clm rename` produces from `rev`'s files with the change placed live, `proposed`,
/// run in the root's directory of the extracted tree.
fn replayed(root: &Path, place: &Root, rev: &str, id: &str, source: &str) -> Result<Replay, Error> {
    let tree = clm_git::extract_tree(root, rev)?;
    let dir = tree.path().join(place.prefix());
    let change = dir.join("changes").join(id).join("change.md");
    if let Some(parent) = change.parent() {
        fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
    }
    fs::write(&change, with_status(source, "proposed")).map_err(|e| io(&change, e))?;
    // No repository of its own and no git command here, as `rename/replay` builds its tree:
    // `rename` reads a directory that is no repository as it reads the replay's, and a
    // `GIT_DIR` the environment sets never reaches the clone.
    clm_rename::rename(&dir, id)?;
    Ok(Replay { _tree: tree, dir })
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

/// A revision's entries, split at the CLM root.
struct Listing {
    /// The regular files under the root, by root-relative path: their blob IDs.
    inside: BTreeMap<String, String>,
    /// Every entry outside the root, by its path from the repository's top: its mode and
    /// object ID.
    outside: BTreeMap<String, (String, String)>,
}

/// `rev`'s tree, read as the CLM root's with what lies outside it apart.
fn listing(root: &Path, place: &Root, rev: &str) -> Result<Listing, Error> {
    let out = run(
        root,
        &["ls-tree", "-r", "-z", "--full-tree", rev],
        Options::default(),
    )?;
    let mut listing = Listing {
        inside: BTreeMap::new(),
        outside: BTreeMap::new(),
    };
    for entry in String::from_utf8_lossy(&out).split('\0') {
        let Some((meta, path)) = entry.split_once('\t') else {
            continue;
        };
        let mut parts = meta.split(' ');
        let (Some(mode), Some(kind), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        match path.strip_prefix(place.prefix()) {
            Some(rel) => {
                if kind == "blob" && mode.starts_with("100") {
                    listing.inside.insert(rel.to_string(), id.to_string());
                }
            }
            None => {
                listing
                    .outside
                    .insert(path.to_string(), (mode.to_string(), id.to_string()));
            }
        }
    }
    Ok(listing)
}

/// The paths outside the root where `a` and `b` differ, by path, mode or object ID, each
/// from the repository's top.
fn outside_differing(a: &Listing, b: &Listing) -> BTreeSet<String> {
    a.outside
        .keys()
        .chain(b.outside.keys())
        .filter(|p| a.outside.get(*p) != b.outside.get(*p))
        .cloned()
        .collect()
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

/// The paths where `HEAD` differs from the replay of `base`, named relative to the root,
/// sorted: under the root, where it differs from what `rename` produced in `replay` (none
/// when `rename` refused), leaving out the backlog files `HEAD` adds; outside it, where it
/// differs from `base`, which the replay holds there.
fn differing(
    root: &Path,
    place: &Root,
    base: &str,
    replay: Option<&Path>,
) -> Result<Vec<String>, Error> {
    let head = listing(root, place, "HEAD")?;
    let before = listing(root, place, base)?;
    let mut paths: BTreeSet<String> = outside_differing(&head, &before)
        .iter()
        .map(|p| place.name(p))
        .collect();
    if let Some(replay) = replay {
        let kept = added_backlog(&head.inside, &before.inside);
        let made = tree_ids(root, replay)?;
        paths.extend(
            head.inside
                .keys()
                .chain(made.keys())
                .filter(|p| !kept.contains(*p) && head.inside.get(*p) != made.get(*p))
                .cloned(),
        );
    }
    Ok(paths.into_iter().collect())
}

/// Makes `HEAD` one commit on `onto`, `Rename <id>`, holding under the root `dir` (what
/// `rename` produced from `onto` in the root's directory) and the backlog files the branch
/// added over `base`, and outside it `onto`'s entries, writing only the paths that change.
/// An uncommitted edit to one of those paths stops it.
fn commit(
    root: &Path,
    place: &Root,
    onto: &str,
    base: &str,
    id: &str,
    dir: &Path,
) -> Result<Outcome, Error> {
    let head = listing(root, place, "HEAD")?;
    let target = listing(root, place, onto)?;
    let added = added_backlog(&head.inside, &listing(root, place, base)?.inside);
    let mut made = tree_ids(root, dir)?;
    let mut extra = Vec::new();
    for path in &added {
        if !made.contains_key(path) {
            let bytes = run(
                root,
                &["cat-file", "blob", &object("HEAD", path)],
                Options::default(),
            )?;
            let to = dir.join(path);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
            }
            fs::write(&to, bytes).map_err(|e| io(&to, e))?;
            extra.push(path.clone());
        }
    }
    if !extra.is_empty() {
        made = tree_ids(root, dir)?;
    }
    let all: BTreeSet<&String> = head
        .inside
        .keys()
        .chain(made.keys())
        .chain(target.inside.keys())
        .collect();
    // Under the root: where `HEAD` isn't the result yet, and where the result isn't `onto`'s.
    let off_head = |p: &&String| head.inside.get(*p) != made.get(*p);
    let touched: BTreeSet<&String> = all
        .iter()
        .copied()
        .filter(|p| off_head(p) || target.inside.get(*p) != made.get(*p))
        .collect();
    // Outside it the commit holds `onto`'s entries: where they aren't `HEAD`'s, named
    // relative to the root.
    let moved = outside_differing(&head, &target);
    let outside: Vec<String> = moved.iter().map(|p| place.name(p)).collect();
    let parent = succeeded(root, &["rev-parse", "-q", "--verify", "HEAD^"])?;
    // Already one commit on `onto` holding the result: a second run adds nothing.
    if !all.iter().any(off_head) && outside.is_empty() && parent.as_deref() == Some(onto) {
        return Ok(Outcome::Done);
    }
    // `--no-relative`, so a `diff.relative` setting never hides a path from the root.
    let edited: BTreeSet<String> = names(
        root,
        &[
            "diff",
            "--name-only",
            "-z",
            "--no-renames",
            "--no-relative",
            "HEAD",
        ],
    )?
    .iter()
    .map(|p| place.name(p))
    .collect();
    // A path the commit writes that `HEAD` doesn't track but the working tree holds is an
    // untracked file it would write over.
    let untracked =
        |name: &str, tracked: bool| !tracked && fs::symlink_metadata(root.join(name)).is_ok();
    let blocked: Vec<String> = touched
        .iter()
        .map(|p| (p.as_str(), untracked(p, head.inside.contains_key(*p))))
        .chain(moved.iter().zip(&outside).map(|(top, name)| {
            (
                name.as_str(),
                untracked(name, head.outside.contains_key(top)),
            )
        }))
        .filter(|(p, over)| *over || edited.contains(*p))
        .map(|(p, _)| format!("uncommitted {p}"))
        .collect();
    if !blocked.is_empty() {
        return Ok(Outcome::Left(blocked));
    }
    let tip = git(root, &["rev-parse", "HEAD"])?;
    let written = write(root, place, onto, id, dir, &made, &touched, &target, &moved);
    if written.is_err() {
        // Leave the branch at its tip, the files written so far as uncommitted edits, rather
        // than on `onto` with nothing committed.
        let _ = git(root, &["reset", "-q", "--mixed", &tip]);
    }
    written
}

/// Writes the regenerated tree into the working tree, under the root (`touched`, from
/// `dir`) and outside it (`moved`, as `onto` holds them), and commits it on `onto`.
#[allow(clippy::too_many_arguments)]
fn write(
    root: &Path,
    place: &Root,
    onto: &str,
    id: &str,
    dir: &Path,
    made: &BTreeMap<String, String>,
    touched: &BTreeSet<&String>,
    target: &Listing,
    moved: &BTreeSet<String>,
) -> Result<Outcome, Error> {
    for path in touched {
        let to = root.join(path);
        if made.contains_key(*path) {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
            }
            let from = dir.join(path);
            fs::copy(&from, &to).map_err(|e| io(&from, e))?;
        } else if to.exists() {
            remove(root, &to)?;
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
    // The index now holds `onto`'s entries outside the root: write each one that isn't
    // `HEAD`'s into the working tree, and remove those `onto` lacks. A submodule's
    // directory is left, as `git checkout` leaves one.
    let (present, gone): (Vec<&String>, Vec<&String>) =
        moved.iter().partition(|p| target.outside.contains_key(*p));
    if !present.is_empty() {
        let listing: String = present
            .iter()
            .map(|p| format!("{}\0", place.name(p)))
            .collect();
        run(
            root,
            &["checkout-index", "-f", "-z", "--stdin"],
            Options {
                stdin: Some(listing.as_bytes()),
                env: &[],
            },
        )?;
    }
    for path in gone {
        let to = root.join(place.name(path));
        if fs::symlink_metadata(&to).is_ok_and(|m| !m.is_dir()) {
            remove(root, &to)?;
        }
    }
    if !git(root, &["diff", "--cached", "--name-only"])?.is_empty() {
        git(root, &["commit", "-q", "-m", &format!("Rename {id}")])?;
    }
    Ok(Outcome::Done)
}

/// Removes the file `to` and each directory above it it leaves empty, short of `root`.
fn remove(root: &Path, to: &Path) -> Result<(), Error> {
    fs::remove_file(to).map_err(|e| io(to, e))?;
    for dir in to.ancestors().skip(1) {
        if dir == root || fs::remove_dir(dir).is_err() {
            break;
        }
    }
    Ok(())
}
