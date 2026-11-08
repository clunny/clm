//! `land/derived-specs`: on a head that adds or applies a change, the merge commit's `specs/`
//! must be what regenerating gives, the base's `specs/` with the changes' deltas and assets
//! applied by `sync.regenerate`'s `regenerated`, the regenerate `clm rebase` runs, since the
//! fingerprint skips `specs/` there (`derived-specs-fingerprint`). It calls `clm-regenerate`
//! directly, never through `clm-sync`, which reaches `clm-health`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Repo};

/// Refuses the land, with `land/derived-specs`, when `head` adds or applies a change and the
/// merge commit's `specs/`, checked out at `merged` (the CLM root in the tree on the merge
/// commit), differs from the regenerated one, or the regenerate stops. A head that adds and
/// applies no change passes.
pub(crate) fn check(repo: &Repo, base: &str, head: &str, merged: &Path) -> Result<(), Error> {
    let Some(ids) = regenerated_ids(repo, base, head, merged)? else {
        return Ok(());
    };
    let tree = regenerate(base, &ids, merged)?;
    let scratch = tree.path();

    let regenerated = files(&scratch.join("specs"))?;
    let landing = files(&merged.join("specs"))?;
    let mut paths: Vec<&PathBuf> = regenerated.union(&landing).collect();
    paths.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
    for rel in paths {
        if entry(&scratch.join("specs").join(rel)) != entry(&merged.join("specs").join(rel)) {
            return Err(refuse(format!(
                "`specs/{}` isn't what regenerating `specs/` from the base and the \
                 branch's changes gives",
                rel.display()
            )));
        }
    }
    Ok(())
}

/// The refusal `land/derived-specs` makes: the rule, why, and `clm rebase`.
fn refuse(why: String) -> Error {
    Error::Refused(format!(
        "land/derived-specs: {why}: run `clm rebase` on the branch, push it, and land it again"
    ))
}

