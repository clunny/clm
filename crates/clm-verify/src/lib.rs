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
use std::io::Write;
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
    /// Settings that can't be used are a reason, not a git failure.
    fn from(e: clm_config::Error) -> Self {
        Error::Refused(e.to_string())
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

/// Says `line` on stderr. A stderr that fails is no reason to end a run, since the check may be
/// running unwatched or its result not yet recorded, so the write's result is dropped
/// (`verify/verify-run`).
pub(crate) fn say(line: &str) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// The cap `land` applies when `[land]` sets none, which `config` doesn't offer without the
/// table.
const DEFAULT_CAP_GB: u64 = 15;

/// The repository a command runs in, with its `[verify]` settings.
pub(crate) struct Ctx<'a> {
    pub(crate) root: PathBuf,
    pub(crate) env: &'a Env,
    pub(crate) verify: clm_config::Verify,
    /// `[affected] all`.
    pub(crate) affected_all: Vec<String>,
    /// `[land] target_cap_gb`, the cap on the build place (`verify-build-cap`).
    pub(crate) cap_gb: u64,
    pub(crate) host: clm_config::Host,
    /// `origin/main`.
    pub(crate) base: String,
    /// Where the CLM repository is in its git repository: `rev-parse --show-prefix`.
    pub(crate) prefix: String,
}

impl<'a> Ctx<'a> {
    /// The context for `root`, or, in `Err(source)`, the name of the committed `clm.toml` whose
    /// settings hold no `[verify]`. `host` comes from the settings of `root`; every other
    /// setting from the `clm.toml` committed at `origin/main`, after fetching it
    /// (`settings-from-main`).
    fn open(root: &Path, env: &'a Env) -> Result<Result<Self, String>, Error> {
        let root = std::fs::canonicalize(root)?;
        let host = clm_config::load(Some(&root), env)?.host.value;
        clm_git::fetch(&root)?;
        let base = format!("{}/main", clm_git::REMOTE);
        let prefix = clm_git::git(&root, &["rev-parse", "--show-prefix"])?;
        let source = format!("{base}:{prefix}clm.toml");
        let text = match clm_git::git(&root, &["rev-parse", "--verify", "--quiet", &source]) {
            Ok(_) => Some(clm_git::git(&root, &["show", &source])?),
            Err(_) => None,
        };
        let settings = clm_config::load_committed(&root, text.as_deref(), &source, env)?;
        let Some(verify) = settings.verify.value else {
            return Ok(Err(source));
        };
        Ok(Ok(Ctx {
            affected_all: settings.affected.value.map(|a| a.all).unwrap_or_default(),
            cap_gb: settings.land.value.map_or(DEFAULT_CAP_GB, |l| l.cap_gb()),
            root,
            env,
            verify,
            host,
            base,
            prefix,
        }))
    }

    pub(crate) fn git(&self, args: &[&str]) -> Result<String, Error> {
        Ok(clm_git::git(&self.root, args)?)
    }

    /// `commit` as `git rev-parse --short` abbreviates it.
    pub(crate) fn short(&self, commit: &str) -> Result<String, Error> {
        self.git(&["rev-parse", "--short", commit])
    }
}

/// Runs one `clm verify` command in the repository at `root`. `env` is the environment
/// `config` reads.
pub fn verify(root: &Path, request: &Request, env: &Env) -> Result<Outcome, Error> {
    let ctx = match Ctx::open(root, env)? {
        Ok(ctx) => ctx,
        Err(source) => {
            return Ok(Outcome::new(
                format!("verify off: no [verify] in {source}"),
                0,
            ));
        }
    };
    let state = State::load(&ctx)?;
    match request {
        Request::Due { work_done, dry_run } => due::due(&ctx, &state, *work_done, *dry_run),
        Request::Commit(sha) => run::commit(&ctx, &state, sha),
        Request::Import => import::import_all(&ctx, &state),
        Request::Bisect(sha) => bisect::bisect_red(&ctx, &state, sha),
    }
}
