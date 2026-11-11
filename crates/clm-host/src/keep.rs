//! What a clone keeps for itself: the remote of a clone with no host (`host/none-keeps-a-remote`)
//! and a body that reached no pull request whole (`host/keep-unopened-body`).

use std::path::{Path, PathBuf};

use clm_config::Host;
use clm_git::{REMOTE, common_dir, git, git_isolated};

use crate::Error;

/// On `none`, a clone with no [`REMOTE`] gets `clm/remote.git` under its common git directory
/// as a bare repository, added as [`REMOTE`] by its absolute path, and the path comes back.
/// Nothing is pushed and no worktree is written. Otherwise nothing changes.
pub fn keep_remote(root: &Path, host: Host) -> Result<Option<PathBuf>, Error> {
    if host != Host::None || git(root, &["remote"])?.lines().any(|r| r == REMOTE) {
        return Ok(None);
    }
    let bare = common_dir(root)?.join("clm").join("remote.git");
    let path = bare
        .to_str()
        .ok_or_else(|| Error::Refused(format!("{} isn't valid UTF-8", bare.display())))?;
    // `init` on an existing repository keeps what it holds.
    git_isolated(root, &["init", "--quiet", "--bare", path])?;
    git(root, &["remote", "add", REMOTE, path])?;
    Ok(Some(bare))
}

/// Where the body of `branch`'s pull request is kept outside every worktree:
/// `<common git dir>/clm/pr-bodies/<branch>.md`, each `/` of the name written as `-`.
pub(crate) fn body_path(root: &Path, branch: &str) -> Result<PathBuf, Error> {
    Ok(common_dir(root)?
        .join("clm")
        .join("pr-bodies")
        .join(format!("{}.md", branch.replace('/', "-"))))
}

/// Writes `body` to [`body_path`], replacing an earlier copy, and gives the path.
pub(crate) fn keep_body(root: &Path, branch: &str, body: &str) -> Result<PathBuf, Error> {
    let path = body_path(root, branch)?;
    let refuse = |e: std::io::Error| Error::Refused(format!("{}: {e}", path.display()));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(refuse)?;
    }
    std::fs::write(&path, body).map_err(refuse)?;
    Ok(path)
}

/// A text written to a fresh temporary file outside the worktree, removed on drop.
pub(crate) struct TempFile(pub PathBuf);

impl TempFile {
    pub(crate) fn write(text: &str) -> Result<Self, Error> {
        static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("clm-pr-body-{}-{n}.md", std::process::id()));
        let refuse = |e: std::io::Error| Error::Refused(format!("{}: {e}", path.display()));
        // `create_new` refuses a file or symlink already at the path.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(refuse)?;
        let guard = TempFile(path.clone());
        std::io::Write::write_all(&mut file, text.as_bytes()).map_err(refuse)?;
        Ok(guard)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
