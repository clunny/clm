//! The remote a clone with no host keeps for itself (`host/none-keeps-a-remote`).

use std::path::{Path, PathBuf};

use clm_config::Host;
use clm_git::{REMOTE, common_dir, git};

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
    git(root, &["init", "--quiet", "--bare", path])?;
    git(root, &["remote", "add", REMOTE, path])?;
    Ok(Some(bare))
}
