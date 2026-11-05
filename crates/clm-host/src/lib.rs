//! The host adapter: pages, remote branch deletion and issues (specs/host/SPEC.md). Every
//! process runs through `clm-git`.

mod github;

use std::path::Path;

use clm_git::{REMOTE, current_branch, git};
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

/// A pull request or an issue, as `show` reports it. What it holds is a stranger's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub pull_request: bool,
    pub author: String,
    /// The author's numeric account ID, which a `Co-authored-by` noreply address needs.
    pub author_id: u64,
    pub state: String,
    pub title: String,
    pub body: String,
    /// A pull request's head repository URL.
    pub head_repo_url: Option<String>,
    pub head_branch: Option<String>,
    pub maintainers_can_edit: bool,
    pub files: Vec<String>,
}

/// An open issue or pull request, as `list` reports it: no text its author wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub number: u64,
    pub pull_request: bool,
    pub author: String,
}

fn no_host<T>() -> Result<T, Error> {
    Err(Error::Refused("there is no host".into()))
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

    /// The fork's parent as `<owner>/<name>`; `None` when [`REMOTE`] isn't a fork.
    fn parent(&self, _root: &Path) -> Result<Option<String>, Error> {
        Ok(None)
    }

    /// Brings [`REMOTE`]'s `main` up to its parent's, without force.
    fn sync(&self, _root: &Path) -> Result<(), Error> {
        no_host()
    }

    /// Opens a pull request from this fork's `branch` to the parent's `main`, or finds the
    /// open one, and returns its URL.
    fn open_on_parent(
        &self,
        _root: &Path,
        _branch: &str,
        _title: &str,
        _body_file: &Path,
    ) -> Result<String, Error> {
        no_host()
    }

    /// Pull request or issue `number`.
    fn show(&self, _root: &Path, _number: u64) -> Result<Shown, Error> {
        no_host()
    }

    /// Makes local `branch` at pull request `number`'s head, without checking it out;
    /// refuses a branch that exists.
    fn fetch(&self, _root: &Path, _number: u64, _branch: &str) -> Result<(), Error> {
        no_host()
    }

    /// Comments `body_file`'s contents on `number`, closing it when `close` is set.
    fn comment(
        &self,
        _root: &Path,
        _number: u64,
        _body_file: &Path,
        _close: bool,
    ) -> Result<(), Error> {
        no_host()
    }

    /// Adds `label` to `number`.
    fn label(&self, _root: &Path, _number: u64, _label: &str) -> Result<(), Error> {
        no_host()
    }

    /// The open issues and pull requests; empty without a host.
    fn list(&self, _root: &Path) -> Result<Vec<Listed>, Error> {
        Ok(Vec::new())
    }

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

/// Why a branch is never pushed or deleted by `clm branch`: `main` moves only through
/// `clm land`, and the `clm/` branches are the ledgers' (`guard-blocks-push-to-main`).
fn refuse_protected(branch: &str) -> Result<(), Error> {
    if branch == "main" {
        return Err(Error::Refused(
            "`main` moves only through `clm land`".into(),
        ));
    }
    if branch.starts_with("clm/") {
        return Err(Error::Refused(format!(
            "`{branch}` is a ledger branch; only `clm` itself writes it"
        )));
    }
    Ok(())
}

fn has_remote(root: &Path) -> Result<bool, Error> {
    Ok(git(root, &["remote"])?.lines().any(|r| r == REMOTE))
}

/// `clm branch delete` (`branch-delete`): deletes [`REMOTE`]'s `branch` through `host` when
/// its tip is `lease` (default: the tip last fetched), and only a tip this clone pushed.
/// Nothing to do, and success, without that remote or when it has no such branch.
pub fn delete_pushed_branch(
    host: &dyn Host,
    root: &Path,
    branch: &str,
    lease: Option<&str>,
) -> Result<(), Error> {
    if !has_remote(root)? {
        return Ok(());
    }
    let refname = format!("refs/heads/{branch}");
    if git(root, &["ls-remote", REMOTE, &refname])?.is_empty() {
        return Ok(());
    }
    refuse_protected(branch)?;
    let tracking = format!("refs/remotes/{REMOTE}/{branch}");
    let tip = git(root, &["rev-parse", "--verify", "-q", &tracking]).unwrap_or_default();
    let lease = match lease {
        Some(lease) => lease.to_string(),
        None if tip.is_empty() => {
            return Err(Error::Refused(format!(
                "no {REMOTE}/{branch} here to take the tip from; fetch, or pass --lease"
            )));
        }
        None => tip,
    };
    // Git writes `update by push` when a push from this clone moves the branch, and another
    // entry when a fetch brings someone else's tip; no reflog fails closed.
    let newest = git(
        root,
        &["reflog", "show", "-n", "1", "--format=%H %gs", &tracking],
    )
    .unwrap_or_default();
    let pushed_here = newest
        .split_once(' ')
        .is_some_and(|(sha, why)| sha == lease && why.starts_with("update by push"));
    if !pushed_here {
        return Err(Error::Refused(format!(
            "this clone didn't push {REMOTE}/{branch} at {lease}; left it"
        )));
    }
    host.branch_delete(root, branch, &lease)
}

/// `clm branch push` (`branch-push`): pushes the checked-out branch to [`REMOTE`] as a
/// compare-and-swap on what this clone last saw, and returns `pushed <branch> <sha>`.
pub fn push_branch(root: &Path) -> Result<String, Error> {
    let branch = current_branch(root)?
        .ok_or_else(|| Error::Refused("`HEAD` is detached; check out a branch".into()))?;
    refuse_protected(&branch)?;
    if !has_remote(root)? {
        return Err(Error::Refused(format!("there is no `{REMOTE}` remote")));
    }
    let refname = format!("refs/heads/{branch}");
    let tracking = format!("refs/remotes/{REMOTE}/{branch}");
    let seen = git(root, &["rev-parse", "--verify", "-q", &tracking]).is_ok();
    let lease = if seen {
        format!("--force-with-lease={refname}")
    } else {
        format!("--force-with-lease={refname}:")
    };
    let mut args = vec!["push", "--set-upstream", &lease];
    if seen {
        args.push("--force-if-includes");
    }
    let target = format!("HEAD:{refname}");
    args.extend([REMOTE, &target]);
    match git(root, &args) {
        Ok(_) => {}
        Err(clm_git::Error::Failed { stderr, .. }) => return Err(Error::Refused(stderr)),
        Err(e) => return Err(e.into()),
    }
    Ok(format!(
        "pushed {branch} {}",
        git(root, &["rev-parse", "HEAD"])?
    ))
}
