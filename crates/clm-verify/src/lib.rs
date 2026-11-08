//! `clm verify`: a project's slow tests run after the land, at the moments the project
//! chooses, and the result recorded in the ledger (specs/verify/SPEC.md,
//! `capability/slow-tests-after-land`). Every `git` call goes through `clm-git`, the ledger
//! through `clm-ledger`, the host's pipeline runs through `clm-host`, and the check through the
//! runner `clm-land` shares.

mod bisect;
mod chain;
mod due;
mod import;
mod run;
mod tree;

use std::fmt;
use std::path::{Path, PathBuf};

pub use clm_land::Env;

use chain::State;

/// What `clm verify` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// `--due [--event work-done] [--dry-run]`.
    Due { work_done: bool, dry_run: bool },
    /// `--commit <sha>`.
    Commit(String),
    /// `--import`.
    Import,
    /// `--bisect <sha>`.
    Bisect(String),
}

/// What a verification came to: the one `verify ...` line for stdout, notes for stderr, and
/// the exit code (0 passed, nothing due or nothing to import; 1 failed; 3 nothing due under
/// `--dry-run`; 4 pending).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub line: String,
    pub notes: Vec<String>,
    pub code: u8,
}

impl Outcome {
    pub(crate) fn new(line: impl Into<String>, code: u8) -> Self {
        Outcome {
            line: line.into(),
            notes: Vec::new(),
            code,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Nothing was verified, for the reason given: exit 1.
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

impl From<clm_git::Failure> for Error {
    fn from(e: clm_git::Failure) -> Self {
        match e {
            clm_git::Failure::Refused(why) => Error::Refused(why),
            clm_git::Failure::Git(why) => Error::Git(why),
        }
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

impl From<clm_land::Error> for Error {
    fn from(e: clm_land::Error) -> Self {
        match e {
            clm_land::Error::Refused(why) => Error::Refused(why),
            clm_land::Error::Git(why) => Error::Git(why),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Git(e.to_string())
    }
}

/// The repository a command runs in, with its `[verify]` settings.
pub(crate) struct Ctx<'a> {
    pub(crate) root: PathBuf,
    pub(crate) env: &'a Env,
    pub(crate) verify: clm_config::Verify,
    /// `[affected] all`.
    pub(crate) affected_all: Vec<String>,
    pub(crate) host: clm_config::Host,
    /// `origin/main`, or the local `main` without a remote.
    pub(crate) base: String,
    pub(crate) remote: bool,
    /// Where the CLM repository is in its git repository: `rev-parse --show-prefix`.
    pub(crate) prefix: String,
}

impl<'a> Ctx<'a> {
    /// The context for `root`, or `None` when its settings hold no `[verify]`.
    fn open(root: &Path, env: &'a Env) -> Result<Option<Self>, Error> {
        let root = std::fs::canonicalize(root)?;
        let settings = clm_config::load(Some(&root), env)?;
        let Some(verify) = settings.verify.value else {
            return Ok(None);
        };
        let host = settings.host.value;
        let remote = host != clm_config::Host::None;
        let base = match remote {
            true => format!("{}/main", clm_git::REMOTE),
            false => "main".to_string(),
        };
        let prefix = clm_git::git(&root, &["rev-parse", "--show-prefix"])?;
        Ok(Some(Ctx {
            affected_all: settings.affected.value.map(|a| a.all).unwrap_or_default(),
            root,
            env,
            verify,
            host,
            base,
            remote,
            prefix,
        }))
    }

    pub(crate) fn git(&self, args: &[&str]) -> Result<String, Error> {
        Ok(clm_git::git(&self.root, args)?)
    }

    /// `git fetch origin`, which brings `origin/main` and the ledger branches; nothing
    /// without a remote.
    fn fetch(&self) -> Result<(), Error> {
        if self.remote {
            clm_git::fetch(&self.root)?;
        }
        Ok(())
    }

    /// `commit` as `git rev-parse --short` abbreviates it.
    pub(crate) fn short(&self, commit: &str) -> Result<String, Error> {
        self.git(&["rev-parse", "--short", commit])
    }
}

/// Runs one `clm verify` command in the repository at `root`. `env` is the environment
/// `config` reads.
pub fn verify(root: &Path, request: &Request, env: &Env) -> Result<Outcome, Error> {
    let Some(ctx) = Ctx::open(root, env)? else {
        return Ok(Outcome::new("verify off: no [verify] in clm.toml", 0));
    };
    ctx.fetch()?;
    let state = State::load(&ctx)?;
    match request {
        Request::Due { work_done, dry_run } => due::due(&ctx, &state, *work_done, *dry_run),
        Request::Commit(sha) => run::commit(&ctx, &state, sha),
        Request::Import => import::import_all(&ctx, &state),
        Request::Bisect(sha) => bisect::bisect_red(&ctx, &state, sha),
    }
}
