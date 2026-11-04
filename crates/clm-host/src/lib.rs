//! The host adapter: pages, remote branch deletion and issues (specs/host/SPEC.md). Every
//! process runs through `clm-git`.

mod github;

use std::path::Path;

use clm_git::{REMOTE, git};
use github::GitHub;

#[derive(Debug)]
pub enum Error {
    /// The work isn't possible; the message says why.
    Refused(String),
    /// There is no host to file an issue with; this is the report that wasn't filed.
    Nowhere(String),
    /// `git` or the host's tool failed.
    Git(String),
}

impl From<clm_git::Error> for Error {
    fn from(e: clm_git::Error) -> Self {
        Error::Git(e.to_string())
    }
}

/// What CLM asks of a git host. One implementation per `host` setting.
pub trait Host {
    /// Opens the page for `branch` against `main`, titled `title` with `body_file`'s
    /// contents, or finds the one already open, and returns its URL; `None` without a host.
    fn page_open(
        &self,
        root: &Path,
        branch: &str,
        title: &str,
        body_file: &Path,
    ) -> Result<Option<String>, Error>;

    /// Sets the body of the current branch's open page to `file`'s contents and nothing
    /// else, and returns `updated <page>`; `None` without a host. Refuses, calling nothing,
    /// when the file is missing or blank, or when the branch has no open page.
    fn page_body(&self, root: &Path, file: &Path) -> Result<Option<String>, Error>;

    /// Files an issue in `repo` titled `title` with `body_file`'s contents where the host
    /// keeps them, and returns its URL; [`Error::Nowhere`] without a host.
    fn issue(
        &self,
        root: &Path,
        title: &str,
        body_file: &Path,
        repo: &str,
    ) -> Result<String, Error>;

    /// Deletes [`REMOTE`]'s `branch` when its tip is still `lease`: plain git on every host,
    /// and nothing to do without that remote or when the branch is already gone.
    fn branch_delete(&self, root: &Path, branch: &str, lease: &str) -> Result<(), Error> {
        if !git(root, &["remote"])?.lines().any(|r| r == REMOTE) {
            return Ok(());
        }
        let refname = format!("refs/heads/{branch}");
        let with_lease = format!("--force-with-lease={refname}:{lease}");
        match git(root, &["push", &with_lease, REMOTE, &format!(":{refname}")]) {
            Ok(_) => Ok(()),
            // A failed lease: the tip moved, or the branch is gone, which leaves nothing to do.
            Err(clm_git::Error::Failed { stderr, .. }) if stderr.contains("stale info") => {
                if git(root, &["ls-remote", REMOTE, &refname])?.is_empty() {
                    return Ok(());
                }
                Err(Error::Refused(format!(
                    "`{branch}` on origin is no longer at {lease}; left it"
                )))
            }
            Err(e) => Err(e.into()),
        }
    }
}

/// `host = "git"` and `"none"`: no host, so nothing to do beyond git.
struct NoHost;

impl Host for NoHost {
    fn page_open(&self, _: &Path, _: &str, _: &str, _: &Path) -> Result<Option<String>, Error> {
        Ok(None)
    }

    fn page_body(&self, _: &Path, _: &Path) -> Result<Option<String>, Error> {
        Ok(None)
    }

    fn issue(&self, root: &Path, _: &str, body_file: &Path, _: &str) -> Result<String, Error> {
        let report = std::fs::read_to_string(root.join(body_file))
            .map_err(|e| Error::Refused(format!("{}: {e}", body_file.display())))?;
        Err(Error::Nowhere(report))
    }
}

/// Refuses a body file that is missing or holds only whitespace: a page body can't be
/// blanked.
fn check_body(root: &Path, file: &Path) -> Result<(), Error> {
    let text = std::fs::read_to_string(root.join(file))
        .map_err(|e| Error::Refused(format!("{}: {e}", file.display())))?;
    if text.trim().is_empty() {
        return Err(Error::Refused(format!(
            "{} holds only whitespace; a page body can't be blanked",
            file.display()
        )));
    }
    Ok(())
}

/// The adapter the `host` setting names.
pub fn adapter(host: clm_config::Host) -> Box<dyn Host> {
    match host {
        clm_config::Host::Github => Box::new(GitHub),
        clm_config::Host::Git | clm_config::Host::None => Box::new(NoHost),
    }
}