/// The IDs of the changes whose deltas the regenerate re-applies for `head` on `base`: the
/// ones `head` adds with status `applied` (`done`, archived) in the tree at `merged` (the CLM
/// root in a checkout), and the ones it applies, sorted; none when it adds or applies no
/// change at all.
fn regenerated_ids(
    repo: &Repo,
    base: &str,
    head: &str,
    merged: &Path,
) -> Result<Option<Vec<String>>, Error> {
    let merge_base = repo.git(&["merge-base", base, head])?;
    let added = added(repo, &merge_base, head)?;
    let read = |rel: String| fs::read(merged.join(rel)).ok();
    let applied = applied_changes(repo, base, &read)?;
    if added.is_empty() && applied.is_empty() {
        return Ok(None);
    }
    let mut ids = applied;
    for path in &added {
        let path = String::from_utf8_lossy(path);
        let rel = path
            .strip_prefix("changes/")
            .and_then(|p| p.strip_suffix("/change.md"))
            .unwrap_or_default();
        let (id, status) = match rel.strip_prefix("archive/") {
            Some(id) => (id, clm_model::ChangeStatus::Done),
            None => (rel, clm_model::ChangeStatus::Applied),
        };
        // Text that isn't UTF-8 doesn't parse, as for `clm rebase`; a read that fails is git's.
        let text = String::from_utf8_lossy(&fs::read(merged.join(&*path))?).into_owned();
        if has_status(&text, status) {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    ids.dedup();
    Ok(Some(ids))
}

/// The `change.md` paths `head` adds over `merge_base`.
fn added(repo: &Repo, merge_base: &str, head: &str) -> Result<Vec<Vec<u8>>, Error> {
    let at_base = repo.approval.listing(merge_base)?;
    let at_head = repo.approval.listing(head)?;
    Ok(clm_gate::added_changes(&at_base, &at_head))
}

/// `sync.regenerate`'s regenerate, without its merge, from the CLM root in the checkout at
/// `merged`: `onto`'s `specs/` with that commit's changes and backlog and `ids` re-applied.
/// A refusal of its own is a stop too, so it names the rule and `clm rebase`.
fn regenerate(onto: &str, ids: &[String], merged: &Path) -> Result<clm_regenerate::Tree, Error> {
    let regenerated = match clm_regenerate::regenerated(merged, onto, ids, false) {
        Err(clm_regenerate::Error::Refused(why)) => return Err(refuse(why)),
        other => other?,
    };
    match regenerated {
        clm_regenerate::Regenerated::Tree(tree) => Ok(tree),
        clm_regenerate::Regenerated::Stopped { left, .. } => Err(refuse(format!(
            "the branch's changes don't regenerate from the base:\n  {}\n",
            left.join("\n  ")
        ))),
    }
}

/// Whether `head` adds or applies a change over `merge_base`, as `check` reads both: adds a
/// `change.md`, or has status `applied` (or `done`, archived) a change that is `proposed` or
/// `approved` at `merge_base`.
pub(crate) fn adds_or_applies(repo: &Repo, merge_base: &str, head: &str) -> Result<bool, Error> {
    if !added(repo, merge_base, head)?.is_empty() {
        return Ok(true);
    }
    let read = |rel: String| {
        let out = clm_git::run(
            &repo.root,
            "git",
            &["show", &format!("{head}:./{rel}")],
            Default::default(),
        );
        out.ok()
    };
    Ok(!applied_changes(repo, merge_base, &read)?.is_empty())
}

/// For a head behind in `specs/` (`land/regenerates-behind-specs`), whose merge `git
/// merge-tree` wrote as `conflicted` (conflicts only in `specs/`, if any): proves the head's
/// own `specs/` derived, with `dir` the checkout at `head`, regenerating onto the merge base
/// as `check` does; then regenerates `specs/` onto `base` from a checkout of `conflicted` in
/// `dir`, and returns the tree of `conflicted` with `specs/` replaced by the result. `dir` is
/// left at the conflicted commit, which the caller's next checkout replaces.
pub(crate) fn behind_tree(
    repo: &Repo,
    base: &str,
    head: &str,
    conflicted: &str,
    dir: &crate::scratch::Scratch,
) -> Result<String, Error> {
    let merge_base = repo.git(&["merge-base", base, head])?;
    check(repo, &merge_base, head, dir.path())?;
    let commit = repo.git(&[
        "commit-tree",
        conflicted,
        "-p",
        base,
        "-p",
        head,
        "-m",
        "specs conflicts, regenerated next",
    ])?;
    dir.checkout(&commit)?;
    let ids = regenerated_ids(repo, base, head, dir.path())?.unwrap_or_default();
    let tree = regenerate(base, &ids, dir.path())?;
    let specs = dir.path().join("specs");
    if specs.exists() {
        fs::remove_dir_all(&specs)?;
    }
    copy_dir(&tree.path().join("specs"), &specs)?;
    clm_git::git(dir.path(), &["add", "-A", "--", "specs"])?;
    Ok(clm_git::git(dir.path(), &["write-tree"])?)
}

/// `from` copied to `to`, symlinks kept as links.
fn copy_dir(from: &Path, to: &Path) -> Result<(), Error> {
    if !from.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for item in fs::read_dir(from)? {
        let item = item?;
        let (src, dst) = (item.path(), to.join(item.file_name()));
        let kind = item.file_type()?;
        if kind.is_dir() {
            copy_dir(&src, &dst)?;
        } else if kind.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&src)?, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

fn has_status(text: &str, status: clm_model::ChangeStatus) -> bool {
    clm_model::parse_change_frontmatter(text).is_some_and(|fm| fm.status == status)
}

/// The changes the head applies: live at the base with status `proposed` or `approved`,
/// and in the merge commit `applied`, or gone with `changes/archive/<id>` `done`
/// (`regenerate-applied-live-changes`).
fn applied_changes(
    repo: &Repo,
    base: &str,
    read: &dyn Fn(String) -> Option<Vec<u8>>,
) -> Result<Vec<String>, Error> {
    use clm_model::ChangeStatus::{Applied, Approved, Done, Proposed};
    let mut ids = Vec::new();
    for entry in repo.approval.listing(base)? {
        let path = String::from_utf8_lossy(&entry.path).into_owned();
        let Some(id) = path
            .strip_prefix("changes/")
            .and_then(|p| p.strip_suffix("/change.md"))
            .filter(|id| !id.contains('/'))
        else {
            continue;
        };
        let at_base = clm_git::run(
            &repo.root,
            "git",
            &["show", &format!("{base}:./{path}")],
            Default::default(),
        )?;
        let at_base = String::from_utf8_lossy(&at_base);
        if !(has_status(&at_base, Proposed) || has_status(&at_base, Approved)) {
            continue;
        }
        let now = match read(path.clone()) {
            Some(text) => has_status(&String::from_utf8_lossy(&text), Applied),
            None => read(format!("changes/archive/{id}/change.md"))
                .is_some_and(|text| has_status(&String::from_utf8_lossy(&text), Done)),
        };
        if now {
            ids.push(id.to_string());
        }
    }
    Ok(ids)
}

/// What git stores for the file at `path`: a symlink's target, or a file's bytes, told
/// apart; none when there is nothing there.
fn entry(path: &Path) -> Option<(bool, Vec<u8>)> {
    let meta = fs::symlink_metadata(path).ok()?;
    match meta.file_type().is_symlink() {
        true => {
            let target = fs::read_link(path).ok()?;
            Some((true, target.as_os_str().as_encoded_bytes().to_vec()))
        }
        false => Some((false, fs::read(path).ok()?)),
    }
}

/// The files under `dir`, symlinks among them, as paths relative to it; none when it is
/// absent.
fn files(dir: &Path) -> Result<BTreeSet<PathBuf>, Error> {
    let mut out = BTreeSet::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let full = dir.join(&rel);
        if !full.is_dir() {
            continue;
        }
        for item in fs::read_dir(&full)? {
            let item = item?;
            let child = rel.join(item.file_name());
            match item.file_type()?.is_dir() {
                true => stack.push(child),
                false => {
                    out.insert(child);
                }
            }
        }
    }
    Ok(out)
}
