//! The binary `clm land` runs its subcommands from: a copy of its own, in a `land-bin-*`
//! directory under the system's temporary directory, so a check that rebuilds or deletes
//! the running `clm` can't take it away mid-land (`land/land-binary-copy`). Removed when
//! dropped, so on every exit path.

use std::path::{Path, PathBuf};

use crate::{Error, Repo, temp_parent};

pub(crate) struct Exe {
    path: PathBuf,
    /// The directory holding the copy; none when `CLM_GATE`, `CLM_CHECK` and
    /// `CLM_AFFECTED` name every command, so no subcommand runs from this binary and `path`
    /// is never run.
    _dir: Option<tempfile::TempDir>,
}

impl Exe {
    pub(crate) fn new(repo: &Repo) -> Result<Self, Error> {
        let overridden = |var: &str| (repo.env)(var).is_some_and(|c| !c.trim().is_empty());
        if ["CLM_GATE", "CLM_CHECK", "CLM_AFFECTED"]
            .into_iter()
            .all(overridden)
        {
            return Ok(Exe {
                path: PathBuf::new(),
                _dir: None,
            });
        }
        let own = std::env::current_exe()
            .map_err(|e| Error::Refused(format!("can't find `clm`'s own binary to copy: {e}")))?;
        let parent = temp_parent(repo.env).map_err(|e| {
            Error::Refused(format!("can't copy `clm`: no temporary directory: {e}"))
        })?;
        let refused = |at: &Path, e: std::io::Error| {
            Error::Refused(format!(
                "can't copy `clm` ({}) to {}: {e}",
                own.display(),
                at.display()
            ))
        };
        let dir = tempfile::Builder::new()
            .prefix("land-bin-")
            .tempdir_in(&parent)
            .map_err(|e| refused(&parent, e))?;
        let name = own.file_name().unwrap_or("clm".as_ref());
        let path = dir.path().join(name);
        // `fs::copy` keeps the file's permissions, so the copy stays executable.
        std::fs::copy(&own, &path).map_err(|e| refused(&path, e))?;
        Ok(Exe {
            path,
            _dir: Some(dir),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}
