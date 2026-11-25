//! Whether a branch's diff is approved or held by a request for changes, `clm approve`, and
//! the signing of approvals and lands (specs/land/approval/SPEC.md, `signed-ledger-records`).
//! Carved out of `clm-land`, which calls it, so it takes the CLM root, the environment lookup
//! and the loaded settings as arguments, runs git through `clm-git`, and has an error type of
//! its own.

mod approve;
mod command;
mod hold;
mod policy;
mod sign;

use std::fmt;
use std::path::{Path, PathBuf};

pub use approve::approve;
pub use command::{ApproveArgs, approve_command};
pub use policy::Policy;
pub use sign::{APPROVE, LAND};

use clm_git::REMOTE;
use clm_ledger::{Fold, person, slug};

/// The environment `config` reads.
pub type Env = dyn Fn(&str) -> Option<String>;

#[derive(Debug)]
pub enum Error {
    /// Nothing was done, for the reason given: exit 1.
    Refused(String),
    /// `git`, the ledger or the settings failed: exit 2.
    Git(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refused(why) | Error::Git(why) => f.write_str(why),
        }
    }
}

impl From<clm_git::Error> for Error {
    fn from(e: clm_git::Error) -> Self {
        Error::Git(e.to_string())
    }
}

impl From<clm_ledger::Error> for Error {
    fn from(e: clm_ledger::Error) -> Self {
        Error::Git(e.to_string())
    }
}

impl From<clm_config::Error> for Error {
    fn from(e: clm_config::Error) -> Self {
        Error::Git(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Git(e.to_string())
    }
}

/// What a command prints: `lines` on stdout, `warnings` on stderr.
#[derive(Debug, Default)]
pub struct Output {
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

const OFFLINE: &str = "offline: origin can't be reached; run `clm approve` again when it can";

/// A branch's head, and `origin`'s tip of the branch when `origin` holds it (`branch-head`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchHead {
    pub head: String,
    pub origin: Option<String>,
}

/// What the ledger's approvals of a diff come to.
pub struct Approvals {
    /// One of them counts.
    pub accepted: bool,
    /// When none counts, each one with why it doesn't.
    pub refused: Vec<String>,
}

/// The repository an approval is judged in: the CLM root, the environment and the settings
/// the judging reads.
pub struct Repo<'a> {
    root: PathBuf,
    env: &'a Env,
    /// The ref `main` is landed on: `origin/main`, on every host (`approval-from-base`).
    base: String,
    /// `approval`, and the layer the clone's `clm.local.toml` may set it in.
    approval: clm_config::Setting<clm_config::Approval>,
}

impl<'a> Repo<'a> {
    /// The repository at `root` (the CLM root, canonical), with its loaded `settings`.
    pub fn new(root: &Path, env: &'a Env, settings: &clm_config::Settings) -> Self {
        Repo {
            root: root.to_path_buf(),
            env,
            base: format!("{REMOTE}/main"),
            approval: settings.approval.clone(),
        }
    }

    /// The ref `main` is landed on: `origin/main`.
    pub(crate) fn base(&self) -> &str {
        &self.base
    }

    pub(crate) fn git(&self, args: &[&str]) -> Result<String, Error> {
        Ok(clm_git::git(&self.root, args)?)
    }

    /// `git fetch --prune origin`, tried again after a failure (`approve-retries-fetch`):
    /// offline when `origin` can't be reached, else refused with git's message from the last
    /// fetch. It prunes, so a tracking ref of a branch `origin` deleted gives no head.
    pub(crate) fn fetch(&self) -> Result<(), Error> {
        match clm_git::fetch_retrying(&self.root, &["fetch", "--prune", REMOTE]) {
            Ok(()) => Ok(()),
            Err(clm_git::FetchError::Offline) => Err(Error::Refused(OFFLINE.into())),
            Err(clm_git::FetchError::Git(e)) => Err(Error::Refused(e.to_string())),
        }
    }

