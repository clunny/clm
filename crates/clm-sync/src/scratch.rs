//! `judged-by-check` (`sync-judges-head`): the temporary worktree `HEAD` is judged in, a
//! `sync-*` directory under the system's temporary directory, outside the repository as
//! `clm land`'s are (`scratch-outside-git`), so no uncommitted edit and no untracked or
//! ignored file decides the outcome.

use std::path::{Path, PathBuf};

use crate::git;

use crate::Error;

pub(crate) struct Scratch {
    /// The repository's CLM root, where `git worktree` runs.
    root: PathBuf,
    /// Dropped last: whatever `git worktree remove` left goes with it.
    dir: Option<tempfile::TempDir>,
    /// The CLM root in the worktree.
    clm_root: PathBuf,
}

impl Scratch {
    /// `HEAD` checked out detached into a new `sync-*` directory under `TMPDIR` (when set
    /// and not empty, made absolute) or the system's temporary directory.
    pub(crate) fn new(root: &Path) -> Result<Self, Error> {
        let parent = std::env::var_os("TMPDIR")
            .filter(|t| !t.is_empty())
            .map_or_else(std::env::temp_dir, PathBuf::from);
        let parent = std::path::absolute(parent).map_err(|e| crate::io(Path::new("TMPDIR"), e))?;
        let dir = tempfile::Builder::new()
            .prefix("sync-")
            .tempdir_in(&parent)
            .map_err(|e| crate::io(&parent, e))?;
        let path = dir.path().to_string_lossy().into_owned();
        // Before the worktree exists, so nothing between making it and owning it can fail.
        let prefix = git(root, &["rev-parse", "--show-prefix"])?;
        git(
            root,
            &["worktree", "add", "--quiet", "--detach", &path, "HEAD"],
        )?;
        let clm_root = match prefix.trim_end_matches('/') {
            "" => dir.path().to_path_buf(),
            prefix => dir.path().join(prefix),
        };
        Ok(Scratch {
            root: root.to_path_buf(),
            dir: Some(dir),
            clm_root,
        })
    }

    /// The CLM root in the worktree, where the settings load and the check runs.
    pub(crate) fn clm_root(&self) -> &Path {
        &self.clm_root
    }

    /// Moves the worktree to `rev`, such as the commit a reconsider item just made.
    pub(crate) fn checkout(&self, rev: &str) -> Result<(), Error> {
        git(&self.clm_root, &["checkout", "--quiet", "--detach", rev])?;
        Ok(())
    }

    /// `text` with the temporary CLM root, leading or not, replaced by `root`, the CLM root
    /// the caller gave, so a line names the file the caller would edit. The root is a fresh
    /// `sync-*` directory, so nothing else in a line spells it.
    pub(crate) fn unscratched(&self, text: &str, root: &Path) -> String {
        let scratch = self.clm_root.to_string_lossy();
        text.replace(scratch.as_ref(), &root.to_string_lossy())
    }

    /// `path` with a leading temporary CLM root replaced by `root`.
    pub(crate) fn unscratched_path(&self, path: &Path, root: &Path) -> PathBuf {
        match path.strip_prefix(&self.clm_root) {
            Ok(rest) => root.join(rest),
            Err(_) => path.to_path_buf(),
        }
    }

    /// Removes the worktree and git's record of it; a warning naming the directory when
    /// that fails.
    pub(crate) fn remove(mut self) -> Option<String> {
        self.remove_now()
    }

    fn remove_now(&mut self) -> Option<String> {
        let dir = self.dir.take()?;
        let path = dir.path().to_string_lossy().into_owned();
        let removed = git(&self.root, &["worktree", "remove", "--force", &path]);
        let pruned = git(&self.root, &["worktree", "prune"]);
        match (removed, pruned) {
            (Ok(_), Ok(_)) => None,
            (Err(e), _) | (_, Err(e)) => Some(format!(
                "couldn't remove the temporary worktree {path}: {e}"
            )),
        }
    }
}

impl Drop for Scratch {
    /// Removed on every exit path, a git failure included.
    fn drop(&mut self) {
        let _ = self.remove_now();
    }
}
