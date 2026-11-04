//! The worktree a land's gate and checks run in, never under `.git`
//! (`land/scratch-outside-git`): the clone's land tree, `<home>/.clm/land-tree`, kept from
//! land to land, for an attempt that holds the lock on `.clm/land` (`land/land-tree`), else
//! a `land-*` directory under the system's temporary directory, removed when dropped, so on
//! every exit path.

use std::path::{Path, PathBuf};

use crate::{Error, Repo, temp_parent};

pub(crate) struct Scratch {
    root: PathBuf,
    /// The top of the worktree.
    top: PathBuf,
    /// Where the CLM repository is in the worktree: its top, or the subdirectory `root` is
    /// in the git repository.
    work: PathBuf,
    /// The temporary directory of a temporary worktree, which is removed when dropped; none
    /// for the land tree, which stays.
    temp: Option<tempfile::TempDir>,
}

impl Scratch {
    /// A fresh detached worktree at `rev` under the system's temporary directory.
    pub(crate) fn temp(repo: &Repo, rev: &str) -> Result<Self, Error> {
        let parent = temp_parent(repo.env)?;
        let dir = tempfile::Builder::new()
            .prefix("land-")
            .tempdir_in(&parent)?;
        let top = dir.path().to_path_buf();
        add(&repo.root, &top, rev)?;
        Self::at(repo, top, Some(dir))
    }

    /// The clone's land tree, `land-tree` in `places` (`<home>/.clm`), at `rev`, clean of
    /// whatever the last land left: checked out and cleaned when git lists it and it is
    /// whole, else made afresh. Only an attempt holding the lock on `.clm/land` may use it.
    pub(crate) fn land_tree(repo: &Repo, places: &Path, rev: &str) -> Result<Self, Error> {
        let top = places.join("land-tree");
        land_tree_at(&repo.root, &top, rev)?;
        Self::at(repo, top, None)
    }

    fn at(repo: &Repo, top: PathBuf, temp: Option<tempfile::TempDir>) -> Result<Self, Error> {
        let prefix = clm_git::git(&repo.root, &["rev-parse", "--show-prefix"])?;
        Ok(Scratch {
            root: repo.root.clone(),
            work: top.join(prefix),
            top,
            temp,
        })
    }

    /// The CLM repository's directory in the worktree, where the gate and the checks run.
    pub(crate) fn path(&self) -> &Path {
        &self.work
    }

    /// Checks `rev` out, dropping whatever an earlier check left behind; the land tree is
    /// made afresh when that fails.
    pub(crate) fn checkout(&self, rev: &str) -> Result<(), Error> {
        match self.temp {
            Some(_) => checkout(&self.top, rev),
            None => land_tree_at(&self.root, &self.top, rev),
        }
    }
}

/// The land tree at `top` with `rev` checked out and clean: checked out and cleaned when git
/// lists it and it is whole; otherwise, or when that fails, whatever is at `top` removed, git's
/// worktree list pruned and the worktree added afresh.
fn land_tree_at(root: &Path, top: &Path, rev: &str) -> Result<(), Error> {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let listed = clm_git::worktrees(root)?
        .iter()
        .any(|w| canon(&w.path) == canon(top));
    if listed && whole(top) && checkout(top, rev).is_ok() {
        return Ok(());
    }
    match std::fs::symlink_metadata(top) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(top)?,
        Ok(_) => std::fs::remove_file(top)?,
        Err(_) => {}
    }
    clm_git::git(root, &["worktree", "prune"])?;
    add(root, top, rev)
}

/// `git worktree add --detach` of `rev` at `top`.
fn add(root: &Path, top: &Path, rev: &str) -> Result<(), Error> {
    let at = top.to_string_lossy();
    clm_git::git(root, &["worktree", "add", "--quiet", "--detach", &at, rev])?;
    Ok(())
}

/// Whether `top` is a worktree of its own: it holds its `.git` file and git finds its top
/// there. Without this, git run in a land tree that lost its `.git` file would find the main
/// worktree around it, and the checkout and clean would act on that.
fn whole(top: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).ok();
    top.join(".git").is_file()
        && canon(top).is_some_and(|own| {
            clm_git::git(top, &["rev-parse", "--show-toplevel"])
                .is_ok_and(|t| canon(Path::new(&t)) == Some(own))
        })
}

/// `rev` checked out at `top`, with nothing tracked, untracked or ignored left from before.
fn checkout(top: &Path, rev: &str) -> Result<(), Error> {
    clm_git::git(top, &["checkout", "--quiet", "--force", "--detach", rev])?;
    clm_git::git(top, &["clean", "--quiet", "-fdx"])?;
    Ok(())
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.temp.is_some() {
            let path = self.top.to_string_lossy().into_owned();
            let _ = clm_git::git(&self.root, &["worktree", "remove", "--force", &path]);
            let _ = clm_git::git(&self.root, &["worktree", "prune"]);
        }
    }
}
