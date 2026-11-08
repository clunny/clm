//! The verify tree: a worktree at `<home>/.clm/verify-tree` where `[verify] check` runs, kept
//! from run to run and held under one lock for as long as a caller uses it, so two runs of one
//! clone never move it under each other.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

use crate::{Ctx, Error};

/// `<main worktree>/<prefix>/.clm`: where a land keeps what it keeps, and so does a
/// verification (`land/land-tree`).
pub(crate) fn places(ctx: &Ctx) -> Result<PathBuf, Error> {
    let main = clm_git::worktrees(&ctx.root)?
        .into_iter()
        .next()
        .map_or_else(|| ctx.root.clone(), |w| w.path);
    Ok(main.join(&ctx.prefix).join(".clm"))
}

/// The tree at a commit, held exclusively until dropped.
pub(crate) struct Tree {
    /// The CLM root inside the tree.
    pub(crate) dir: PathBuf,
    _lock: File,
}

impl Tree {
    /// Waits for the verify lock, then checks `commit` out, clean of untracked files (files
    /// git ignores stay, so a build directory is reused from run to run).
    pub(crate) fn at(ctx: &Ctx, commit: &str) -> Result<Tree, Error> {
        let places = places(ctx)?;
        let lock = lock(&places.join("verify.lock"))?;
        let top = places.join("verify-tree");
        checkout(&ctx.root, &top, commit)?;
        Ok(Tree {
            dir: top.join(&ctx.prefix),
            _lock: lock,
        })
    }
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
            eprintln!("waiting for another check on {}", path.display());
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
    let canon = |p: &Path| std::fs::canonicalize(p).ok();
    top.join(".git").is_file()
        && canon(top).is_some_and(|own| {
            clm_git::git(top, &["rev-parse", "--show-toplevel"])
                .is_ok_and(|t| canon(Path::new(&t)) == Some(own))
        })
}

fn switch(top: &Path, commit: &str) -> Result<(), clm_git::Error> {
    clm_git::git(top, &["checkout", "--quiet", "--force", "--detach", commit])?;
    clm_git::git(top, &["clean", "--quiet", "-fd"])?;
    Ok(())
}
