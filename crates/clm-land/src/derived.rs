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
    let merge_base = repo.git(&["merge-base", base, head])?;
    let at_base = repo.approval.listing(&merge_base)?;
    let at_head = repo.approval.listing(head)?;
    let added = clm_gate::added_changes(&at_base, &at_head);
    let applied = applied_changes(repo, base, merged)?;
    if added.is_empty() && applied.is_empty() {
        return Ok(());
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
    let refuse = |why: String| {
        Error::Refused(format!(
            "land/derived-specs: {why}: run `clm rebase` on the branch, push it, and land it again"
        ))
    };

    // `sync.regenerate`'s regenerate, without its merge, from the CLM root in the tree on
    // the merge commit: the base's `specs/` with that commit's changes and backlog.
    // A refusal of its own is a stop too, so it names the rule and `clm rebase`.
    let regenerated = match clm_regenerate::regenerated(merged, base, &ids, false) {
        Err(clm_regenerate::Error::Refused(why)) => return Err(refuse(why)),
        other => other?,
    };
    let tree = match regenerated {
        clm_regenerate::Regenerated::Tree(tree) => tree,
        clm_regenerate::Regenerated::Stopped { left, .. } => {
            return Err(refuse(format!(
                "the branch's changes don't regenerate from the base:\n  {}\n",
                left.join("\n  ")
            )));
        }
    };
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

fn has_status(text: &str, status: clm_model::ChangeStatus) -> bool {
    clm_model::parse_change_frontmatter(text).is_some_and(|fm| fm.status == status)
}

/// The changes the head applies: live at the base with status `proposed` or `approved`,
/// and in the merge commit `applied`, or gone with `changes/archive/<id>` `done`
/// (`regenerate-applied-live-changes`).
fn applied_changes(repo: &Repo, base: &str, merged: &Path) -> Result<Vec<String>, Error> {
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
        let read = |rel: String| fs::read(merged.join(rel)).ok();
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
