//! The verify tree: a worktree at `<home>/.clm/verify-tree` where `[verify] check` runs, kept
//! from run to run and held under one lock for as long as a caller uses it, so two runs of one
//! clone never move it under each other.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use crate::{Ctx, Error, say};

/// `<main worktree>/<prefix>/.clm`: where a land keeps what it keeps, and so does a
/// verification (`land/land-tree`).
pub(crate) fn places(ctx: &Ctx) -> Result<PathBuf, Error> {
    places_of(&ctx.root, &ctx.prefix)
}

/// `<places>/verify-build`, the place `[verify] check` and `[verify] mutate` build in
/// (`verify-build-dir`), created when missing so a command may write into it before building
/// anything there. A place that can't be created is said on stderr and the command runs all
/// the same, finding it missing.
pub(crate) fn build_place(ctx: &Ctx) -> Result<PathBuf, Error> {
    let build = places(ctx)?.join("verify-build");
    if let Err(e) = std::fs::create_dir_all(&build) {
        say(&format!("clm verify: cannot create .clm/verify-build: {e}"));
    }
    Ok(build)
}

/// `places` for the CLM root `root`, whose place in its git repository is `prefix`. The main
/// worktree is found from the common git directory, as `git worktree list` finds it, without
/// listing the worktrees: a run that waits for the verify lock would otherwise read the entry
/// of a worktree another run is still adding, and fail on it.
pub(crate) fn places_of(root: &Path, prefix: &str) -> Result<PathBuf, Error> {
    let common = clm_git::common_dir(root)?;
    let main = match common.file_name() {
        Some(name) if name == ".git" => common.parent().map_or(common.clone(), Path::to_path_buf),
        _ => common,
    };
    Ok(main.join(prefix).join(".clm"))
}

/// The tree at a commit.
pub(crate) struct Tree {
    /// The CLM root inside the tree.
    pub(crate) dir: PathBuf,
}

impl Tree {
    /// Waits for the verify lock, unless the call already holds it, then checks `commit` out,
    /// clean of untracked files (files git ignores stay, so what the check builds serves the
    /// call's mutation pass, until `clear` removes them at the call's end:
    /// `verify-tree-cleared`). The call holds the lock from here until `clear`, so no other
    /// call cleans or moves the tree between its check and its pass.
    pub(crate) fn at(ctx: &Ctx, commit: &str) -> Result<Tree, Error> {
        let places = places(ctx)?;
        if ctx.tree_lock.borrow().is_none() {
            let held = lock(&places.join("verify.lock"))?;
            *ctx.tree_lock.borrow_mut() = Some(held);
        }
        let top = places.join("verify-tree");
        checkout(&ctx.root, &top, commit)?;
        ctx.checked_out.set(true);
        Ok(Tree {
            dir: top.join(&ctx.prefix),
        })
    }
}

/// Removes from the verify tree every file git doesn't track, those it ignores included, as
/// `git clean -ffdx` at its top does, under the verify lock the call holds, which it then
/// lets go (`verify-tree-cleared`).
/// Only a tree whose own top git finds there is cleaned: in a tree whose `.git` is gone, git
/// finds the main worktree around it. A clean that can't be made is said on stderr and
/// changes nothing else.
pub(crate) fn clear(ctx: &Ctx) {
    let cleared = || -> Result<(), String> {
        let places = places(ctx).map_err(|e| e.to_string())?;
        let _lock = match ctx.tree_lock.borrow_mut().take() {
            Some(held) => held,
            None => lock(&places.join("verify.lock")).map_err(|e| e.to_string())?,
        };
        let top = places.join("verify-tree");
        own_top(&top)?;
        clm_git::git(&top, &["clean", "--quiet", "-ffdx"]).map_err(|e| e.to_string())?;
        Ok(())
    };
    if let Err(why) = cleared() {
        say(&format!("clm verify: cannot clean .clm/verify-tree: {why}"));
    }
}

/// Whether git run in `top` finds its top at `top` itself, and if not, why.
fn own_top(top: &Path) -> Result<(), String> {
    let own = std::fs::canonicalize(top).map_err(|e| format!("{}: {e}", top.display()))?;
    let found = clm_git::git(top, &["rev-parse", "--show-toplevel"]).map_err(|e| e.to_string())?;
    if std::fs::canonicalize(&found).ok() != Some(own) {
        return Err(format!("git finds its top at {found}, not the tree"));
    }
    Ok(())
}

/// Locks `path`, creating it and its directory, and says once when another run holds it.
fn lock(path: &Path) -> Result<File, Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            say(&format!("waiting for another check on {}", path.display()));
            file.lock()?;
        }
        Err(TryLockError::Error(e)) => return Err(e.into()),
    }
    Ok(file)
}

/// The worktree at `top` with `commit` checked out: reused when git lists it and it is
/// whole, else made afresh.
fn checkout(root: &Path, top: &Path, commit: &str) -> Result<(), Error> {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let listed = clm_git::worktrees(root)?
        .iter()
        .any(|w| canon(&w.path) == canon(top));
    if listed && whole(top) && switch(top, commit).is_ok() {
        return Ok(());
    }
    match std::fs::symlink_metadata(top) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(top)?,
        Ok(_) => std::fs::remove_file(top)?,
        Err(_) => {}
    }
    clm_git::git(root, &["worktree", "prune"])?;
    let at = top.to_string_lossy();
    clm_git::git(
        root,
        &["worktree", "add", "--quiet", "--detach", &at, commit],
    )?;
    Ok(())
}

/// Whether `top` is a worktree of its own: git run there finds its top at `top`, not the
/// worktree around it.
fn whole(top: &Path) -> bool {
    top.join(".git").is_file() && own_top(top).is_ok()
}

fn switch(top: &Path, commit: &str) -> Result<(), clm_git::Error> {
    clm_git::git(top, &["checkout", "--quiet", "--force", "--detach", commit])?;
    clm_git::git(top, &["clean", "--quiet", "-fd"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_ignore_a_worktree_still_being_added() {
        // req:verify/verify-run@ff539674
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main");
        std::fs::create_dir(&main).unwrap();
        clm_git::git_isolated(&main, &["init", "--quiet"]).unwrap();
        // The entry `git worktree add` has begun and not finished: `git worktree list`
        // fails on it.
        let half = main.join(".git/worktrees/half");
        std::fs::create_dir_all(&half).unwrap();
        let gitdir = dir.path().join("half/.git");
        std::fs::write(half.join("gitdir"), gitdir.display().to_string()).unwrap();
        std::fs::write(half.join("commondir"), "").unwrap();
        assert!(clm_git::worktrees(&main).is_err());
        let sub = main.join("sub");
        std::fs::create_dir(&sub).unwrap();
        let canon = std::fs::canonicalize(&main).unwrap();
        assert_eq!(places_of(&sub, "sub/").unwrap(), canon.join("sub/.clm"));
    }
}