    /// The commit `rev` names, or `None` when it names none.
    pub fn rev(&self, rev: &str) -> Result<Option<String>, Error> {
        let commit = format!("{rev}^{{commit}}");
        match clm_git::git(&self.root, &["rev-parse", "--verify", "--quiet", &commit]) {
            Ok(sha) => Ok(Some(sha)),
            Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// `branch`, or the current one; refused when it is `main` or there is none. `verb` says
    /// what can't be done to `main`.
    pub fn branch_name(&self, branch: Option<&str>, verb: &str) -> Result<String, Error> {
        let name = match branch {
            Some(b) => b.to_string(),
            None => clm_git::current_branch(&self.root)?
                .ok_or_else(|| Error::Refused("HEAD is detached: name the branch".into()))?,
        };
        if name == "main" {
            return Err(Error::Refused(format!("`main` can't be {verb}")));
        }
        Ok(name)
    }

    /// `branch`'s head by one rule for `clm land` (`for_land`) and `clm approve`, whatever its
    /// name, read from `refs/heads/<branch>` and `refs/remotes/origin/<branch>` as the caller's
    /// fetch left them (`branch-head`). A land takes the local branch when it holds everything
    /// `origin` holds of it, and is refused when it is behind or has diverged; an approve takes
    /// `origin`'s tip when the local branch is absent or an ancestor of it.
    pub fn branch_head(&self, branch: &str, for_land: bool) -> Result<BranchHead, Error> {
        let local = self.rev(&format!("refs/heads/{branch}"))?;
        let origin = self.rev(&format!("refs/remotes/{REMOTE}/{branch}"))?;
        let ancestor = |a: &str, b: &str| clm_git::is_ancestor(&self.root, a, b);
        let head = match (&local, &origin) {
            (None, None) => return Err(Error::Refused(format!("no branch `{branch}`"))),
            (Some(l), None) => l.clone(),
            (None, Some(o)) => o.clone(),
            (Some(l), Some(o)) if !for_land => match ancestor(l, o)? {
                true => o.clone(),
                false => l.clone(),
            },
            (Some(l), Some(o)) if ancestor(o, l)? => l.clone(),
            (Some(l), Some(o)) => {
                let tracking = format!("{REMOTE}/{branch}");
                return Err(Error::Refused(match ancestor(l, o)? {
                    true => format!(
                        "`{branch}` is behind `{tracking}`: bring it up to `{tracking}`, then \
                         land it"
                    ),
                    false => format!(
                        "`{branch}` has diverged from `{tracking}`: rebase it onto `{tracking}` \
                         or merge `{tracking}` into it, then land it"
                    ),
                }));
            }
        };
        Ok(BranchHead { head, origin })
    }

    /// The merge base of the base and `head`.
    pub(crate) fn merge_base(&self, head: &str) -> Result<String, Error> {
        self.git(&["merge-base", self.base(), head])
    }

    /// The fingerprint of `head`'s changes relative to `base`, as `clm fingerprint` computes
    /// it, from the trees alone: no worktree is needed.
    pub fn fingerprint(&self, base: &str, head: &str) -> Result<String, Error> {
        let merge_base = self.git(&["merge-base", base, head])?;
        Ok(clm_gate::fingerprint_of(
            &self.listing(&merge_base)?,
            &self.listing(head)?,
        ))
    }

    /// `rev`'s tree entries under the CLM repository, as `git ls-tree -r -z` lists them.
    pub fn listing(&self, rev: &str) -> Result<Vec<clm_gate::Entry>, Error> {
        let out = clm_git::run(
            &self.root,
            "git",
            &["ls-tree", "-r", "-z", rev],
            Default::default(),
        )?;
        Ok(clm_gate::parse_ls_tree(&out))
    }

    /// The live change IDs `head` adds, modifies or changes the type of relative to the base,
    /// sorted, as `ledger`'s `change_ids` names them. Renames are off, so a renamed change
    /// directory counts as added; `-z` keeps a non-ASCII ID unquoted, and `--no-relative`
    /// keeps a user's `diff.relative` from emptying the list.
    pub fn changes(&self, head: &str) -> Result<Vec<String>, Error> {
        let base = self.merge_base(head)?;
        let names = self.git(&[
            "diff",
            "--name-only",
            "--no-renames",
            "--no-relative",
            "-z",
            "--diff-filter=AMT",
            &base,
            head,
            "--",
            "changes/",
        ])?;
        Ok(clm_ledger::change_ids(names.split('\0')))
    }

    /// Whether the fold holds an approval of `fingerprint` that counts, and, when none does,
    /// why each one doesn't (`approvals-counted`). With `other` (`approval = "other"` in the
    /// policy the land reads, `policy-at-base`), an approval counts only when its writer's
    /// person is none of the authors of the commits from the merge base to `head`, slugged
    /// with `ledger`'s `slug`. In a repository signed at `signed`, it counts only when its
    /// signature proves its writer, and, with `other`, when its writer isn't the lander.
    pub fn approval(
        &self,
        fold: &Fold,
        fingerprint: &str,
        head: &str,
        signed: Option<&str>,
        other: bool,
    ) -> Result<Approvals, Error> {
        let records = fold
            .approvals
            .get(fingerprint)
            .map_or(&[][..], Vec::as_slice);
        let mut authors = Vec::new();
        let mut lander = None;
        if other && !records.is_empty() {
            let range = format!("{}..{head}", self.merge_base(head)?);
            authors = self
                .git(&["log", "--format=%an", &range])?
                .lines()
                .map(slug)
                .collect();
            if signed.is_some() {
                let writer = clm_ledger::writer(&self.root, self.env)?;
                lander = Some(person(&writer).to_string());
            }
        }
        let mut refused = Vec::new();
        for record in records {
            let person = person(&record.writer);
            let proven = || -> Result<bool, Error> {
                let (Some(base), Some(line)) = (signed, fold.lines.get(&record.id)) else {
                    return Ok(signed.is_none());
                };
                Ok(clm_ledger::signed_by(
                    &self.root,
                    base,
                    line,
                    sign::APPROVE,
                )?)
            };
            let why = if !proven()? {
                "not signed by its writer: approve with `clm approve` and an SSH signing key \
                 (git's user.signingkey); an approval made in web is never signed"
            } else if authors.iter().any(|a| a == person) {
                "approval = \"other\": its writer is one of the branch's authors"
            } else if lander.as_deref() == Some(person) {
                "approval = \"other\": its writer is the lander, who can't approve their own land"
            } else {
                return Ok(Approvals {
                    accepted: true,
                    refused: Vec::new(),
                });
            };
            refused.push(format!("{} at {}: {why}", record.writer, record.at));
        }
        Ok(Approvals {
            accepted: false,
            refused,
        })
    }

    fn append(&self, entry: impl Into<clm_ledger::Entry>) -> Result<clm_ledger::Appended, Error> {
        Ok(clm_ledger::append(&self.root, entry, self.env)?)
    }
}
